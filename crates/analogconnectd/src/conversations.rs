use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::Serialize;
use thiserror::Error;

use crate::media_auth::RandomSource;

const CONVERSATION_ID_BYTES: usize = 16;
pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const MAX_PAGE_SIZE: usize = 100;
const MAX_CURSOR_OFFSET: usize = 1_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    Received,
    Sent,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StoredConversation {
    pub conversation_key: String,
    pub display_address: String,
    pub participant_count: usize,
    pub latest_unix_millis: i64,
    pub message_count: u64,
    pub unread_count: u64,
    pub latest_outgoing_state: Option<String>,
    pub group_title: Option<String>,
    pub is_ancs_group: bool,
    pub identity_conflict: bool,
}

impl std::fmt::Debug for StoredConversation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StoredConversation([private fields redacted])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StoredMessage {
    pub local_id: u64,
    pub address: String,
    pub conversation_key: String,
    pub timestamp_unix_millis: i64,
    pub direction: MessageDirection,
    pub body: String,
    pub read: bool,
    pub outgoing_state: Option<String>,
}

impl std::fmt::Debug for StoredMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StoredMessage([private fields redacted])")
    }
}

#[derive(Clone)]
pub enum ConversationRepository {
    Unavailable,
    InMemory(Arc<InMemoryConversationRepository>),
    Imsg(Arc<ImsgConversationRepository>),
}

impl ConversationRepository {
    #[must_use]
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }

    pub async fn conversations(&self) -> Result<Vec<StoredConversation>, ConversationError> {
        match self {
            Self::Unavailable => Err(ConversationError::Unavailable),
            Self::InMemory(repository) => repository.conversations(),
            Self::Imsg(repository) => repository.conversations().await,
        }
    }

    pub async fn messages_for(
        &self,
        conversation_key: &str,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<StoredMessage>, ConversationError> {
        match self {
            Self::Unavailable => Err(ConversationError::Unavailable),
            Self::InMemory(repository) => repository.messages_for(conversation_key, limit, offset),
            Self::Imsg(repository) => {
                repository
                    .messages_for(conversation_key, limit, offset)
                    .await
            }
        }
    }
}

pub struct ImsgConversationRepository {
    store: tokio::sync::OnceCell<Arc<imsg_store::Store>>,
}

impl ImsgConversationRepository {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            store: tokio::sync::OnceCell::const_new(),
        }
    }

    async fn store(&self) -> Result<&Arc<imsg_store::Store>, ConversationError> {
        self.store
            .get_or_try_init(|| async {
                let (path, key) = tokio::task::spawn_blocking(|| {
                    let config =
                        imsg_config::load(None).map_err(|_| ConversationError::Unavailable)?;
                    let path = config
                        .store
                        .resolve()
                        .ok_or(ConversationError::Unavailable)?;
                    let ready =
                        imsg_keyring::init_store().map_err(|_| ConversationError::Unavailable)?;
                    let key = imsg_keyring::get_or_create_db_key(&ready)
                        .map_err(|_| ConversationError::Unavailable)?;
                    Ok::<_, ConversationError>((path, key))
                })
                .await
                .map_err(|_| ConversationError::Unavailable)??;
                let store = imsg_store::Store::open(path, key)
                    .await
                    .map_err(|_| ConversationError::Unavailable)?;
                Ok(Arc::new(store))
            })
            .await
    }

    async fn conversations(&self) -> Result<Vec<StoredConversation>, ConversationError> {
        self.store()
            .await?
            .threads()
            .await
            .map_err(|_| ConversationError::Unavailable)?
            .into_iter()
            .map(|row| {
                let count = usize::try_from(row.participant_count)
                    .map_err(|_| ConversationError::Unavailable)?;
                Ok(StoredConversation {
                    conversation_key: row.conversation_key,
                    participant_count: count,
                    display_address: if count <= 1 || row.participants.is_empty() {
                        row.address
                    } else if row.participants.len() > 128 {
                        format!("Group conversation ({count} participants)")
                    } else {
                        row.participants.replace(',', ", ")
                    },
                    latest_unix_millis: row.latest_ms,
                    message_count: u64::try_from(row.total)
                        .map_err(|_| ConversationError::Unavailable)?,
                    unread_count: u64::try_from(row.unread)
                        .map_err(|_| ConversationError::Unavailable)?,
                    latest_outgoing_state: row
                        .latest_outgoing_status
                        .map(|status| status.to_string()),
                    group_title: row.group_title,
                    is_ancs_group: row.is_ancs_group,
                    identity_conflict: row.identity_conflict,
                })
            })
            .collect()
    }

    async fn messages_for(
        &self,
        conversation_key: &str,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<StoredMessage>, ConversationError> {
        self.store()
            .await?
            .list_conversation_messages(conversation_key, limit, offset)
            .await
            .map_err(|_| ConversationError::Unavailable)?
            .into_iter()
            .map(|row| {
                Ok(StoredMessage {
                    local_id: u64::try_from(row.rowid)
                        .map_err(|_| ConversationError::Unavailable)?,
                    address: row.address,
                    conversation_key: row.conversation_key,
                    timestamp_unix_millis: row.timestamp_ms,
                    direction: match row.direction {
                        imsg_store::Direction::Received => MessageDirection::Received,
                        imsg_store::Direction::Sent => MessageDirection::Sent,
                    },
                    body: row.text,
                    read: row.status == imsg_store::STATUS_READ,
                    outgoing_state: row.outgoing_status.map(|status| status.to_string()),
                })
            })
            .collect()
    }
}

