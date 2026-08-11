use rusqlite::{OptionalExtension as _, params};

use crate::{
    row::{GroupAssignmentResult, MessageRow},
    Error, NewMessage, OutgoingStatus, Store,
};

impl Store {
    /// Marks a received MAP message as unsafe for private reply after competing ANCS evidence.
    pub async fn mark_ancs_ambiguous(
        &self,
        map_handle: &str,
        observed_at: i64,
    ) -> Result<(), Error> {
        let map_handle = map_handle.to_owned();
        self.conn()
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO ancs_ambiguous_messages (map_handle, observed_at) \
                     VALUES (?1, ?2) ON CONFLICT(map_handle) DO UPDATE SET \
                     observed_at = MAX(observed_at, excluded.observed_at)",
                    params![map_handle, observed_at],
                )?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Clears a prior fail-closed marker after unique ANCS evidence resolves the message.
    pub async fn clear_ancs_ambiguous(&self, map_handle: &str) -> Result<(), Error> {
        let map_handle = map_handle.to_owned();
        self.conn()
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM ancs_ambiguous_messages WHERE map_handle = ?1",
                    [map_handle],
                )?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Assigns an existing MAP message to a durable ANCS Subtitle group.
    ///
    /// The group metadata, observed sender, notification UID, and message conversation key are
    /// updated atomically. If the message was already assigned to another group, both identities
    /// are marked conflicted and the existing message assignment is preserved.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or transaction fails.
    pub async fn assign_ancs_group(
        &self,
        map_handle: &str,
        group_id: &str,
        display_subtitle: &str,
        sender: &str,
        notification_uid: [u8; 4],
        seen_at: i64,
    ) -> Result<GroupAssignmentResult, Error> {
        let map_handle = map_handle.to_owned();
        let group_id = group_id.to_owned();
        let display_subtitle = display_subtitle.to_owned();
        let sender = sender.to_owned();
        self.conn()
            .call(move |conn| {
                let tx = conn.transaction()?;
                tx.execute(
                    "INSERT INTO ancs_group_conversations \
                     (group_id, display_subtitle, identity_source, first_seen, last_seen, \
                      conversation_kind, identity_conflict) \
                     VALUES (?1, ?2, 'ANCS_SUBTITLE_V1', ?3, ?3, 'GROUP', 0) \
                     ON CONFLICT(group_id) DO UPDATE SET \
                       display_subtitle = excluded.display_subtitle, \
                       last_seen = MAX(last_seen, excluded.last_seen)",
                    params![group_id, display_subtitle, seen_at],
                )?;

                let existing: Option<String> = tx
                    .query_row(
                        "SELECT group_id FROM ancs_group_message_assignments \
                         WHERE map_handle = ?1",
                        [map_handle.as_str()],
                        |row| row.get(0),
                    )
                    .optional()?;
                if let Some(existing_group) = existing {
                    if existing_group != group_id {
                        tx.execute(
                            "UPDATE ancs_group_conversations SET identity_conflict = 1 \
                             WHERE group_id IN (?1, ?2)",
                            params![existing_group, group_id],
                        )?;
                        tx.commit()?;
                        return Ok(GroupAssignmentResult::IdentityConflict);
                    }
                }

                tx.execute(
                    "INSERT INTO ancs_group_senders \
                     (group_id, sender, first_seen, last_seen) VALUES (?1, ?2, ?3, ?3) \
                     ON CONFLICT(group_id, sender) DO UPDATE SET \
                       last_seen = MAX(last_seen, excluded.last_seen)",
                    params![group_id, sender, seen_at],
                )?;
                tx.execute(
                    "INSERT INTO ancs_group_message_assignments \
                     (map_handle, group_id, notification_uid, correlated_at) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(map_handle) DO UPDATE SET \
                       notification_uid = excluded.notification_uid, \
                       correlated_at = excluded.correlated_at",
                    params![map_handle, group_id, notification_uid.to_vec(), seen_at],
                )?;
                tx.execute(
                    "UPDATE messages SET conversation_key = ?1 WHERE map_handle = ?2",
                    params![group_id, map_handle],
                )?;
                tx.execute(
                    "DELETE FROM ancs_ambiguous_messages WHERE map_handle = ?1",
                    [map_handle],
                )?;
                tx.commit()?;
                Ok(GroupAssignmentResult::Assigned)
            })
            .await
            .map_err(Error::Connection)
    }

