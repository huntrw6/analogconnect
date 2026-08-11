//! Privacy-safe hardware probe for optional MAP conversation-listing support.

use map_core::{folders::Folder, messages::ListMessagesFilter};
use std::{collections::HashSet, time::Duration};

fn hex(value: Option<&[u8]>) -> String {
    value.map_or_else(
        || "absent".to_owned(),
        |bytes| bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load(None)?;
    let address = cfg.device.address().parse::<bluer::Address>()?;
    let stream =
        transport::rfcomm::connect(address, cfg.device.map_channel, Duration::from_secs(5)).await?;
    let mut client = map_core::client::MapClient::connect(stream).await?;
    client.set_folder(Folder::Inbox).await?;
    let messages = client
        .list_messages_diagnostic(&ListMessagesFilter {
            max_count: 100,
            offset: 0,
            ..Default::default()
        })
        .await?;
    let with_id = messages
        .entries
        .iter()
        .filter(|entry| !entry.conversation_id.trim().is_empty())
        .count();
    let with_name = messages
        .entries
        .iter()
        .filter(|entry| !entry.conversation_name.trim().is_empty())
        .count();
    let with_direction = messages
        .entries
        .iter()
        .filter(|entry| !entry.direction.trim().is_empty())
        .count();
    println!(
        "MAP_MESSAGE_LISTING=PASS obex=0x{:02x} listing_size={} body_size={} parsed={} conversation_id={} conversation_name={} direction={}",
        messages.metadata.obex_code,
        messages.metadata.listing_size.map_or_else(|| "absent".to_owned(), |v| v.to_string()),
        messages.metadata.body_size,
        messages.entries.len(),
        with_id,
        with_name,
        with_direction,
    );
    if let Some(newest) = messages.entries.first() {
        println!(
            "MAP_NEWEST mas=0 conversation_id={} conversation_name={} direction={} sender_metadata={} recipient_metadata={} raw_xml_attributes={}",
            if newest.conversation_id.trim().is_empty() { "absent" } else { "present" },
            if newest.conversation_name.trim().is_empty() { "absent" } else { "present" },
            if newest.direction.trim().is_empty() { "absent" } else { "present" },
            if newest.sender_addressing.trim().is_empty() { "absent" } else { "present" },
            if newest.recipient_addressing.trim().is_empty() { "absent" } else { "present" },
            newest.attribute_names.join(","),
        );
    }

    let listing = client.list_conversations_diagnostic(100, 0).await?;
    let rows = &listing.entries;
    let groups = rows
        .iter()
        .filter(|row| {
            row.participants
                .iter()
                .map(|participant| participant.uci.trim())
                .filter(|uci| !uci.is_empty())
                .collect::<HashSet<_>>()
                .len()
                > 1
        })
        .count();
    let with_participants = rows
        .iter()
        .filter(|row| {
            row.participants
                .iter()
                .any(|participant| !participant.uci.trim().is_empty())
        })
        .count();
    println!(
        "MAP_CONVERSATION_LISTING=PASS obex=0x{:02x} listing_size={} body_size={} database_identifier={} conversation_version_counter={} conversations={} with_participants={} groups={}",
        listing.metadata.obex_code,
        listing.metadata.listing_size.map_or_else(|| "absent".to_owned(), |v| v.to_string()),
        listing.metadata.body_size,
        hex(listing.metadata.database_identifier.as_deref()),
        hex(listing.metadata.conversation_version_counter.as_deref()),
        rows.len(), with_participants, groups
    );
    Ok(())
}
