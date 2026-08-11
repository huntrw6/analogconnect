//! Privacy-safe structural probe for raw MAP bMessages.
//!
//! The raw response is held only in memory. Output contains cardinalities and booleans, never
//! handles, addresses, names, timestamps, or message text.

use map_core::{folders::Folder, messages::ListMessagesFilter};
use std::time::Duration;

#[derive(Default)]
struct CardCounts {
    cards: usize,
    tel_fields: usize,
    email_fields: usize,
    named_cards: usize,
    other_fields: usize,
}

#[derive(Default)]
struct RawShape {
    originator: CardCounts,
    recipients: CardCounts,
}

fn field_name(line: &str) -> &str {
    line.split_once(':')
        .map_or(line, |(left, _)| left)
        .split_once(';')
        .map_or_else(
            || line.split_once(':').map_or(line, |(left, _)| left),
            |(name, _)| name,
        )
}

fn inspect_raw(raw: &[u8]) -> RawShape {
    let text = String::from_utf8_lossy(raw);
    let mut shape = RawShape::default();
    let mut envelope_depth = 0usize;
    let mut in_card = false;
    let mut card_is_recipient = false;

    for line in text.lines().map(str::trim_end) {
        match line {
            "BEGIN:BENV" => envelope_depth = envelope_depth.saturating_add(1),
            "END:BENV" => envelope_depth = envelope_depth.saturating_sub(1),
            "BEGIN:VCARD" => {
                in_card = true;
                card_is_recipient = envelope_depth > 0;
                let counts = if card_is_recipient {
                    &mut shape.recipients
                } else {
                    &mut shape.originator
                };
                counts.cards += 1;
            }
            "END:VCARD" => in_card = false,
            _ if in_card => {
                let counts = if card_is_recipient {
                    &mut shape.recipients
                } else {
                    &mut shape.originator
                };
                match field_name(line) {
                    "TEL" => counts.tel_fields += 1,
                    "EMAIL" => counts.email_fields += 1,
                    "FN" | "N" => counts.named_cards += 1,
                    "BEGIN" | "END" | "VERSION" | "" => {}
                    _ => counts.other_fields += 1,
                }
            }
            _ => {}
        }
    }
    shape
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load(None)?;
    let address = cfg.device.address().parse::<bluer::Address>()?;
    let stream =
        transport::rfcomm::connect(address, cfg.device.map_channel, Duration::from_secs(5)).await?;
    let mut client = map_core::client::MapClient::connect(stream).await?;
    client.set_folder(Folder::Inbox).await?;
    let entries = client
        .list_messages(&ListMessagesFilter {
            max_count: 8,
            ..Default::default()
        })
        .await?;

    println!("RAW_GROUP_PROBE=PASS messages={}", entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let raw = client.get_message_raw(&entry.handle).await?;
        let shape = inspect_raw(&raw);
        println!(
            "message={} direction={} conversation_id={} originator_cards={} originator_tel={} originator_email={} recipient_cards={} recipient_tel={} recipient_email={} recipient_named_fields={} recipient_other_fields={}",
            index + 1,
            if entry.sent { "sent" } else { "received" },
            if entry.conversation_id.trim().is_empty() { "absent" } else { "present" },
            shape.originator.cards,
            shape.originator.tel_fields,
            shape.originator.email_fields,
            shape.recipients.cards,
            shape.recipients.tel_fields,
            shape.recipients.email_fields,
            shape.recipients.named_cards,
            shape.recipients.other_fields,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::inspect_raw;

    #[test]
    fn separates_originator_and_recipient_cards_without_retaining_values() {
        let raw = b"BEGIN:BMSG\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nTEL:private-a\r\nEND:VCARD\r\nBEGIN:BENV\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nTEL:private-b\r\nEMAIL:private@example.invalid\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nTEL:private-c\r\nEND:VCARD\r\nEND:BENV\r\nEND:BMSG\r\n";
        let shape = inspect_raw(raw);
        assert_eq!(shape.originator.cards, 1);
        assert_eq!(shape.originator.tel_fields, 1);
        assert_eq!(shape.recipients.cards, 2);
        assert_eq!(shape.recipients.tel_fields, 2);
        assert_eq!(shape.recipients.email_fields, 1);
    }
}