impl Default for ImsgConversationRepository {
    fn default() -> Self {
        Self::new()
    }
}

pub struct InMemoryConversationRepository {
    conversations: RwLock<Vec<StoredConversation>>,
    messages: RwLock<Vec<StoredMessage>>,
}

impl InMemoryConversationRepository {
    #[must_use]
    pub fn new(conversations: Vec<StoredConversation>, messages: Vec<StoredMessage>) -> Self {
        Self {
            conversations: RwLock::new(conversations),
            messages: RwLock::new(messages),
        }
    }

    fn conversations(&self) -> Result<Vec<StoredConversation>, ConversationError> {
        let mut rows = self
            .conversations
            .read()
            .map_err(|_| ConversationError::Unavailable)?
            .clone();
        rows.sort_by(|left, right| {
            right
                .latest_unix_millis
                .cmp(&left.latest_unix_millis)
                .then_with(|| left.conversation_key.cmp(&right.conversation_key))
        });
        Ok(rows)
    }

    fn messages_for(
        &self,
        conversation_key: &str,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<StoredMessage>, ConversationError> {
        let mut rows: Vec<_> = self
            .messages
            .read()
            .map_err(|_| ConversationError::Unavailable)?
            .iter()
            .filter(|message| message.conversation_key == conversation_key)
            .cloned()
            .collect();
        rows.sort_by(|left, right| {
            right
                .timestamp_unix_millis
                .cmp(&left.timestamp_unix_millis)
                .then_with(|| right.local_id.cmp(&left.local_id))
        });
        let start = usize::from(offset).min(rows.len());
        let end = start.saturating_add(usize::from(limit)).min(rows.len());
        Ok(rows[start..end].to_vec())
    }
}

#[derive(Default)]
pub struct ConversationAliases {
    key_to_id: HashMap<String, String>,
    id_to_key: HashMap<String, String>,
}

impl ConversationAliases {
    pub fn expose_stable_id(
        &mut self,
        conversation_key: &str,
    ) -> Result<String, ConversationError> {
        if let Some(existing) = self.key_to_id.get(conversation_key) {
            return Ok(existing.clone());
        }
        if let Some(existing_key) = self.id_to_key.get(conversation_key)
            && existing_key != conversation_key
        {
            return Err(ConversationError::RandomUnavailable);
        }
        self.key_to_id
            .insert(conversation_key.to_owned(), conversation_key.to_owned());
        self.id_to_key
            .insert(conversation_key.to_owned(), conversation_key.to_owned());
        Ok(conversation_key.to_owned())
    }

