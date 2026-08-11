//! Message listing, retrieval, push, and status operations (`GetMessage`, `PushMessage`, `SetMessageStatus`).

use crate::{params, MapError};
use bytes::Bytes;

/// Read/unread state filter for `ListMessages`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStatus {
    /// Wire value `0x01`; excludes read messages from results.
    Unread,
    /// Wire value `0x02`; excludes unread messages from results.
    Read,
}

impl ReadStatus {
    pub(crate) const fn to_wire(self) -> u8 {
        match self {
            Self::Unread => 0x01,
            Self::Read => 0x02,
        }
    }
}

/// Default fetches up to 1024 messages with no filters applied.
#[derive(Debug, Clone)]
pub struct ListMessagesFilter {
    /// Maximum entries to return; capped by the device's own limit.
    pub max_count: u16,
    /// Zero-based index of the first entry to return.
    pub offset: u16,
    /// Omit to return all messages regardless of read state.
    pub read_status: Option<ReadStatus>,
    /// Filter by sender/originator address. Null bytes prohibited.
    pub originating_address: Option<String>,
    /// Earliest message timestamp, format `YYYYMMDDTHHMMSS[±HHMM]`. Null bytes prohibited.
    pub period_begin: Option<String>,
    /// Latest message timestamp, format `YYYYMMDDTHHMMSS[±HHMM]`. Null bytes prohibited.
    pub period_end: Option<String>,
}

impl Default for ListMessagesFilter {
    fn default() -> Self {
        Self {
            max_count: 1024,
            offset: 0,
            read_status: None,
            originating_address: None,
            period_begin: None,
            period_end: None,
        }
    }
}

impl ListMessagesFilter {
    pub(crate) fn to_app_params(&self) -> Result<Bytes, MapError> {
        Ok(Bytes::from(params::list_messages_params(
            self.max_count,
            self.offset,
            self.read_status.map(ReadStatus::to_wire),
            self.originating_address.as_deref(),
            self.period_begin.as_deref(),
            self.period_end.as_deref(),
        )?))
    }
}

/// One `<msg>` entry in a `MAP-msg-listing` response; absent attributes default to zero/false/empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEntry {
    /// XML attribute names present on the raw listing element; values are not retained here.
    pub attribute_names: Vec<String>,
    /// Opaque device-assigned message handle (hex string).
    pub handle: String,
    /// Opaque device-assigned conversation identifier; empty when unavailable.
    pub conversation_id: String,
    /// Human-readable conversation title; empty when unavailable.
    pub conversation_name: String,
    /// MAP 1.1 direction attribute; empty when unavailable.
    pub direction: String,
    /// Message subject or first line of body text.
    pub subject: String,
    /// Timestamp in MAP format (`YYYYMMDDTHHMMSS[±HHMM]`).
    pub datetime: String,
    /// Sender display name; empty if unavailable.
    pub sender_name: String,
    /// Sender address (phone number or email).
    pub sender_addressing: String,
    /// Recipient display name; empty for received messages.
    pub recipient_name: String,
    /// Recipient address; empty for received messages.
    pub recipient_addressing: String,
    /// Message type string (`SMS_GSM`, `SMS_CDMA`, `EMAIL`, `MMS`).
    pub msg_type: String,
    /// Body size in bytes as reported by the device.
    pub size: u32,
    /// Device-reported read state.
    pub read: bool,
    /// `true` for outbound messages from this device; `false` for received.
    pub sent: bool,
}

/// One participant nested inside a MAP conversation-listing entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationParticipant {
    /// Uniform Contact Identifier, normally a phone number or messaging address.
    pub uci: String,
    /// Device-provided display label; empty when unavailable.
    pub display_name: String,
}

/// One `<conversation>` entry returned by MAP's separate conversation listing operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationEntry {
    /// Opaque device-assigned conversation identifier.
    pub id: String,
    /// Device-provided conversation title; empty when unavailable.
    pub name: String,
    /// Participants nested beneath this conversation.
    pub participants: Vec<ConversationParticipant>,
}

/// Privacy-safe wire metadata returned with a MAP listing response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMetadata {
    /// OBEX response code from the successful final packet.
    pub obex_code: u8,
    /// Combined XML body size in bytes.
    pub body_size: usize,
    /// Server-reported listing size, when present.
    pub listing_size: Option<u16>,
    /// Database identifier bytes, when present.
    pub database_identifier: Option<Vec<u8>>,
    /// Conversation-listing version-counter bytes, when present.
    pub conversation_version_counter: Option<Vec<u8>>,
}

/// Parsed rows plus wire-level response metadata for a diagnostic listing request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticListing<T> {
    /// Privacy-safe response metadata.
    pub metadata: ListingMetadata,
    /// Parsed listing rows.
    pub entries: Vec<T>,
}
