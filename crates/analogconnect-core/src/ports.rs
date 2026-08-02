use crate::{
    AudioTransportState, BluetoothConnectionState, CallState, Contact, ContactSyncState,
    HfpControlState, MessageSyncState,
};

/// Read-only Bluetooth lifecycle boundary. Implementations must not expose device addresses.
pub trait BluetoothBackend: Send + Sync {
    fn connection_state(&self) -> BluetoothConnectionState;
}

/// MAP boundary. Message bodies and recipients must never be written to routine logs.
pub trait MapBackend: Send + Sync {
    fn sync_state(&self) -> MessageSyncState;
}

/// PBAP boundary. Contact names and phone numbers must never be written to routine logs.
pub trait PbapBackend: Send + Sync {
    fn sync_state(&self) -> ContactSyncState;
}

/// Pulls a complete phonebook. Implementations must never log returned payloads.
pub trait ContactSource: Send + Sync {
    type Error;

    fn pull_all(&self) -> Result<Vec<Contact>, Self::Error>;
}

/// HFP control boundary. Remote identifiers stay inside the adapter implementation.
pub trait HfpBackend: Send + Sync {
    fn control_state(&self) -> HfpControlState;
    fn call_state(&self) -> CallState;
}

/// PipeWire/SCO boundary. Implementations stream audio and must never record it.
pub trait AudioBackend: Send + Sync {
    fn transport_state(&self) -> AudioTransportState;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBluetooth;

    impl BluetoothBackend for MockBluetooth {
        fn connection_state(&self) -> BluetoothConnectionState {
            BluetoothConnectionState::ServicesResolved
        }
    }

    #[test]
    fn bluetooth_boundary_is_mockable_without_hardware() {
        let backend: Box<dyn BluetoothBackend> = Box::new(MockBluetooth);
        assert_eq!(
            backend.connection_state(),
            BluetoothConnectionState::ServicesResolved
        );
    }
}