    pub fn id_for<R: RandomSource>(
        &mut self,
        conversation_key: &str,
        random: &mut R,
    ) -> Result<String, ConversationError> {
        if let Some(existing) = self.key_to_id.get(conversation_key) {
            return Ok(existing.clone());
        }
        for _ in 0..4 {
            let mut bytes = [0_u8; CONVERSATION_ID_BYTES];
            random
                .fill(&mut bytes)
                .map_err(|_| ConversationError::RandomUnavailable)?;
            let id = encode_hex(&bytes);
            if !self.id_to_key.contains_key(&id) {
                self.key_to_id
                    .insert(conversation_key.to_owned(), id.clone());
                self.id_to_key
                    .insert(id.clone(), conversation_key.to_owned());
                return Ok(id);
            }
        }
        Err(ConversationError::RandomUnavailable)
    }

    #[must_use]
    pub fn key_for(&self, id: &str) -> Option<&str> {
        self.id_to_key.get(id).map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageCursor(usize);

impl PageCursor {
    pub fn parse(value: Option<&str>) -> Result<Self, ConversationError> {
        let Some(value) = value else {
            return Ok(Self(0));
        };
        if value.len() != 16 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ConversationError::InvalidCursor);
        }
        let offset =
            usize::from_str_radix(value, 16).map_err(|_| ConversationError::InvalidCursor)?;
        if offset > MAX_CURSOR_OFFSET {
            return Err(ConversationError::InvalidCursor);
        }
        Ok(Self(offset))
    }

    #[must_use]
    pub fn offset(self) -> usize {
        self.0
    }

    #[must_use]
    pub fn encode(self) -> String {
        format!("{:016x}", self.0)
    }

    #[must_use]
    pub fn advance(self, count: usize) -> Self {
        Self(self.0.saturating_add(count).min(MAX_CURSOR_OFFSET))
    }
}

#[derive(Serialize)]
pub struct ConversationItem {
    pub conversation_id: String,
    pub display_address: String,
    pub display_name: Option<String>,
    pub is_group: bool,
    pub reply_supported: bool,
    pub latest_unix_millis: i64,
    pub message_count: u64,
    pub unread_count: u64,
    pub latest_outgoing_state: Option<String>,
    pub kind: ConversationKindResponse,
    pub title: String,
    pub can_reply: bool,
    pub identity_conflict: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationKindResponse {
    Private,
    Group,
    Ambiguous,
}

impl std::fmt::Debug for ConversationItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ConversationItem([private fields redacted])")
    }
}

#[derive(Serialize)]
pub struct ConversationPage {
    pub items: Vec<ConversationItem>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirectionResponse {
    Received,
    Sent,
}

#[derive(Serialize)]
pub struct MessageItem {
    pub message_id: String,
    pub timestamp_unix_millis: i64,
    pub direction: MessageDirectionResponse,
    pub peer_address: String,
    pub body: String,
    pub read: bool,
    pub outgoing_state: Option<String>,
}

impl std::fmt::Debug for MessageItem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MessageItem([private fields redacted])")
    }
}

