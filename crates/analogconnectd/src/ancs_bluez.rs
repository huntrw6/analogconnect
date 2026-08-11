//! Production BlueZ ANCS GATT bearer.
//!
//! This module owns only the LE/GATT session. It never disconnects the remote device, so a
//! failed ANCS attempt cannot intentionally tear down MAP, PBAP, or HFP Classic bearers.

use std::time::Duration;

use bluer::gatt::remote::Characteristic;
use futures_util::StreamExt as _;
use thiserror::Error;
use tokio::sync::{mpsc, watch};

use crate::ancs_transport::{AncsNotificationMetadata, AncsSupervisor, SupervisorCommand};

const ANCS_SERVICE: &str = "7905f431-b5ce-4e99-a40f-4b1e122d00d0";
const NOTIFICATION_SOURCE: &str = "9fbf120d-6301-42d9-8c58-25e699a21dbd";
const CONTROL_POINT: &str = "69d1d8f3-45e1-49a8-9821-9bbdfdaad9d9";
const DATA_SOURCE: &str = "22eac6e9-24d6-4bb5-be44-b36ace7c7bfb";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Error)]
pub enum BluezAncsError {
    #[error("ANCS configuration is unavailable")]
    Configuration,
    #[error("BlueZ is unavailable")]
    Bluez,
    #[error("configured device is not paired and trusted")]
    DeviceNotTrusted,
    #[error("ANCS GATT operation timed out")]
    Timeout,
    #[error("ANCS service or characteristics are unavailable")]
    Characteristics,
    #[error("ANCS notification stream ended")]
    StreamEnded,
    #[error("ANCS event consumer stopped")]
    ConsumerStopped,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BluezAncsState {
    #[default]
    Disconnected,
    Discovering,
    SubscribingNotificationSource,
    SubscribingDataSource,
    Ready,
    BackingOff,
}

#[derive(Default)]
struct CharacteristicSet {
    notification_source: Option<Characteristic>,
    control_point: Option<Characteristic>,
    data_source: Option<Characteristic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharacteristicRole {
    NotificationSource,
    ControlPoint,
    DataSource,
}

fn characteristic_role(uuid: &str) -> Option<CharacteristicRole> {
    if uuid.eq_ignore_ascii_case(NOTIFICATION_SOURCE) {
        Some(CharacteristicRole::NotificationSource)
    } else if uuid.eq_ignore_ascii_case(CONTROL_POINT) {
        Some(CharacteristicRole::ControlPoint)
    } else if uuid.eq_ignore_ascii_case(DATA_SOURCE) {
        Some(CharacteristicRole::DataSource)
    } else {
        None
    }
}

impl CharacteristicSet {
    fn complete(self) -> Result<(Characteristic, Characteristic, Characteristic), BluezAncsError> {
        Ok((
            self.notification_source
                .ok_or(BluezAncsError::Characteristics)?,
            self.control_point.ok_or(BluezAncsError::Characteristics)?,
            self.data_source.ok_or(BluezAncsError::Characteristics)?,
        ))
    }
}

/// Runs forever until cancellation, reconnecting with the protocol supervisor's bounded backoff.
pub async fn run(
    output: mpsc::Sender<AncsNotificationMetadata>,
    state: watch::Sender<BluezAncsState>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut supervisor = AncsSupervisor::default();
    loop {
        if *shutdown.borrow() {
            return;
        }
        let _ = state.send(BluezAncsState::Discovering);
        match session(&mut supervisor, &output, &state, &mut shutdown).await {
            Ok(()) if *shutdown.borrow() => return,
            Ok(()) => {}
            Err(error) => {
                tracing::warn!(
                    event = "ancs_bearer_retry",
                    reason = %error,
                    "ANCS bearer unavailable; Classic Bluetooth remains untouched"
                );
            }
        }
        {
            let SupervisorCommand::RetryAfterSeconds(seconds) = supervisor.disconnected() else {
                return;
            };
            let _ = state.send(BluezAncsState::BackingOff);
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(seconds)) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() { return; }
                }
            }
        }
    }
}

