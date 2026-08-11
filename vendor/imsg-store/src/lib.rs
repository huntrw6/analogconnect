//! Encrypted `SQLCipher` message store for imsg.
//!
//! Owns the database connection, schema, migrations, and all query operations.
//! Receives an already-resolved database path and an in-memory key — does not
//! resolve paths and does not talk to the keyring. Introduces no dependency on
//! the protocol crates.

mod cursors;
mod outbox;
mod query;
mod read;
mod row;
mod store;

pub use row::{
    Direction, FolderCursor, FolderSyncStatus, GroupAssignmentResult, MessageRow, NewMessage,
    OutboxRow, OutboxStatus, OutgoingStatus, ThreadRow, STATUS_READ, STATUS_UNREAD,
};
pub use store::Store;

use thiserror::Error;

/// Database open, async-dispatch, migration, or invalid outbox-transition failures.
#[derive(Debug, Error)]
pub enum Error {
    /// A rusqlite error from opening the database file (bad path, wrong key, etc.).
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// An async task or channel error from `tokio-rusqlite` (call dispatch, task panic, etc.).
    #[error("connection error: {0}")]
    Connection(#[from] tokio_rusqlite::Error),
    /// Schema migrations failed on open; the binary may be a downgrade.
    ///
    /// Inner string preserves the `Box<dyn Error>` message from refinery — the concrete
    /// migration error type is not stable across refinery versions.
    #[error("migration failed: {0}")]
    Migration(String),
    /// `resolve` was called with `OutboxStatus::Queued`, which is not a valid transition target.
    ///
    /// `resolve` advances outbox state; callers must supply `Sending`, `Sent`, `Failed`, or `Unknown`.
    #[error("invalid outbox transition: Queued is not a valid resolve target")]
    InvalidTransition,
}

#[cfg(test)]
mod tests {
    use secrecy::SecretBox;

    use super::{
        Direction, GroupAssignmentResult, NewMessage, Store, STATUS_READ, STATUS_UNREAD,
    };

    fn message(handle: &str, address: &str, conversation_key: &str, status: i32) -> NewMessage {
        NewMessage {
            map_handle: handle.to_owned(),
            timestamp_ms: if handle == "synthetic-a" { 1 } else { 2 },
            folder: "inbox".to_owned(),
            direction: Direction::Received,
            address: address.to_owned(),
            conversation_key: conversation_key.to_owned(),
            participants: "synthetic-a, synthetic-b, synthetic-local".to_owned(),
            status,
            synced_at: 3,
            text: "synthetic-body".to_owned(),
            outgoing_status: None,
        }
    }

    #[tokio::test]
    async fn participant_key_groups_different_senders_and_refreshes_existing_handles(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(
            directory.path().join("messages.db"),
            SecretBox::new(Box::new([9_u8; 32])),
        )
        .await?;

        store
            .upsert(message(
                "synthetic-a",
                "synthetic-a",
                "synthetic-legacy-key",
                STATUS_UNREAD,
            ))
            .await?;
        store
            .upsert(message(
                "synthetic-a",
                "synthetic-a",
                "synthetic-group-key",
                STATUS_UNREAD,
            ))
            .await?;
        store
            .upsert(message(
                "synthetic-b",
                "synthetic-b",
                "synthetic-group-key",
                STATUS_READ,
            ))
            .await?;

        let threads = store.threads().await?;
        assert_eq!(threads.len(), 1);
        let thread = threads
            .first()
            .ok_or_else(|| std::io::Error::other("synthetic thread missing"))?;
        assert_eq!(thread.total, 2);
        assert_eq!(thread.unread, 1);
        assert_eq!(thread.participant_count, 2);
        let messages = store
            .list_conversation_messages("synthetic-group-key", 50, 0)
            .await?;
        assert_eq!(messages.len(), 2);
        let first = messages
            .first()
            .ok_or_else(|| std::io::Error::other("synthetic first message missing"))?;
        let last = messages
            .last()
            .ok_or_else(|| std::io::Error::other("synthetic last message missing"))?;
        assert_ne!(first.address, last.address);
        Ok(())
    }

    #[tokio::test]
    async fn ancs_group_assignment_persists_title_groups_senders_and_fails_on_conflict(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("messages.db");
        let open = || Store::open(path.clone(), SecretBox::new(Box::new([8_u8; 32])));
        let store = open().await?;
        store
            .upsert(message("synthetic-a", "sender-a", "sender-a", STATUS_UNREAD))
            .await?;
        store
            .upsert(message("synthetic-b", "sender-b", "sender-b", STATUS_READ))
            .await?;
        store.mark_ancs_ambiguous("synthetic-a", 9).await?;
        assert!(store
            .threads()
            .await?
            .iter()
            .any(|thread| thread.identity_conflict));
        store.clear_ancs_ambiguous("synthetic-a").await?;
        let group_id = format!("ancs-v1-{}", "a".repeat(64));
        for (handle, sender, uid) in [
            ("synthetic-a", "sender-a", [1, 0, 0, 0]),
            ("synthetic-b", "sender-b", [2, 0, 0, 0]),
        ] {
            assert_eq!(
                store
                    .assign_ancs_group(handle, &group_id, "Synthetic Group", sender, uid, 10)
                    .await?,
                GroupAssignmentResult::Assigned
            );
        }
        drop(store);

        let reopened = open().await?;
        let threads = reopened.threads().await?;
        assert_eq!(threads.len(), 1);
        let thread = threads
            .first()
            .ok_or_else(|| std::io::Error::other("synthetic ANCS thread missing"))?;
        assert_eq!(thread.conversation_key, group_id);
        assert_eq!(thread.group_title.as_deref(), Some("Synthetic Group"));
        assert!(thread.is_ancs_group);
        assert!(!thread.identity_conflict);
        assert_eq!(thread.participant_count, 2);

        let other_group = format!("ancs-v1-{}", "b".repeat(64));
        assert_eq!(
            reopened
                .assign_ancs_group(
                    "synthetic-a",
                    &other_group,
                    "Other Synthetic Group",
                    "sender-a",
                    [3, 0, 0, 0],
                    20,
                )
                .await?,
            GroupAssignmentResult::IdentityConflict
        );
        let messages = reopened
            .list_conversation_messages(&group_id, 10, 0)
            .await?;
        assert_eq!(messages.len(), 2);
        Ok(())
    }
}
