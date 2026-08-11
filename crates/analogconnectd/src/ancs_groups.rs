use std::collections::VecDeque;

use sha2::{Digest as _, Sha256};
use unicode_casefold::UnicodeCaseFold as _;
use unicode_normalization::UnicodeNormalization as _;

const HASH_DOMAIN: &[u8] = b"ancs-subtitle-v1\0";

/// Applies the version-one ANCS Subtitle identity normalization contract.
#[must_use]
pub fn normalize_ancs_subtitle_v1(value: &str) -> String {
    value
        .nfkc()
        .case_fold()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns a persistent full-SHA256 group ID, or `None` for an empty direct Subtitle.
#[must_use]
pub fn ancs_group_id(value: &str) -> Option<String> {
    let normalized = normalize_ancs_subtitle_v1(value);
    if normalized.is_empty() {
        return None;
    }
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(normalized.as_bytes());
    Some(format!("ancs-v1-{}", encode_hex(&digest.finalize())))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingAncsNotification {
    pub notification_uid: [u8; 4],
    pub title: String,
    pub subtitle: String,
    pub observed_unix_millis: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingMapMessage {
    pub map_handle: String,
    pub sender: String,
    pub sender_display_name: Option<String>,
    pub observed_unix_millis: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorrelationResult {
    Group {
        map_handle: String,
        notification_uid: [u8; 4],
        group_id: String,
        display_subtitle: String,
        sender: String,
        observed_unix_millis: i64,
    },
    DirectCandidate {
        map_handle: String,
        notification_uid: [u8; 4],
    },
    Ambiguous,
}

/// Bounded in-memory ANCS/MAP correlation window that fails closed.
pub struct AncsMapCorrelator {
    notifications: VecDeque<PendingAncsNotification>,
    messages: VecDeque<PendingMapMessage>,
    completed_uids: VecDeque<[u8; 4]>,
    max_delta_millis: u64,
}

impl AncsMapCorrelator {
    #[must_use]
    pub const fn new(max_delta_millis: u64) -> Self {
        Self {
            notifications: VecDeque::new(),
            messages: VecDeque::new(),
            completed_uids: VecDeque::new(),
            max_delta_millis,
        }
    }

    /// Adds or updates an ANCS notification and resolves a previously observed MAP message.
    pub fn observe_ancs(
        &mut self,
        notification: PendingAncsNotification,
    ) -> Option<CorrelationResult> {
        if self.completed_uids.contains(&notification.notification_uid) {
            return None;
        }
        if let Some(existing) = self
            .notifications
            .iter_mut()
            .find(|existing| existing.notification_uid == notification.notification_uid)
        {
            *existing = notification;
            return None;
        }
        let candidates = self
            .messages
            .iter()
            .enumerate()
            .filter(|(_, message)| self.matches(&notification, message))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if let [index] = candidates.as_slice() {
            let message = self.messages.remove(*index)?;
            self.remember_completed(notification.notification_uid);
            return Some(correlate_pair(notification, message));
        }
        self.notifications.push_back(notification);
        while self.notifications.len() > 32 {
            let _ = self.notifications.pop_front();
        }
        (candidates.len() > 1).then_some(CorrelationResult::Ambiguous)
    }

    #[must_use]
    pub fn correlate_map(&mut self, message: PendingMapMessage) -> CorrelationResult {
        let candidates = self
            .notifications
            .iter()
            .enumerate()
            .filter(|(_, notification)| self.matches(notification, &message))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [index] = candidates.as_slice() else {
            if candidates.is_empty()
                && !self
                    .messages
                    .iter()
                    .any(|existing| existing.map_handle == message.map_handle)
            {
                self.messages.push_back(message);
                while self.messages.len() > 32 {
                    let _ = self.messages.pop_front();
                }
            }
            return CorrelationResult::Ambiguous;
        };
        let Some(notification) = self.notifications.remove(*index) else {
            return CorrelationResult::Ambiguous;
        };
        self.remember_completed(notification.notification_uid);
        correlate_pair(notification, message)
    }

    fn matches(&self, notification: &PendingAncsNotification, message: &PendingMapMessage) -> bool {
        let sender_label = message
            .sender_display_name
            .as_deref()
            .unwrap_or(message.sender.as_str());
        notification
            .observed_unix_millis
            .abs_diff(message.observed_unix_millis)
            <= self.max_delta_millis
            && normalize_ancs_subtitle_v1(&notification.title)
                == normalize_ancs_subtitle_v1(sender_label)
    }

    fn remember_completed(&mut self, uid: [u8; 4]) {
        self.completed_uids.push_back(uid);
        while self.completed_uids.len() > 64 {
            let _ = self.completed_uids.pop_front();
        }
    }
}

fn correlate_pair(
    notification: PendingAncsNotification,
    message: PendingMapMessage,
) -> CorrelationResult {
    let Some(group_id) = ancs_group_id(&notification.subtitle) else {
        return CorrelationResult::DirectCandidate {
            map_handle: message.map_handle,
            notification_uid: notification.notification_uid,
        };
    };
    CorrelationResult::Group {
        map_handle: message.map_handle,
        notification_uid: notification.notification_uid,
        group_id,
        display_subtitle: notification.subtitle,
        sender: message.sender,
        observed_unix_millis: notification.observed_unix_millis,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyCorrelationResult {
    GroupAssigned,
    DirectCandidate,
    Ambiguous,
    IdentityConflict,
}

/// Applies a proven correlation to the encrypted message store.
///
/// This is the production ingest boundary for a future supervised ANCS transport.
pub async fn apply_correlation(
    store: &imsg_store::Store,
    result: CorrelationResult,
) -> Result<ApplyCorrelationResult, imsg_store::Error> {
    let (map_handle, notification_uid, group_id, display_subtitle, sender, observed_unix_millis) =
        match result {
            CorrelationResult::Group {
                map_handle,
                notification_uid,
                group_id,
                display_subtitle,
                sender,
                observed_unix_millis,
            } => (
                map_handle,
                notification_uid,
                group_id,
                display_subtitle,
                sender,
                observed_unix_millis,
            ),
            CorrelationResult::DirectCandidate { .. } => {
                return Ok(ApplyCorrelationResult::DirectCandidate);
            }
            CorrelationResult::Ambiguous => return Ok(ApplyCorrelationResult::Ambiguous),
        };
    match store
        .assign_ancs_group(
            &map_handle,
            &group_id,
            &display_subtitle,
            &sender,
            notification_uid,
            observed_unix_millis,
        )
        .await?
    {
        imsg_store::GroupAssignmentResult::Assigned => Ok(ApplyCorrelationResult::GroupAssigned),
        imsg_store::GroupAssignmentResult::IdentityConflict => {
            Ok(ApplyCorrelationResult::IdentityConflict)
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretBox;

    use super::*;

    #[test]
    fn normalization_and_hash_use_full_versioned_sha256() {
        let id = ancs_group_id("  STRAẞE\u{00a0} Group  ").unwrap();
        assert_eq!(
            normalize_ancs_subtitle_v1("  STRAẞE\u{00a0} Group  "),
            "strasse group"
        );
        assert_eq!(id.len(), 72);
        assert!(id.starts_with("ancs-v1-"));
        assert_eq!(id, ancs_group_id("strasse group").unwrap());
        assert_ne!(id, ancs_group_id("different group").unwrap());
        assert_eq!(ancs_group_id(" \n\t "), None);
    }

    #[test]
    fn different_senders_same_subtitle_resolve_to_same_group() {
        let expected = ancs_group_id("Named Group").unwrap();
        for (title, sender) in [("Person A", "sender-a"), ("Person B", "sender-b")] {
            let mut resolver = AncsMapCorrelator::new(5_000);
            let _ = resolver.observe_ancs(PendingAncsNotification {
                notification_uid: [1, 0, 0, 0],
                title: title.to_owned(),
                subtitle: "Named Group".to_owned(),
                observed_unix_millis: 10_000,
            });
            let CorrelationResult::Group { group_id, .. } =
                resolver.correlate_map(PendingMapMessage {
                    map_handle: "handle".to_owned(),
                    sender: sender.to_owned(),
                    sender_display_name: Some(title.to_owned()),
                    observed_unix_millis: 10_100,
                })
            else {
                panic!("expected group correlation");
            };
            assert_eq!(group_id, expected);
        }
    }

    #[test]
    fn direct_is_explicit_and_missing_or_competing_correlation_is_ambiguous() {
        let mut direct = AncsMapCorrelator::new(5_000);
        let _ = direct.observe_ancs(PendingAncsNotification {
            notification_uid: [1, 2, 3, 4],
            title: "Person".to_owned(),
            subtitle: String::new(),
            observed_unix_millis: 10,
        });
        assert!(matches!(
            direct.correlate_map(PendingMapMessage {
                map_handle: "handle".to_owned(),
                sender: "address".to_owned(),
                sender_display_name: Some("Person".to_owned()),
                observed_unix_millis: 11,
            }),
            CorrelationResult::DirectCandidate { .. }
        ));
        let mut missing = AncsMapCorrelator::new(5_000);
        assert_eq!(
            missing.correlate_map(PendingMapMessage {
                map_handle: "handle".to_owned(),
                sender: "address".to_owned(),
                sender_display_name: None,
                observed_unix_millis: 11,
            }),
            CorrelationResult::Ambiguous
        );
    }

    #[test]
    fn duplicate_updates_do_not_compete_and_same_sender_bursts_do() {
        let notification = PendingAncsNotification {
            notification_uid: [1, 2, 3, 4],
            title: "Person".to_owned(),
            subtitle: "Group".to_owned(),
            observed_unix_millis: 100,
        };
        let mut duplicate = AncsMapCorrelator::new(5_000);
        let _ = duplicate.observe_ancs(notification.clone());
        let _ = duplicate.observe_ancs(notification);
        assert!(matches!(
            duplicate.correlate_map(PendingMapMessage {
                map_handle: "handle".to_owned(),
                sender: "sender".to_owned(),
                sender_display_name: Some("Person".to_owned()),
                observed_unix_millis: 101,
            }),
            CorrelationResult::Group { .. }
        ));

        let mut burst = AncsMapCorrelator::new(5_000);
        for uid in [[1, 0, 0, 0], [2, 0, 0, 0]] {
            let _ = burst.observe_ancs(PendingAncsNotification {
                notification_uid: uid,
                title: "Person".to_owned(),
                subtitle: "Group".to_owned(),
                observed_unix_millis: 100,
            });
        }
        assert_eq!(
            burst.correlate_map(PendingMapMessage {
                map_handle: "handle".to_owned(),
                sender: "sender".to_owned(),
                sender_display_name: Some("Person".to_owned()),
                observed_unix_millis: 101,
            }),
            CorrelationResult::Ambiguous
        );
    }

    #[test]
    fn out_of_order_arrival_resolves_but_stale_notification_does_not() {
        let mut resolver = AncsMapCorrelator::new(1_000);
        assert_eq!(
            resolver.correlate_map(PendingMapMessage {
                map_handle: "out-of-order".to_owned(),
                sender: "sender".to_owned(),
                sender_display_name: Some("Person".to_owned()),
                observed_unix_millis: 10_000,
            }),
            CorrelationResult::Ambiguous
        );
        assert!(matches!(
            resolver.observe_ancs(PendingAncsNotification {
                notification_uid: [1, 0, 0, 0],
                title: "Person".to_owned(),
                subtitle: "Group".to_owned(),
                observed_unix_millis: 10_100,
            }),
            Some(CorrelationResult::Group { .. })
        ));

        let mut stale = AncsMapCorrelator::new(1_000);
        let _ = stale.observe_ancs(PendingAncsNotification {
            notification_uid: [2, 0, 0, 0],
            title: "Person".to_owned(),
            subtitle: "Group".to_owned(),
            observed_unix_millis: 1,
        });
        assert_eq!(
            stale.correlate_map(PendingMapMessage {
                map_handle: "stale".to_owned(),
                sender: "sender".to_owned(),
                sender_display_name: Some("Person".to_owned()),
                observed_unix_millis: 5_000,
            }),
            CorrelationResult::Ambiguous
        );
    }

    #[tokio::test]
    async fn correlation_boundary_assigns_group_but_not_ambiguous_or_direct()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let store = imsg_store::Store::open(
            directory.path().join("messages.db"),
            SecretBox::new(Box::new([3_u8; 32])),
        )
        .await?;
        store
            .upsert(imsg_store::NewMessage {
                map_handle: "synthetic-handle".to_owned(),
                timestamp_ms: 100,
                folder: "inbox".to_owned(),
                direction: imsg_store::Direction::Received,
                address: "synthetic-sender".to_owned(),
                conversation_key: "synthetic-sender".to_owned(),
                participants: "synthetic-sender".to_owned(),
                status: imsg_store::STATUS_UNREAD,
                synced_at: 101,
                text: "synthetic-body".to_owned(),
                outgoing_status: None,
            })
            .await?;
        let group_id = ancs_group_id("Synthetic Group").unwrap();
        assert_eq!(
            apply_correlation(
                &store,
                CorrelationResult::Group {
                    map_handle: "synthetic-handle".to_owned(),
                    notification_uid: [1, 2, 3, 4],
                    group_id: group_id.clone(),
                    display_subtitle: "Synthetic Group".to_owned(),
                    sender: "synthetic-sender".to_owned(),
                    observed_unix_millis: 102,
                },
            )
            .await?,
            ApplyCorrelationResult::GroupAssigned
        );
        let threads = store.threads().await?;
        let thread = threads
            .first()
            .ok_or_else(|| std::io::Error::other("synthetic ANCS thread missing"))?;
        assert_eq!(thread.conversation_key, group_id);
        assert_eq!(
            apply_correlation(&store, CorrelationResult::Ambiguous).await?,
            ApplyCorrelationResult::Ambiguous
        );
        assert_eq!(
            apply_correlation(
                &store,
                CorrelationResult::DirectCandidate {
                    map_handle: "synthetic-handle".to_owned(),
                    notification_uid: [4, 3, 2, 1],
                },
            )
            .await?,
            ApplyCorrelationResult::DirectCandidate
        );
        Ok(())
    }
}