async fn session(
    supervisor: &mut AncsSupervisor,
    output: &mpsc::Sender<AncsNotificationMetadata>,
    state: &watch::Sender<BluezAncsState>,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), BluezAncsError> {
    let address = tokio::task::spawn_blocking(|| {
        imsg_config::load(None)
            .map(|config| config.device.address().to_owned())
            .map_err(|_| BluezAncsError::Configuration)
    })
    .await
    .map_err(|_| BluezAncsError::Configuration)??;
    let address = address.parse().map_err(|_| BluezAncsError::Configuration)?;
    let session = timeout(bluer::Session::new()).await?;
    let adapter = timeout(session.default_adapter()).await?;
    let device = adapter.device(address).map_err(|_| BluezAncsError::Bluez)?;
    if !timeout(device.is_paired()).await? || !timeout(device.is_trusted()).await? {
        return Err(BluezAncsError::DeviceNotTrusted);
    }
    if !timeout(device.is_connected()).await? {
        timeout(device.connect()).await?;
    }
    let (notification_source, control_point, data_source) = discover(&device).await?;
    let _ = supervisor.connected();

    let _ = state.send(BluezAncsState::SubscribingNotificationSource);
    let notification_events = timeout(notification_source.notify()).await?;
    let _ = supervisor.notification_source_subscribed();
    let _ = state.send(BluezAncsState::SubscribingDataSource);
    let data_events = timeout(data_source.notify()).await?;
    supervisor.data_source_subscribed();
    let _ = state.send(BluezAncsState::Ready);
    futures_util::pin_mut!(notification_events, data_events);

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { return Ok(()); }
            }
            value = notification_events.next() => {
                let value = value.ok_or(BluezAncsError::StreamEnded)?;
                if let Some(SupervisorCommand::WriteControlPoint(request)) =
                    supervisor.notification_source(&value)
                {
                    timeout(control_point.write(&request)).await?;
                }
            }
            value = data_events.next() => {
                let value = value.ok_or(BluezAncsError::StreamEnded)?;
                let (metadata, command) = supervisor.data_source(&value);
                if let Some(metadata) = metadata {
                    output.try_send(metadata).map_err(|error| match error {
                        mpsc::error::TrySendError::Closed(_) => BluezAncsError::ConsumerStopped,
                        mpsc::error::TrySendError::Full(_) => BluezAncsError::Bluez,
                    })?;
                }
                if let Some(SupervisorCommand::WriteControlPoint(request)) = command {
                    timeout(control_point.write(&request)).await?;
                }
            }
        }
    }
}

async fn discover(
    device: &bluer::Device,
) -> Result<(Characteristic, Characteristic, Characteristic), BluezAncsError> {
    let services = timeout(device.services()).await?;
    let mut found = CharacteristicSet::default();
    for service in services {
        let uuid = timeout(service.uuid()).await?.to_string();
        if !uuid.eq_ignore_ascii_case(ANCS_SERVICE) {
            continue;
        }
        for characteristic in timeout(service.characteristics()).await? {
            let uuid = timeout(characteristic.uuid()).await?.to_string();
            match characteristic_role(&uuid) {
                Some(CharacteristicRole::NotificationSource) => {
                    found.notification_source = Some(characteristic);
                }
                Some(CharacteristicRole::ControlPoint) => {
                    found.control_point = Some(characteristic);
                }
                Some(CharacteristicRole::DataSource) => found.data_source = Some(characteristic),
                None => {}
            }
        }
    }
    found.complete()
}

async fn timeout<T>(
    future: impl std::future::Future<Output = bluer::Result<T>>,
) -> Result<T, BluezAncsError> {
    tokio::time::timeout(OPERATION_TIMEOUT, future)
        .await
        .map_err(|_| BluezAncsError::Timeout)?
        .map_err(|_| BluezAncsError::Bluez)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_and_uuid_contract_are_stable() {
        assert_eq!(BluezAncsState::default(), BluezAncsState::Disconnected);
        assert_eq!(ANCS_SERVICE.len(), 36);
        assert_ne!(NOTIFICATION_SOURCE, DATA_SOURCE);
        assert_ne!(CONTROL_POINT, DATA_SOURCE);
        assert_eq!(
            characteristic_role(&NOTIFICATION_SOURCE.to_uppercase()),
            Some(CharacteristicRole::NotificationSource)
        );
        assert_eq!(
            characteristic_role(CONTROL_POINT),
            Some(CharacteristicRole::ControlPoint)
        );
        assert_eq!(
            characteristic_role(DATA_SOURCE),
            Some(CharacteristicRole::DataSource)
        );
        assert_eq!(characteristic_role(ANCS_SERVICE), None);
    }
}
