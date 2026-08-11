use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use analogconnect_core::{Contact, ContactSource, ContactSyncState, parse_imsg_contacts};
use rusqlite::{Connection, params};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ContactSummary {
    pub contact_count: u64,
    pub phone_count: u64,
    pub last_sync_unix_seconds: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct ContactItem {
    pub display_name: Option<String>,
    pub phone_numbers: Vec<String>,
}

impl std::fmt::Debug for ContactItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ContactItem([private fields redacted])")
    }
}

impl From<Contact> for ContactItem {
    fn from(contact: Contact) -> Self {
        Self {
            display_name: contact.display_name,
            phone_numbers: contact
                .phones
                .into_iter()
                .map(|phone| phone.display)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ContactPage {
    pub items: Vec<ContactItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Error)]
pub enum ImsgSourceError {
    #[error("PBAP client could not be started")]
    Spawn,
    #[error("PBAP client failed")]
    Failed,
    #[error("PBAP client returned invalid text")]
    InvalidText,
    #[error("PBAP client returned an invalid contact payload")]
    InvalidPayload,
}

/// Privacy boundary around `imsg contacts --raw`.
///
/// Command output is parsed in memory and is never included in errors or logs.
pub struct ImsgContactSource {
    executable: PathBuf,
}

impl ImsgContactSource {
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for ImsgContactSource {
    fn default() -> Self {
        Self::new("imsg")
    }
}

impl ContactSource for ImsgContactSource {
    type Error = ImsgSourceError;

    fn pull_all(&self) -> Result<Vec<Contact>, Self::Error> {
        let output = Command::new(&self.executable)
            .args(["contacts", "--raw"])
            .output()
            .map_err(|_| ImsgSourceError::Spawn)?;
        if !output.status.success() {
            return Err(ImsgSourceError::Failed);
        }
        let stdout = String::from_utf8(output.stdout).map_err(|_| ImsgSourceError::InvalidText)?;
        parse_imsg_contacts(&stdout).map_err(|_| ImsgSourceError::InvalidPayload)
    }
}

#[derive(Debug, Error)]
pub enum ContactStoreError {
    #[error("contact database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("contact database file could not be secured")]
    FilePermissions(#[from] std::io::Error),
    #[error("contact store lock was poisoned")]
    LockPoisoned,
}

pub struct ContactStore {
    connection: Mutex<Connection>,
}

impl ContactStore {
    pub fn open(path: &Path) -> Result<Self, ContactStoreError> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(path)?;
        #[cfg(unix)]
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        drop(file);

        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self, ContactStoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, ContactStoreError> {
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS contacts (
                 id INTEGER PRIMARY KEY,
                 display_name TEXT
             );
             CREATE TABLE IF NOT EXISTS contact_phones (
                 id INTEGER PRIMARY KEY,
                 contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
                 display_number TEXT NOT NULL,
                 normalized_number TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS contact_name_idx ON contacts(display_name COLLATE NOCASE);
             CREATE INDEX IF NOT EXISTS phone_normalized_idx ON contact_phones(normalized_number);",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn replace_all(&self, contacts: &[Contact]) -> Result<ContactSummary, ContactStoreError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ContactStoreError::LockPoisoned)?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM contacts", [])?;

        let mut phone_count = 0_u64;
        for contact in contacts {
            transaction.execute(
                "INSERT INTO contacts (display_name) VALUES (?1)",
                params![contact.display_name],
            )?;
            let contact_id = transaction.last_insert_rowid();
            for phone in &contact.phones {
                transaction.execute(
                    "INSERT INTO contact_phones
                     (contact_id, display_number, normalized_number) VALUES (?1, ?2, ?3)",
                    params![contact_id, phone.display, phone.normalized],
                )?;
                phone_count += 1;
            }
        }
        transaction.commit()?;

        Ok(ContactSummary {
            contact_count: contacts.len() as u64,
            phone_count,
            last_sync_unix_seconds: None,
        })
    }

    pub fn summary(&self) -> Result<ContactSummary, ContactStoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContactStoreError::LockPoisoned)?;
        let contact_count = connection.query_row("SELECT COUNT(*) FROM contacts", [], |row| {
            row.get::<_, u64>(0)
        })?;
        let phone_count =
            connection.query_row("SELECT COUNT(*) FROM contact_phones", [], |row| {
                row.get::<_, u64>(0)
            })?;
        Ok(ContactSummary {
            contact_count,
            phone_count,
            last_sync_unix_seconds: None,
        })
    }

    pub fn search_names(&self, query: &str, limit: u16) -> Result<Vec<Contact>, ContactStoreError> {
        self.search_names_page(query, limit, 0)
    }

    pub fn search_names_page(
        &self,
        query: &str,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Contact>, ContactStoreError> {
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContactStoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT id, display_name FROM contacts
             WHERE display_name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
             ORDER BY display_name COLLATE NOCASE, id LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![pattern, limit.min(101), offset], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })?;

        let mut contacts = Vec::new();
        for row in rows {
            let (id, display_name) = row?;
            contacts.push(Contact {
                display_name,
                phones: load_phones(&connection, id)?,
            });
        }
        Ok(contacts)
    }

    pub fn lookup_display_name(&self, number: &str) -> Result<Option<String>, ContactStoreError> {
        Ok(self
            .lookup_caller(number)?
            .and_then(|contact| contact.display_name))
    }

    pub fn lookup_caller(&self, incoming: &str) -> Result<Option<Contact>, ContactStoreError> {
        let Some(parsed) = analogconnect_core::PhoneNumber::parse(incoming) else {
            return Ok(None);
        };
        let key = parsed.match_key();
        let suffix = if key.len() >= 10 {
            &key[key.len() - 10..]
        } else {
            key
        };
        let connection = self
            .connection
            .lock()
            .map_err(|_| ContactStoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT DISTINCT c.id, c.display_name
             FROM contacts c JOIN contact_phones p ON p.contact_id = c.id
             WHERE TRIM(p.normalized_number, '+') = ?1
                OR (?2 != '' AND TRIM(p.normalized_number, '+') LIKE '%' || ?2)
             ORDER BY c.id LIMIT 2",
        )?;
        let matches = statement
            .query_map(params![key, suffix], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        if matches.len() != 1 {
            return Ok(None);
        }
        let (id, display_name) = &matches[0];
        Ok(Some(Contact {
            display_name: display_name.clone(),
            phones: load_phones(&connection, *id)?,
        }))
    }
}

#[derive(Debug, Error)]
pub enum ContactSyncError {
    #[error("PBAP contact pull failed")]
    Source,
    #[error("contact persistence failed")]
    Store,
    #[error("contact synchronization state lock was poisoned")]
    LockPoisoned,
}

/// Full-snapshot PBAP synchronizer. Existing contacts remain intact unless both
/// the pull and the transactional replacement succeed.
pub struct ContactSynchronizer<S> {
    source: S,
    store: ContactStore,
    state: Mutex<ContactSyncState>,
}

impl<S> ContactSynchronizer<S>
where
    S: ContactSource,
{
    #[must_use]
    pub fn new(source: S, store: ContactStore) -> Self {
        Self {
            source,
            store,
            state: Mutex::new(ContactSyncState::Idle),
        }
    }

    pub fn state(&self) -> Result<ContactSyncState, ContactSyncError> {
        self.state
            .lock()
            .map(|state| *state)
            .map_err(|_| ContactSyncError::LockPoisoned)
    }

    pub fn sync(&self) -> Result<ContactSummary, ContactSyncError> {
        self.set_state(ContactSyncState::Syncing)?;
        let contacts = match self.source.pull_all() {
            Ok(contacts) => contacts,
            Err(_) => {
                self.set_state(ContactSyncState::BackingOff)?;
                return Err(ContactSyncError::Source);
            }
        };

        let mut summary = match self.store.replace_all(&contacts) {
            Ok(summary) => summary,
            Err(_) => {
                self.set_state(ContactSyncState::Error)?;
                return Err(ContactSyncError::Store);
            }
        };
        summary.last_sync_unix_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs());
        self.set_state(ContactSyncState::Idle)?;
        Ok(summary)
    }

    fn set_state(&self, next: ContactSyncState) -> Result<(), ContactSyncError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ContactSyncError::LockPoisoned)?;
        state
            .transition_to(next)
            .map_err(|_| ContactSyncError::LockPoisoned)
    }
}

