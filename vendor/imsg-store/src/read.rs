//! Read queries: point lookup, filtered list, and thread aggregation.

use rusqlite::OptionalExtension as _;

use crate::{
    row::{MessageRow, ThreadRow, STATUS_UNREAD},
    Error, OutgoingStatus, Store,
};

/// Maps a `messages` result row (columns 0–11) to a [`MessageRow`].
///
/// Column order must match every `SELECT` that uses this mapper:
/// `rowid, map_handle, timestamp_ms, folder, direction, address, conversation_key,
/// participants, status, synced_at, text, outgoing_status`.
fn map_msg_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageRow> {
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
}

/// Builds the SQL and bound parameters for [`Store::list_messages`].
///
/// Always appends `ORDER BY timestamp_ms DESC LIMIT ? OFFSET ?`; `limit` and `offset` are the
/// final two parameters. Intended to be called inside a `tokio_rusqlite` closure so the
/// returned `Vec<Box<dyn ToSql>>` does not need to be `Send`.
fn build_list_query(
    folder: Option<String>,
    unread_only: bool,
    from: Option<String>,
    since_ms: Option<i64>,
    limit: u16,
    offset: u16,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = String::from(
        "SELECT rowid, map_handle, timestamp_ms, folder, direction, address, \
         conversation_key, participants, status, synced_at, text, outgoing_status \
         FROM messages WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(6);
    if let Some(f) = folder {
        sql.push_str(" AND folder = ?");
        params.push(Box::new(f));
    }
    if unread_only {
        sql.push_str(" AND status = ?");
        params.push(Box::new(STATUS_UNREAD));
    }
    if let Some(addr) = from {
        sql.push_str(" AND address = ?");
        params.push(Box::new(addr));
    }
    if let Some(since) = since_ms {
        sql.push_str(" AND timestamp_ms >= ?");
        params.push(Box::new(since));
    }
    sql.push_str(" ORDER BY timestamp_ms DESC LIMIT ? OFFSET ?");
    params.push(Box::new(i64::from(limit)));
    params.push(Box::new(i64::from(offset)));
    (sql, params)
}

impl Store {
    /// Returns the message with the given `map_handle`, or `None` if absent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn get_by_handle(&self, handle: &str) -> Result<Option<MessageRow>, Error> {
        let handle = handle.to_owned();
        self.conn()
            .call(move |conn: &mut rusqlite::Connection| {
                conn.prepare_cached(
                    "SELECT rowid, map_handle, timestamp_ms, folder, direction, address, \
                     conversation_key, participants, status, synced_at, text, outgoing_status \
                     FROM messages WHERE map_handle = ?1 LIMIT 1",
                )?
                .query_row([handle.as_str()], map_msg_row)
                .optional()
            })
            .await
            .map_err(Error::Connection)
    }

    /// Returns messages matching all supplied criteria, newest-first.
    ///
    /// `folder` restricts to a single folder leaf (`"inbox"`, `"sent"`, etc.); `None` searches
    /// all folders. `unread_only` adds `status = 0`. `from` matches `address` exactly.
    /// `since_ms` is an inclusive lower bound on `timestamp_ms`. `limit` and `offset` page the
    /// result; pass `1024` and `0` for the default first page.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn list_messages(
        &self,
        folder: Option<&str>,
        unread_only: bool,
        from: Option<&str>,
        since_ms: Option<i64>,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<MessageRow>, Error> {
        let folder = folder.map(str::to_owned);
        let from = from.map(str::to_owned);
        self.conn()
            .call(move |conn: &mut rusqlite::Connection| {
                let (sql, params) =
                    build_list_query(folder, unread_only, from, since_ms, limit, offset);
                let param_refs: Vec<&dyn rusqlite::ToSql> =
                    params.iter().map(std::convert::AsRef::as_ref).collect();
                let mut stmt = conn.prepare_cached(&sql)?;
                let rows = stmt.query_map(param_refs.as_slice(), map_msg_row)?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(Error::Connection)
    }

    /// Returns one participant-set conversation newest-first.
    ///
    /// Unlike [`Store::list_messages`], this query uses the canonical private
    /// `conversation_key` rather than one sender/recipient address.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn list_conversation_messages(
        &self,
        conversation_key: &str,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<MessageRow>, Error> {
        let conversation_key = conversation_key.to_owned();
        self.conn()
            .call(move |conn: &mut rusqlite::Connection| {
                let mut stmt = conn.prepare_cached(
                    "SELECT rowid, map_handle, timestamp_ms, folder, direction, address, \
                     conversation_key, participants, status, synced_at, text, outgoing_status \
                     FROM messages WHERE conversation_key = ?1 \
                       AND folder IN ('inbox', 'sent', 'telecom/msg/inbox', 'telecom/msg/sent') \
                     ORDER BY timestamp_ms DESC, rowid DESC LIMIT ?2 OFFSET ?3",
                )?;
                let rows = stmt.query_map(
                    rusqlite::params![conversation_key, i64::from(limit), i64::from(offset)],
                    map_msg_row,
                )?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(Error::Connection)
    }

    /// Returns a per-participant-set thread summary, most-recent-first.
    ///
    /// Groups all stored messages by `conversation_key`, counting total messages and unread
    /// received messages (`status = 0`, `direction = 0`). Rows without a key are excluded.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Connection`] if the async dispatch or underlying `SQLite` read fails.
    pub async fn threads(&self) -> Result<Vec<ThreadRow>, Error> {
        self.conn()
            .call(|conn: &mut rusqlite::Connection| {
                let mut stmt = conn.prepare_cached(
                    "SELECT m.conversation_key, \
                            (SELECT m2.address FROM messages m2 \
                             WHERE m2.conversation_key = m.conversation_key \
                               AND m2.folder IN \
                                   ('inbox', 'sent', 'telecom/msg/inbox', 'telecom/msg/sent') \
                             ORDER BY m2.timestamp_ms DESC, m2.rowid DESC LIMIT 1), \
                            GROUP_CONCAT(DISTINCT CASE WHEN m.address != '' THEN m.address END), \
                            COUNT(DISTINCT CASE WHEN m.address != '' THEN m.address END), \
                            MAX(m.timestamp_ms) AS latest_ms, \
                            COUNT(*) AS total, \
                            SUM(CASE WHEN m.status = 0 AND m.direction = 0 THEN 1 ELSE 0 END) \
                                AS unread, \
                            (SELECT m2.outgoing_status FROM messages m2 \
                             WHERE m2.conversation_key = m.conversation_key \
                               AND m2.folder IN \
                                   ('inbox', 'sent', 'telecom/msg/inbox', 'telecom/msg/sent') \
                             ORDER BY m2.timestamp_ms DESC, m2.rowid DESC LIMIT 1) \
                                AS latest_outgoing_status, \
                            g.display_subtitle, \
                            CASE WHEN g.group_id IS NULL THEN 0 ELSE 1 END AS is_ancs_group, \
                            COALESCE(g.identity_conflict, 0) AS identity_conflict \
                     FROM messages m \
                     LEFT JOIN ancs_group_conversations g \
                       ON g.group_id = m.conversation_key \
                     WHERE m.conversation_key != '' \
                       AND m.folder IN \
                           ('inbox', 'sent', 'telecom/msg/inbox', 'telecom/msg/sent') \
                     GROUP BY m.conversation_key ORDER BY latest_ms DESC",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(ThreadRow {
                        conversation_key: row.get(0)?,
                        address: row.get(1)?,
                        participants: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        participant_count: row.get(3)?,
                        latest_ms: row.get(4)?,
                        total: row.get(5)?,
                        unread: row.get(6)?,
                        latest_outgoing_status: row
                            .get::<_, Option<String>>(7)?
                            .and_then(|s| s.parse::<OutgoingStatus>().ok()),
                        group_title: row.get(8)?,
                        is_ancs_group: row.get::<_, i64>(9)? != 0,
                        identity_conflict: row.get::<_, i64>(10)? != 0,
                    })
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(Error::Connection)
    }
}
