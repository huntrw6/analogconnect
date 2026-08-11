//! Privacy-safe Apple Notification Center Service diagnostic.
//!
//! Attribute values exist only in memory and are rendered as structural metadata. Message body
//! is never requested.

use std::time::Duration;
use tokio::io::AsyncReadExt;

const ANCS_SERVICE: &str = "7905f431-b5ce-4e99-a40f-4b1e122d00d0";
const NOTIFICATION_SOURCE: &str = "9fbf120d-6301-42d9-8c58-25e699a21dbd";
const CONTROL_POINT: &str = "69d1d8f3-45e1-49a8-9821-9bbdfdaad9d9";
const DATA_SOURCE: &str = "22eac6e9-24d6-4bb5-be44-b36ace7c7bfb";
const MESSAGES_APP: &str = "com.apple.MobileSMS";
const ATTRIBUTE_IDS: [u8; 7] = [0, 1, 2, 4, 5, 6, 7];

fn shape(value: &[u8]) -> String {
    let text = String::from_utf8_lossy(value);
    let words = text.split_whitespace().count();
    let punctuation = text
        .chars()
        .filter(|ch| !ch.is_alphanumeric() && !ch.is_whitespace())
        .count();
    format!(
        "present={} bytes={} words={} punctuation={}",
        !value.is_empty(),
        value.len(),
        words,
        punctuation
    )
}

fn request(uid: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(uid);
    bytes.push(0);
    for id in [1_u8, 2] {
        bytes.push(id);
        bytes.extend_from_slice(&256_u16.to_le_bytes());
    }
    bytes.extend_from_slice(&[4, 5, 6, 7]);
    bytes
}

fn parse_attributes(buf: &[u8]) -> Option<Vec<&[u8]>> {
    if buf.len() < 5 || buf[0] != 0 {
        return None;
    }
    let mut offset = 5usize;
    let mut values = Vec::with_capacity(ATTRIBUTE_IDS.len());
    for expected in ATTRIBUTE_IDS {
        if buf.get(offset).copied()? != expected {
            return None;
        }
        let len_bytes = [*buf.get(offset + 1)?, *buf.get(offset + 2)?];
        let len = usize::from(u16::from_le_bytes(len_bytes));
        let start = offset.checked_add(3)?;
        let end = start.checked_add(len)?;
        values.push(buf.get(start..end)?);
        offset = end;
    }
    Some(values)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load(None)?;
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    let address = cfg.device.address().parse::<bluer::Address>()?;
    let device = adapter.device(address)?;

    let mut notification_source = None;
    let mut control_point = None;
    let mut data_source = None;
    for service in device.services().await? {
        if !service
            .uuid()
            .await?
            .to_string()
            .eq_ignore_ascii_case(ANCS_SERVICE)
        {
            continue;
        }
        for characteristic in service.characteristics().await? {
            let uuid = characteristic.uuid().await?.to_string();
            if uuid.eq_ignore_ascii_case(NOTIFICATION_SOURCE) {
                notification_source = Some(characteristic);
            } else if uuid.eq_ignore_ascii_case(CONTROL_POINT) {
                control_point = Some(characteristic);
            } else if uuid.eq_ignore_ascii_case(DATA_SOURCE) {
                data_source = Some(characteristic);
            }
        }
    }
    let notification_source = notification_source
        .ok_or_else(|| anyhow::anyhow!("ANCS notification source unavailable"))?;
    let control_point =
        control_point.ok_or_else(|| anyhow::anyhow!("ANCS control point unavailable"))?;
    let data_source = data_source.ok_or_else(|| anyhow::anyhow!("ANCS data source unavailable"))?;

    let mut data_events = data_source.notify_io().await?;
    let mut notification_events = notification_source.notify_io().await?;
    println!("ANCS_PROBE=READY timeout_seconds=180");

    let run = async {
        let mut response = Vec::new();
        let mut requested = 0usize;
        let mut events = 0usize;
        let mut request_pending = false;
        let mut notification_buf = [0_u8; 64];
        let mut data_buf = [0_u8; 512];
        while requested < 3 {
            tokio::select! {
                count = notification_events.read(&mut notification_buf) => {
                    let count = count?;
                    let event = &notification_buf[..count];
                    if event.len() == 8 {
                        events += 1;
                        println!(
                            "ANCS_EVENT={} kind={} category={} request_pending={}",
                            events,
                            match event[0] { 0 => "added", 1 => "modified", 2 => "removed", _ => "unknown" },
                            event[2],
                            request_pending,
                        );
                    }
                    if event.len() == 8 && event[0] <= 1 && !request_pending {
                        control_point.write(&request(&event[4..8])).await?;
                        response.clear();
                        request_pending = true;
                    }
                }
                count = data_events.read(&mut data_buf) => {
                    let count = count?;
                    let fragment = &data_buf[..count];
                    response.extend_from_slice(&fragment);
                    if let Some(values) = parse_attributes(&response) {
                        let is_messages = values[0] == MESSAGES_APP.as_bytes();
                        if is_messages {
                            requested += 1;
                            println!(
                                "notification={} app=messages title=[redacted; {}] subtitle=[redacted; {}] message_size={} date={} positive_action={} negative_action={}",
                                requested,
                                shape(values[1]),
                                shape(values[2]),
                                shape(values[3]),
                                shape(values[4]),
                                shape(values[5]),
                                shape(values[6]),
                            );
                        } else {
                            println!("ANCS_ATTRIBUTE_RESULT app=non_messages values=redacted");
                        }
                        response.clear();
                        request_pending = false;
                    }
                }
            }
        }
        anyhow::Ok(())
    };
    tokio::time::timeout(Duration::from_secs(180), run)
        .await
        .map_err(|_| anyhow::anyhow!("ANCS probe timed out"))??;
    println!("ANCS_PROBE=PASS messages_notifications=3");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_attributes, request, shape};

    #[test]
    fn request_omits_message_body_attribute() {
        let bytes = request(&[1, 2, 3, 4]);
        assert_eq!(bytes, [0, 1, 2, 3, 4, 0, 1, 0, 1, 2, 0, 1, 4, 5, 6, 7]);
    }

    #[test]
    fn parser_and_shape_do_not_return_values() {
        let mut response = vec![0, 1, 2, 3, 4];
        for (id, value) in [(0, b"app".as_slice()), (1, b"private title"), (2, b"group")]
            .into_iter()
            .chain([(4, b"12".as_slice()), (5, b"date"), (6, b"yes"), (7, b"no")])
        {
            response.push(id);
            response.extend_from_slice(&(value.len() as u16).to_le_bytes());
            response.extend_from_slice(value);
        }
        let values = parse_attributes(&response).expect("complete response");
        let rendered = shape(values[1]);
        assert!(!rendered.contains("private"));
        assert!(rendered.contains("words=2"));
    }
}