fn load_phones(
    connection: &Connection,
    contact_id: i64,
) -> Result<Vec<analogconnect_core::PhoneNumber>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT display_number, normalized_number FROM contact_phones
         WHERE contact_id = ?1 ORDER BY id",
    )?;
    statement
        .query_map([contact_id], |row| {
            Ok(analogconnect_core::PhoneNumber {
                display: row.get(0)?,
                normalized: row.get(1)?,
            })
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use analogconnect_core::PhoneNumber;

    fn number(parts: &[&str]) -> String {
        parts.concat()
    }

    fn contact(name: &str, parts: &[&str]) -> Contact {
        Contact {
            display_name: Some(name.to_owned()),
            phones: vec![PhoneNumber::parse(&number(parts)).unwrap()],
        }
    }

    struct FixtureSource(Result<Vec<Contact>, ()>);

    impl ContactSource for FixtureSource {
        type Error = ();

        fn pull_all(&self) -> Result<Vec<Contact>, Self::Error> {
            self.0.clone()
        }
    }

    #[test]
    fn replacement_is_atomic_and_searchable() {
        let store = ContactStore::in_memory().unwrap();
        let alpha = contact("Example Alpha", &["+1", "202", "555", "0101"]);
        let beta = contact("Example Beta", &["+1", "202", "555", "0102"]);
        let summary = store.replace_all(&[alpha, beta]).unwrap();
        assert_eq!(summary.contact_count, 2);
        assert_eq!(summary.phone_count, 2);

        let matches = store.search_names("alpha", 10).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].display_name.as_deref(), Some("Example Alpha"));
    }

    #[test]
    fn search_pages_and_unique_name_lookup_preserve_number_target() {
        let store = ContactStore::in_memory().unwrap();
        let alpha = contact("Example Alpha", &["+1", "202", "555", "0101"]);
        let beta = contact("Example Beta", &["+1", "202", "555", "0102"]);
        store.replace_all(&[alpha, beta]).unwrap();

        let page = store.search_names_page("Example", 1, 1).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].display_name.as_deref(), Some("Example Beta"));
        assert_eq!(
            store
                .lookup_display_name("+1 (202) 555-0101")
                .unwrap()
                .as_deref(),
            Some("Example Alpha")
        );
    }

    #[test]
    fn caller_matching_uses_normalized_suffix() {
        let store = ContactStore::in_memory().unwrap();
        store
            .replace_all(&[contact("Example Alpha", &["+1", "202", "555", "0101"])])
            .unwrap();
        let incoming = number(&["202", "555", "0101"]);
        let found = store.lookup_caller(&incoming).unwrap().unwrap();
        assert_eq!(found.display_name.as_deref(), Some("Example Alpha"));
    }

    #[test]
    fn ambiguous_suffix_does_not_guess() {
        let store = ContactStore::in_memory().unwrap();
        let suffix = ["202", "555", "0101"];
        store
            .replace_all(&[
                contact("Example Alpha", &["+1", suffix[0], suffix[1], suffix[2]]),
                contact("Example Beta", &["+9", suffix[0], suffix[1], suffix[2]]),
            ])
            .unwrap();
        assert!(store.lookup_caller(&number(&suffix)).unwrap().is_none());
    }

    #[test]
    fn percent_in_search_is_literal() {
        let store = ContactStore::in_memory().unwrap();
        store
            .replace_all(&[contact("Example Alpha", &["+1", "202", "555", "0101"])])
            .unwrap();
        assert!(store.search_names("%", 10).unwrap().is_empty());
    }

    #[test]
    fn successful_sync_replaces_snapshot_and_returns_to_idle() {
        let source = FixtureSource(Ok(vec![contact(
            "Example Alpha",
            &["+1", "202", "555", "0101"],
        )]));
        let synchronizer = ContactSynchronizer::new(source, ContactStore::in_memory().unwrap());
        let summary = synchronizer.sync().unwrap();
        assert_eq!(summary.contact_count, 1);
        assert!(summary.last_sync_unix_seconds.is_some());
        assert_eq!(synchronizer.state().unwrap(), ContactSyncState::Idle);
    }

    #[test]
    fn failed_pull_preserves_existing_snapshot_and_backs_off() {
        let store = ContactStore::in_memory().unwrap();
        store
            .replace_all(&[contact("Example Alpha", &["+1", "202", "555", "0101"])])
            .unwrap();
        let synchronizer = ContactSynchronizer::new(FixtureSource(Err(())), store);
        assert!(matches!(synchronizer.sync(), Err(ContactSyncError::Source)));
        assert_eq!(synchronizer.state().unwrap(), ContactSyncState::BackingOff);
        assert_eq!(synchronizer.store.summary().unwrap().contact_count, 1);
    }
}