    /// Inserts a message or refreshes conversation metadata when `map_handle` already exists.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` write fails.
    pub async fn upsert(&self, msg: NewMessage) -> Result<(), Error> {
        self.conn()
            .call(
                move |conn: &mut rusqlite::Connection| -> Result<(), rusqlite::Error> {
                    let mut stmt = conn.prepare_cached(
                        "INSERT INTO messages \
                     (map_handle, timestamp_ms, folder, direction, address, conversation_key, \
                      participants, status, synced_at, text, outgoing_status) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                     ON CONFLICT(map_handle) DO UPDATE SET \
                       conversation_key = excluded.conversation_key, \
                       participants = excluded.participants",
                    )?;
                    stmt.execute(params![
                        msg.map_handle,
                        msg.timestamp_ms,
                        msg.folder,
                        msg.direction,
                        msg.address,
                        msg.conversation_key,
                        msg.participants,
                        msg.status,
                        msg.synced_at,
                        msg.text,
                        msg.outgoing_status,
                    ])?;
                    Ok(())
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Atomically inserts a speculative `messages` row and a linked `outbox` entry.
    ///
    /// Both inserts are wrapped in a single `SQLite` transaction; either both succeed or
    /// neither is written. The `messages` row is created with a placeholder `map_handle`
    /// of the form `"local:{outbox_id}"` which is later overwritten by [`Store::promote_outgoing`]
    /// once the device assigns a real handle.
    ///
    /// Returns `(message_rowid, outbox_id)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the transaction fails at any step.
    pub async fn enqueue_send(
        &self,
        msg: NewMessage,
        command: &str,
        payload: &str,
        created_at: i64,
    ) -> Result<(i64, i64), Error> {
        let command = command.to_owned();
        let payload = payload.to_owned();
        self.conn()
            .call(move |conn| {
                use crate::row::OutboxStatus;
                let tx = conn.transaction()?;

                // Insert outbox entry first so its rowid can seed the placeholder handle.
                tx.execute(
                    "INSERT INTO outbox \
                     (command, payload, local_message_id, status, created_at) \
                     VALUES (?1, ?2, NULL, ?3, ?4)",
                    params![command, payload, OutboxStatus::Queued, created_at],
                )?;
                let outbox_id = tx.last_insert_rowid();

                // Speculative message handle: unique, identifiable, replaced on push success.
                let placeholder = format!("local:{outbox_id}");
                tx.execute(
                    "INSERT INTO messages \
                     (map_handle, timestamp_ms, folder, direction, address, conversation_key, \
                      participants, status, synced_at, text, outgoing_status) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        placeholder,
                        msg.timestamp_ms,
                        msg.folder,
                        msg.direction,
                        msg.address,
                        msg.conversation_key,
                        msg.participants,
                        msg.status,
                        msg.synced_at,
                        msg.text,
                        msg.outgoing_status,
                    ],
                )?;
                let msg_rowid = tx.last_insert_rowid();

                // Link the outbox entry to its speculative message row.
                tx.execute(
                    "UPDATE outbox SET local_message_id = ?1 WHERE id = ?2",
                    params![msg_rowid, outbox_id],
                )?;

                tx.commit()?;
                Ok((msg_rowid, outbox_id))
            })
            .await
            .map_err(Error::Connection)
    }

    /// Updates `outgoing_status` for the message identified by `handle`.
    ///
    /// Used on push failure or ambiguous outcome. No-ops silently if the handle is absent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the update fails.
    pub async fn update_outgoing_status(
        &self,
        handle: &str,
        status: OutgoingStatus,
    ) -> Result<(), Error> {
        let handle = handle.to_owned();
        self.conn()
            .call(move |conn| {
                conn.prepare_cached(
                    "UPDATE messages SET outgoing_status = ?1 WHERE map_handle = ?2",
                )?
                .execute(params![status, handle])?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Renames the placeholder `map_handle` to the device-assigned handle and simultaneously
    /// sets `outgoing_status` to the supplied value.
    ///
    /// Called on push success: `old_handle` is `"local:{outbox_id}"`, `new_handle` is the
    /// real MAP handle returned by the device. No-ops silently if `old_handle` is absent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the update fails.
    pub async fn promote_outgoing(
        &self,
        old_handle: &str,
        new_handle: &str,
        status: OutgoingStatus,
    ) -> Result<(), Error> {
        let (old, new) = (old_handle.to_owned(), new_handle.to_owned());
        self.conn()
            .call(move |conn| {
                conn.prepare_cached(
                    "UPDATE messages SET map_handle = ?1, outgoing_status = ?2 \
                     WHERE map_handle = ?3",
                )?
                .execute(params![new, status, old])?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Advances `outgoing_status` from `sent_unconfirmed` to `sent_confirmed` for `handle`.
    ///
    /// Called during Sent-folder backfill reconciliation: when a device Sent message matches
    /// a local row that was speculatively created by [`Store::enqueue_send`], this confirms
    /// the device has the message. No-ops if the row does not exist or has a different status.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the update fails.
    pub async fn reconcile_outgoing(&self, handle: &str) -> Result<(), Error> {
        let handle = handle.to_owned();
        self.conn()
            .call(move |conn| {
                conn.prepare_cached(
                    "UPDATE messages SET outgoing_status = 'sent_confirmed' \
                     WHERE map_handle = ?1 AND outgoing_status = 'sent_unconfirmed'",
                )?
                .execute([handle.as_str()])?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Returns all messages with `timestamp_ms` strictly greater than `after_ms`,
    /// ordered oldest-first.
    ///
    /// Pass `0` to retrieve all stored messages.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn messages_since(&self, after_ms: i64) -> Result<Vec<MessageRow>, Error> {
        self.conn()
            .call(
                move |conn: &mut rusqlite::Connection| -> Result<Vec<MessageRow>, rusqlite::Error> {
                    let mut stmt = conn.prepare_cached(
                        "SELECT rowid, map_handle, timestamp_ms, folder, direction, \
                         address, conversation_key, participants, status, synced_at, text, \
                         outgoing_status \
                         FROM messages WHERE timestamp_ms > ?1 ORDER BY timestamp_ms",
                    )?;
                    let rows = stmt
                        .query_map([after_ms], |row| {
                            Ok(MessageRow {
                                rowid: row.get(0)?,
                                map_handle: row.get(1)?,
                                timestamp_ms: row.get(2)?,
                                folder: row.get(3)?,
                                direction: row.get(4)?,
                                address: row.get(5)?,
                                conversation_key: row.get(6)?,
                                participants: row.get(7)?,
                                status: row.get(8)?,
                                synced_at: row.get(9)?,
                                text: row.get(10)?,
                                outgoing_status: row.get(11)?,
                            })
                        })?
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(rows)
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Returns the maximum `timestamp_ms` across all stored messages, or `None` if the store is empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn max_timestamp(&self) -> Result<Option<i64>, Error> {
        self.conn()
            .call(
                |conn: &mut rusqlite::Connection| -> Result<Option<i64>, rusqlite::Error> {
                    conn.query_row("SELECT MAX(timestamp_ms) FROM messages", [], |row| {
                        row.get(0)
                    })
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Returns the `last_sync_at` timestamp from the `meta` table, or `None` if never set.
    ///
    /// A `None` result means no backfill has completed; callers should treat the store as empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn last_sync_at(&self) -> Result<Option<i64>, Error> {
        self.conn()
            .call(
                |conn: &mut rusqlite::Connection| -> Result<Option<i64>, rusqlite::Error> {
                    let mut stmt =
                        conn.prepare_cached("SELECT value FROM meta WHERE key = 'last_sync_at'")?;
                    let mut rows = stmt.query([])?;
                    rows.next()?.map_or_else(
                        || Ok(None),
                        |row| {
                            row.get::<_, String>(0)?
                                .parse::<i64>()
                                .map(Some)
                                .map_err(|e| {
                                    rusqlite::Error::FromSqlConversionFailure(
                                        0,
                                        rusqlite::types::Type::Text,
                                        Box::new(e),
                                    )
                                })
                        },
                    )
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Persists `ms` as the `last_sync_at` anchor in the `meta` table.
    ///
    /// Subsequent calls overwrite the previous value. Callers set this only after
    /// a backfill run completes successfully.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` write fails.
    pub async fn set_last_sync_at(&self, ms: i64) -> Result<(), Error> {
        self.set_meta("last_sync_at", &ms.to_string()).await
    }

    /// Deletes the message identified by `handle`.
    ///
    /// No-ops silently if the handle is not present.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` write fails.
    pub async fn delete_by_handle(&self, handle: &str) -> Result<(), Error> {
        let handle = handle.to_owned();
        self.conn()
            .call(
                move |conn: &mut rusqlite::Connection| -> Result<(), rusqlite::Error> {
                    conn.prepare_cached("DELETE FROM messages WHERE map_handle = ?1")?
                        .execute(params![handle])?;
                    Ok(())
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Updates the `folder` column for the message identified by `handle`.
    ///
    /// No-ops silently if the handle is not present.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` write fails.
    pub async fn update_folder(&self, handle: &str, folder: &str) -> Result<(), Error> {
        let (handle, folder) = (handle.to_owned(), folder.to_owned());
        self.conn()
            .call(
                move |conn: &mut rusqlite::Connection| -> Result<(), rusqlite::Error> {
                    conn.prepare_cached("UPDATE messages SET folder = ?1 WHERE map_handle = ?2")?
                        .execute(params![folder, handle])?;
                    Ok(())
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Updates the `status` column for the message identified by `handle`.
    ///
    /// No-ops silently if the handle is not present.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` write fails.
    pub async fn update_status(&self, handle: &str, status: i32) -> Result<(), Error> {
        let handle = handle.to_owned();
        self.conn()
            .call(
                move |conn: &mut rusqlite::Connection| -> Result<(), rusqlite::Error> {
                    conn.prepare_cached("UPDATE messages SET status = ?1 WHERE map_handle = ?2")?
                        .execute(params![status, handle])?;
                    Ok(())
                },
            )
            .await
            .map_err(Error::Connection)
    }

    /// Returns the raw text value stored under `key` in the `meta` table, or `None` if absent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the query fails.
    pub async fn get_meta(&self, key: &str) -> Result<Option<String>, Error> {
        let key = key.to_owned();
        self.conn()
            .call(move |conn| {
                let mut stmt = conn.prepare_cached("SELECT value FROM meta WHERE key = ?1")?;
                let mut rows = stmt.query([key.as_str()])?;
                rows.next()?
                    .map_or_else(|| Ok(None), |row| row.get(0).map(Some))
            })
            .await
            .map_err(Error::Connection)
    }

    /// Atomically resolves the outbox entry to `Sent` and promotes the speculative message handle.
    ///
    /// Wraps both updates in a single `SQLite` transaction: `outbox.status → 'sent'` with
    /// `resolved_at = now_ms`, and `messages.map_handle` renamed from `old_handle` to
    /// `new_handle` with `outgoing_status → 'sent_unconfirmed'`. Either both writes
    /// succeed or neither is visible — callers are safe against partial-update inconsistency.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the transaction fails at any step.
    pub async fn complete_send(
        &self,
        outbox_id: i64,
        old_handle: &str,
        new_handle: &str,
        now_ms: i64,
    ) -> Result<(), Error> {
        let (old, new) = (old_handle.to_owned(), new_handle.to_owned());
        self.conn()
            .call(move |conn| {
                use crate::row::{OutboxStatus, OutgoingStatus};
                let tx = conn.transaction()?;
                tx.execute(
                    "UPDATE outbox SET status = ?1, resolved_at = ?2, error = NULL WHERE id = ?3",
                    params![OutboxStatus::Sent, now_ms, outbox_id],
                )?;
                tx.execute(
                    "UPDATE messages SET map_handle = ?1, outgoing_status = ?2 \
                     WHERE map_handle = ?3",
                    params![new, OutgoingStatus::SentUnconfirmed, old],
                )?;
                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }

    /// Upserts `value` under `key` in the `meta` table, overwriting any existing entry.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the upsert fails.
    pub async fn set_meta(&self, key: &str, value: &str) -> Result<(), Error> {
        let (key, value) = (key.to_owned(), value.to_owned());
        self.conn()
            .call(move |conn| {
                conn.prepare_cached(
                    "INSERT INTO meta (key, value) VALUES (?1, ?2) \
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                )?
                .execute(params![key, value])?;
                Ok(())
            })
            .await
            .map_err(Error::Connection)
    }
}