#[derive(Serialize)]
pub struct MessagePage {
    pub items: Vec<MessageItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConversationError {
    #[error("conversation store unavailable")]
    Unavailable,
    #[error("conversation identifier expired")]
    Expired,
    #[error("conversation cursor is invalid")]
    InvalidCursor,
    #[error("conversation identifier generation unavailable")]
    RandomUnavailable,
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureRandom(u8);

    impl RandomSource for FixtureRandom {
        type Error = ();

        fn fill(&mut self, bytes: &mut [u8]) -> Result<(), Self::Error> {
            bytes.fill(self.0);
            self.0 = self.0.wrapping_add(1);
            Ok(())
        }
    }

    #[test]
    fn aliases_are_stable_opaque_and_reverse_mapped() {
        let mut aliases = ConversationAliases::default();
        let mut random = FixtureRandom(7);
        let first = aliases.id_for("synthetic-address-a", &mut random).unwrap();
        let repeated = aliases.id_for("synthetic-address-a", &mut random).unwrap();
        let second = aliases.id_for("synthetic-address-b", &mut random).unwrap();
        assert_eq!(first, repeated);
        assert_ne!(first, second);
        assert_eq!(aliases.key_for(&first), Some("synthetic-address-a"));
        assert!(!first.contains("address"));
    }

    #[test]
    fn cursors_are_canonical_bounded_and_reject_invalid_input() {
        let cursor = PageCursor::parse(None).unwrap().advance(50);
        assert_eq!(cursor.offset(), 50);
        assert_eq!(
            PageCursor::parse(Some(&cursor.encode())).unwrap().offset(),
            50
        );
        for invalid in ["", "1", "zzzzzzzzzzzzzzzz", "ffffffffffffffff"] {
            assert_eq!(
                PageCursor::parse(Some(invalid)).unwrap_err(),
                ConversationError::InvalidCursor
            );
        }
    }

    #[tokio::test]
    async fn in_memory_repository_orders_without_revealing_debug_data() {
        let private_marker = "synthetic-private-marker";
        let repository =
            ConversationRepository::InMemory(Arc::new(InMemoryConversationRepository::new(
                vec![
                    StoredConversation {
                        conversation_key: "synthetic-key-a".to_owned(),
                        display_address: "synthetic-address-a".to_owned(),
                        participant_count: 1,
                        latest_unix_millis: 1,
                        message_count: 1,
                        unread_count: 0,
                        latest_outgoing_state: None,
                        group_title: None,
                        is_ancs_group: false,
                        identity_conflict: false,
                    },
                    StoredConversation {
                        conversation_key: "synthetic-key-b".to_owned(),
                        display_address: "synthetic-address-b".to_owned(),
                        participant_count: 1,
                        latest_unix_millis: 2,
                        message_count: 1,
                        unread_count: 1,
                        latest_outgoing_state: None,
                        group_title: None,
                        is_ancs_group: false,
                        identity_conflict: false,
                    },
                ],
                vec![StoredMessage {
                    local_id: 1,
                    address: "synthetic-address-a".to_owned(),
                    conversation_key: "synthetic-key-a".to_owned(),
                    timestamp_unix_millis: 1,
                    direction: MessageDirection::Received,
                    body: private_marker.to_owned(),
                    read: false,
                    outgoing_state: None,
                }],
            )));
        let conversations = repository.conversations().await.unwrap();
        assert_eq!(conversations[0].display_address, "synthetic-address-b");
        let messages = repository
            .messages_for("synthetic-key-a", 10, 0)
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        assert!(!format!("{:?}", messages[0]).contains(private_marker));
    }

    #[tokio::test]
    async fn typed_imsg_store_adapter_reads_concurrently_and_redacts_rows() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            imsg_store::Store::open(
                directory.path().join("messages.db"),
                secrecy::SecretBox::new(Box::new([7_u8; 32])),
            )
            .await
            .unwrap(),
        );
        let private_marker = "synthetic-private-store-body";
        store
            .upsert(imsg_store::NewMessage {
                map_handle: "synthetic-handle".to_owned(),
                timestamp_ms: 10,
                folder: "inbox".to_owned(),
                direction: imsg_store::Direction::Received,
                address: "synthetic-address".to_owned(),
                conversation_key: "synthetic-group-conversation".to_owned(),
                participants: "synthetic-address, synthetic-local, synthetic-other".to_owned(),
                status: imsg_store::STATUS_UNREAD,
                synced_at: 11,
                text: private_marker.to_owned(),
                outgoing_status: None,
            })
            .await
            .unwrap();
        store
            .upsert(imsg_store::NewMessage {
                map_handle: "synthetic-group-handle".to_owned(),
                timestamp_ms: 12,
                folder: "inbox".to_owned(),
                direction: imsg_store::Direction::Received,
                address: "synthetic-other".to_owned(),
                conversation_key: "synthetic-group-conversation".to_owned(),
                participants: "synthetic-address, synthetic-local, synthetic-other".to_owned(),
                status: imsg_store::STATUS_READ,
                synced_at: 13,
                text: "synthetic-private-group-body".to_owned(),
                outgoing_status: None,
            })
            .await
            .unwrap();
        let adapter = Arc::new(ImsgConversationRepository::new());
        assert!(adapter.store.set(store).is_ok());
        let repository = ConversationRepository::Imsg(adapter);

        let (conversations, messages) = tokio::join!(
            repository.conversations(),
            repository.messages_for("synthetic-group-conversation", 50, 0)
        );
        let conversations = conversations.unwrap();
        let messages = messages.unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].unread_count, 1);
        assert_eq!(conversations[0].participant_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].body, private_marker);
        assert!(!format!("{:?}", messages[0]).contains(private_marker));
    }
}
