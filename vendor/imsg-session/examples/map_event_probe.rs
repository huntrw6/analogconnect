//! Privacy-safe live MAP Event Report diagnostic with a MAP 1.4 MNS SDP record.

use bluer::rfcomm::{Profile, Role};
use futures::StreamExt;
use quick_xml::{events::Event, Reader};
use std::time::Duration;
use uuid::uuid;

const MNS_RECORD: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<record>
 <attribute id="0x0001"><sequence><uuid value="0x1133"/></sequence></attribute>
 <attribute id="0x0004"><sequence><sequence><uuid value="0x0100"/></sequence><sequence><uuid value="0x0003"/><uint8 value="0x11"/></sequence><sequence><uuid value="0x0008"/></sequence></sequence></attribute>
 <attribute id="0x0009"><sequence><sequence><uuid value="0x1134"/><uint16 value="0x0104"/></sequence></sequence></attribute>
 <attribute id="0x0100"><text value="AnalogConnect MAP MNS Diagnostic"/></attribute>
 <attribute id="0x0317"><uint32 value="0x001003df"/></attribute>
</record>"#;

fn inspect(xml: &[u8]) -> anyhow::Result<(String, Vec<String>)> {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Empty(event) | Event::Start(event) if event.name().as_ref() == b"event" => {
                let mut event_type = "unknown".to_owned();
                let mut names = Vec::new();
                for attribute in event.attributes() {
                    let attribute = attribute?;
                    let name = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
                    if name == "type" {
                        let value = String::from_utf8_lossy(attribute.value.as_ref());
                        event_type = match value.as_ref() {
                            "NewMessage"
                            | "ConversationChanged"
                            | "ParticipantPresenceChanged"
                            | "ParticipantChatStateChanged" => value.into_owned(),
                            _ => "other".to_owned(),
                        };
                    }
                    names.push(name);
                }
                return Ok((event_type, names));
            }
            Event::Eof => anyhow::bail!("event element absent"),
            _ => {}
        }
        buf.clear();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load(None)?;
    let session = bluer::Session::new().await?;
    let profile = Profile {
        uuid: uuid!("00001133-0000-1000-8000-00805f9b34fb"),
        name: Some("AnalogConnect MAP MNS Diagnostic".into()),
        channel: Some(17),
        role: Some(Role::Server),
        service_record: Some(MNS_RECORD.into()),
        ..Default::default()
    };
    let mut listener = session.register_profile(profile).await?;
    let address = cfg.device.address().parse::<bluer::Address>()?;
    let stream =
        transport::rfcomm::connect(address, cfg.device.map_channel, Duration::from_secs(5)).await?;
    let mut map = map_core::client::MapClient::connect(stream).await?;
    map.set_notification_registration(true).await?;

    let request = tokio::time::timeout(Duration::from_secs(30), listener.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("iPhone did not open MNS"))?;
    let stream = request.accept()?;
    let mut mns = map_core::mns_server::MnsServer::accept(stream).await?;
    println!("MAP_EVENT_PROBE=READY mns_profile=1.4 mce_features=0x001003df");
    let raw = tokio::time::timeout(Duration::from_secs(180), mns.next_event_raw())
        .await??
        .ok_or_else(|| anyhow::anyhow!("MNS disconnected before event"))?;
    let (event_type, attributes) = inspect(&raw)?;
    println!(
        "MAP_EVENT_PROBE=PASS event_type={} conversation_id={} conversation_name={} participant_uci={} contact_uid={} raw_xml_attributes={}",
        event_type,
        attributes.iter().any(|name| name == "conversation_id"),
        attributes.iter().any(|name| name == "conversation_name"),
        attributes.iter().any(|name| name == "participant_uci"),
        attributes.iter().any(|name| name == "contact_uid"),
        attributes.join(","),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::inspect;

    #[test]
    fn reports_names_without_private_values() -> anyhow::Result<()> {
        let xml = br#"<MAP-event-report version="1.2"><event type="NewMessage" sender_name="private" conversation_id="private-id"/></MAP-event-report>"#;
        let (kind, names) = inspect(xml)?;
        assert_eq!(kind, "NewMessage");
        assert!(names.iter().any(|name| name == "conversation_id"));
        assert!(!format!("{names:?}").contains("private-id"));
        Ok(())
    }
}
