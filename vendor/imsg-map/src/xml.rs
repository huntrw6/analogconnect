//! MAP XML parser for message listings (`MAP-msg-listing`).

use quick_xml::{events::Event, Reader, XmlVersion};
use thiserror::Error;

use crate::messages::{ConversationEntry, ConversationParticipant, MessageEntry};

/// quick-xml parse, attribute decode, and u32 invalid int.
#[derive(Debug, Error)]
pub enum MessageListingError {
    /// Underlying quick-xml reader error; also covers entity-decoding failures.
    #[error("XML error: {0}")]
    Parse(#[from] quick_xml::Error),
    /// Attribute encoding error from quick-xml.
    #[error("attribute error: {0}")]
    Attr(#[from] quick_xml::events::attributes::AttrError),
    /// Numeric attribute (`size`) could not be parsed as a `u32`.
    #[error("invalid integer attribute: {0}")]
    InvalidInt(#[from] std::num::ParseIntError),
}

/// Parses a `<MAP-msg-listing>` document from raw bytes.
///
/// Returns one [`MessageEntry`] per `<msg>` element in document order. Attributes absent from
/// an element default to their zero/false/empty value.
///
/// # Errors
///
/// Returns [`MessageListingError`] on malformed XML, undecodable attributes, non-UTF-8
/// attribute values, or a non-numeric `size` field.
pub fn parse_message_listing(xml: &[u8]) -> Result<Vec<MessageEntry>, MessageListingError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut messages = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Empty(e) | Event::Start(e) if e.name().as_ref() == b"msg" => {
                let mut handle = String::new();
                let mut conversation_id = String::new();
                let mut conversation_name = String::new();
                let mut direction = String::new();
                let mut subject = String::new();
                let mut datetime = String::new();
                let mut sender_name = String::new();
                let mut sender_addressing = String::new();
                let mut recipient_name = String::new();
                let mut recipient_addressing = String::new();
                let mut msg_type = String::new();
                let mut size = 0u32;
                let mut read = false;
                let mut sent = false;
                let mut attribute_names = Vec::new();
                for attr in e.attributes() {
                    let a = attr?;
                    attribute_names.push(String::from_utf8_lossy(a.key.as_ref()).into_owned());
                    let val = a.normalized_value(XmlVersion::Implicit1_0)?.into_owned();
                    match a.key.as_ref() {
                        b"handle" => handle = val,
                        b"conversation_id" => conversation_id = val,
                        b"conversation_name" => conversation_name = val,
                        b"direction" => direction = val,
                        b"subject" => subject = val,
                        b"datetime" => datetime = val,
                        b"sender_name" => sender_name = val,
                        b"sender_addressing" => sender_addressing = val,
                        b"recipient_name" => recipient_name = val,
                        b"recipient_addressing" => recipient_addressing = val,
                        b"type" => msg_type = val,
                        b"size" => size = val.parse()?,
                        b"read" => read = val == "yes",
                        b"sent" => sent = val == "yes",
                        _ => {}
                    }
                }
                messages.push(MessageEntry {
                    attribute_names,
                    handle,
                    conversation_id,
                    conversation_name,
                    direction,
                    subject,
                    datetime,
                    sender_name,
                    sender_addressing,
                    recipient_name,
                    recipient_addressing,
                    msg_type,
                    size,
                    read,
                    sent,
                });
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(messages)
}

/// Parses a `MAP-convo-listing` document, preserving only identity, title, and participant UCI.
///
/// # Errors
///
/// Returns [`MessageListingError`] for malformed XML or undecodable attributes.
pub fn parse_conversation_listing(
    xml: &[u8],
) -> Result<Vec<ConversationEntry>, MessageListingError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut conversations = Vec::new();
    let mut current: Option<ConversationEntry> = None;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == b"conversation" => {
                let mut id = String::new();
                let mut name = String::new();
                for attr in e.attributes() {
                    let a = attr?;
                    let val = a.normalized_value(XmlVersion::Implicit1_0)?.into_owned();
                    match a.key.as_ref() {
                        b"id" => id = val,
                        b"name" => name = val,
                        _ => {}
                    }
                }
                current = Some(ConversationEntry {
                    id,
                    name,
                    participants: Vec::new(),
                });
            }
            Event::Empty(e) | Event::Start(e) if e.name().as_ref() == b"convocontact" => {
                if let Some(conversation) = current.as_mut() {
                    let mut uci = String::new();
                    let mut display_name = String::new();
                    for attr in e.attributes() {
                        let a = attr?;
                        let val = a.normalized_value(XmlVersion::Implicit1_0)?.into_owned();
                        match a.key.as_ref() {
                            b"x_bt_uci" => uci = val,
                            b"display_name" | b"name" if display_name.is_empty() => {
                                display_name = val;
                            }
                            _ => {}
                        }
                    }
                    conversation
                        .participants
                        .push(ConversationParticipant { uci, display_name });
                }
            }
            Event::End(e) if e.name().as_ref() == b"conversation" => {
                if let Some(conversation) = current.take() {
                    conversations.push(conversation);
                }
            }
            Event::Empty(e) if e.name().as_ref() == b"conversation" => {
                let mut id = String::new();
                let mut name = String::new();
                for attr in e.attributes() {
                    let a = attr?;
                    let val = a.normalized_value(XmlVersion::Implicit1_0)?.into_owned();
                    match a.key.as_ref() {
                        b"id" => id = val,
                        b"name" => name = val,
                        _ => {}
                    }
                }
                conversations.push(ConversationEntry {
                    id,
                    name,
                    participants: Vec::new(),
                });
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(conversations)
}

#[cfg(test)]
mod tests {
    use super::{parse_conversation_listing, parse_message_listing};

    #[test]
    fn preserves_opaque_conversation_identity() -> Result<(), Box<dyn std::error::Error>> {
        let rows = parse_message_listing(
            br#"<?xml version="1.0"?><MAP-msg-listing version="1.1"><msg handle="1" conversation_id="synthetic-conversation" conversation_name="Synthetic" direction="incoming"/></MAP-msg-listing>"#,
        )?;
        let row = rows
            .first()
            .ok_or_else(|| std::io::Error::other("synthetic row missing"))?;
        assert_eq!(row.conversation_id, "synthetic-conversation");
        assert_eq!(row.conversation_name, "Synthetic");
        assert_eq!(row.direction, "incoming");
        Ok(())
    }

    #[test]
    fn parses_conversations_and_nested_participants() -> Result<(), Box<dyn std::error::Error>> {
        let rows = parse_conversation_listing(
            br#"<?xml version="1.0"?><MAP-convo-listing version="1.0"><conversation id="opaque-one" name="Group"><convocontact x_bt_uci="participant-a" display_name="A"/><convocontact x_bt_uci="participant-b"/></conversation></MAP-convo-listing>"#,
        )?;
        assert_eq!(rows.len(), 1);
        let row = rows
            .first()
            .ok_or_else(|| std::io::Error::other("conversation missing"))?;
        assert_eq!(row.id, "opaque-one");
        assert_eq!(row.participants.len(), 2);
        let participant = row
            .participants
            .first()
            .ok_or_else(|| std::io::Error::other("participant missing"))?;
        assert_eq!(participant.uci, "participant-a");
        Ok(())
    }
}
