//! Shared device read, normalized to a store-agnostic [`FetchedMessage`].
//!
//! Lists a MAP folder and fetches each message body. Consumed by both the persist tail
//! (`sync::backfill_folder`) and the live read path (`live`), so the `list_messages` +
//! per-entry `get_message` loop exists once. No store access — the cursor that derives
//! `since_ms` is read by the caller.

use map_core::client::MapClient;
use map_core::folders::Folder;
use map_core::{
    messages::{ListMessagesFilter, MessageEntry},
    BMessage,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::util::datetime_to_ms;

/// A message read from the device and normalized, before it is persisted or rendered.
///
/// `address` is already resolved to the peer (recipient for sent, sender for received). `sent`
/// is retained so the persist tail can derive the stored `direction`. Carries no store identity
/// (`rowid`/`synced_at`) — those have no meaning until/unless the row is written.
pub struct FetchedMessage {
    /// Opaque device-assigned MAP handle.
    pub handle: String,
    /// Milliseconds since Unix epoch; falls back to the caller's `now` when the entry datetime
    /// is absent or malformed.
    pub timestamp_ms: i64,
    /// MAP folder leaf the message was listed in.
    pub folder: String,
    /// `true` for outbound messages from this device; `false` for received.
    pub sent: bool,
    /// Peer phone number/address: recipient for sent messages, sender for received.
    pub address: String,
    /// Canonical private participant-set key used to preserve group conversations.
    pub conversation_key: String,
    /// Human-readable canonical participant list for authenticated clients.
    pub participants: String,
    /// Device-reported read state.
    pub read: bool,
    /// Decoded message body text.
    pub text: String,
}

impl FetchedMessage {
    fn from_entry(
        entry: &MessageEntry,
        folder: &str,
        text: String,
        participant_addresses: Vec<String>,
        now: i64,
    ) -> Self {
        let address = if entry.sent {
            entry.recipient_addressing.clone()
        } else {
            entry.sender_addressing.clone()
        };
        let (conversation_key, participants) =
            canonicalize_participants(participant_addresses, &address);
        let conversation_key = if entry.conversation_id.trim().is_empty() {
            conversation_key
        } else {
            format!("map:{}", entry.conversation_id.trim())
        };
        Self {
            handle: entry.handle.clone(),
            timestamp_ms: datetime_to_ms(&entry.datetime).unwrap_or(now),
            folder: folder.to_owned(),
            sent: entry.sent,
            address,
            conversation_key,
            participants,
            read: entry.read,
            text,
        }
    }
}

const PARTICIPANT_KEY_SEPARATOR: &str = "\u{1f}";

fn bmessage_participants(entry: &MessageEntry, bmsg: &BMessage) -> Vec<String> {
    let mut addresses = vec![
        entry.sender_addressing.clone(),
        entry.recipient_addressing.clone(),
    ];
    if let Some(originator) = bmsg.originator() {
        addresses.push(originator.tel.clone());
    }
    addresses.extend(
        bmsg.envelope()
            .recipients
            .iter()
            .map(|recipient| recipient.tel.clone()),
    );
    addresses
}

/// Sorts, deduplicates, and renders a private MAP participant address set.
pub(crate) fn canonicalize_participants(
    mut addresses: Vec<String>,
    fallback: &str,
) -> (String, String) {
    for address in &mut addresses {
        *address = address.trim().to_owned();
    }
    addresses.retain(|address| !address.is_empty());
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() && !fallback.trim().is_empty() {
        addresses.push(fallback.trim().to_owned());
    }
    let display = addresses.join(", ");
    let key = if addresses.len() <= 2 && !fallback.trim().is_empty() {
        fallback.trim().to_owned()
    } else {
        addresses.join(PARTICIPANT_KEY_SEPARATOR)
    };
    (key, display)
}

/// Lists `folder`, paging at 1024 entries per request and accumulating across pages.
///
/// Listing only — no bodies are fetched, so this is cheap relative to [`fetch_folder`] and is
/// what the live `threads` aggregation uses. Pure device read: no store access. Returns entries
/// in device listing order.
///
/// # Errors
///
/// Returns an error if any MAP `set_folder`/`list_messages` operation fails.
pub async fn list_folder<T: AsyncRead + AsyncWrite + Unpin>(
    client: &mut MapClient<T>,
    folder: Folder,
) -> anyhow::Result<Vec<MessageEntry>> {
    const PAGE: u16 = 1024;

    client.set_folder(folder).await?;
    let mut out = Vec::new();
    let mut offset: u16 = 0;
    loop {
        let filter = ListMessagesFilter {
            max_count: PAGE,
            offset,
            ..Default::default()
        };
        let entries = client.list_messages(&filter).await?;
        let count = entries.len();
        out.extend(entries);
        if count < usize::from(PAGE) {
            break;
        }
        match offset.checked_add(PAGE) {
            Some(next) => offset = next,
            None => break,
        }
    }
    Ok(out)
}

/// Lists `folder` and fetches each message body since `since_ms`.
///
/// Entries older than `since_ms` (by listing datetime) are skipped before the body fetch, so a
/// caller passing the per-folder cursor anchor fetches only new messages; `None` fetches the full
/// window. `now` is the fallback timestamp for entries with an absent/malformed datetime. Pure
/// device read: no store access, no cursor writes.
///
/// # Errors
///
/// Returns an error if any MAP `set_folder`/`list_messages`/`get_message` operation fails.
pub async fn fetch_folder<T: AsyncRead + AsyncWrite + Unpin>(
    client: &mut MapClient<T>,
    folder: Folder,
    since_ms: Option<i64>,
    now: i64,
) -> anyhow::Result<Vec<FetchedMessage>> {
    let folder_str = folder.as_str();
    let mut out = Vec::new();
    for entry in list_folder(client, folder).await? {
        if since_ms.is_some_and(|since| datetime_to_ms(&entry.datetime).unwrap_or(i64::MAX) < since)
        {
            continue;
        }
        let bmsg = client.get_message(&entry.handle).await?;
        let text = bmsg.envelope().body.text.clone();
        let participant_addresses = bmessage_participants(&entry, &bmsg);
        out.push(FetchedMessage::from_entry(
            &entry,
            folder_str,
            text,
            participant_addresses,
            now,
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(sent: bool) -> MessageEntry {
        MessageEntry {
            attribute_names: Vec::new(),
            handle: "0400".to_owned(),
            conversation_id: String::new(),
            conversation_name: String::new(),
            direction: String::new(),
            subject: "hi".to_owned(),
            datetime: "20260101T120000".to_owned(),
            sender_name: "Alice".to_owned(),
            sender_addressing: "+15550001".to_owned(),
            recipient_name: "Bob".to_owned(),
            recipient_addressing: "+15550002".to_owned(),
            msg_type: "SMS_GSM".to_owned(),
            size: 2,
            read: true,
            sent,
        }
    }

    #[test]
    fn received_resolves_address_to_sender() {
        let m = FetchedMessage::from_entry(
            &entry(false),
            "inbox",
            "body".to_owned(),
            vec!["+15550001".to_owned(), "+15550002".to_owned()],
            0,
        );
        assert!(!m.sent);
        assert_eq!(m.address, "+15550001");
        assert_eq!(m.conversation_key, "+15550001");
        assert_eq!(m.folder, "inbox");
        assert!(m.read);
    }

    #[test]
    fn sent_resolves_address_to_recipient() {
        let m = FetchedMessage::from_entry(
            &entry(true),
            "sent",
            "body".to_owned(),
            vec!["+15550001".to_owned(), "+15550002".to_owned()],
            0,
        );
        assert!(m.sent);
        assert_eq!(m.address, "+15550002");
        assert_eq!(m.conversation_key, "+15550002");
    }

    #[test]
    fn malformed_datetime_falls_back_to_now() {
        let mut e = entry(false);
        e.datetime = "not-a-date".to_owned();
        let m = FetchedMessage::from_entry(
            &e,
            "inbox",
            "body".to_owned(),
            vec!["+15550001".to_owned(), "+15550002".to_owned()],
            42,
        );
        assert_eq!(m.timestamp_ms, 42);
    }

    #[test]
    fn participant_sets_are_sorted_deduplicated_and_group_stable() {
        let (first_key, first_display) = canonicalize_participants(
            vec![
                "+15550003".to_owned(),
                "+15550001".to_owned(),
                "+15550002".to_owned(),
                "+15550003".to_owned(),
            ],
            "",
        );
        let (second_key, second_display) = canonicalize_participants(
            vec![
                "+15550002".to_owned(),
                "+15550003".to_owned(),
                "+15550001".to_owned(),
            ],
            "",
        );
        assert_eq!(first_key, second_key);
        assert_eq!(first_display, second_display);
        assert_eq!(first_display.split(", ").count(), 3);
    }

    #[test]
    fn direct_conversations_keep_the_peer_key_used_by_outgoing_rows() {
        let (key, display) = canonicalize_participants(
            vec!["synthetic-local".to_owned(), "synthetic-peer".to_owned()],
            "synthetic-peer",
        );
        assert_eq!(key, "synthetic-peer");
        assert_eq!(display.split(", ").count(), 2);
    }

    #[test]
    fn map_conversation_id_overrides_per_message_peer_grouping() {
        let mut first = entry(false);
        first.conversation_id = "synthetic-map-conversation".to_owned();
        let message = FetchedMessage::from_entry(
            &first,
            "inbox",
            "synthetic-body".to_owned(),
            vec!["synthetic-local".to_owned(), "synthetic-peer".to_owned()],
            0,
        );
        assert_eq!(message.conversation_key, "map:synthetic-map-conversation");
    }
}
