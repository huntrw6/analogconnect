use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BluetoothConnectionState {
    Disconnected,
    Connecting,
    Connected,
    ServicesResolved,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSyncState {
    Idle,
    Syncing,
    BackingOff,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactSyncState {
    Idle,
    Syncing,
    BackingOff,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HfpControlState {
    Disconnected,
    AclConnected,
    ServicesResolved,
    ProfileRegistered,
    RfcommConnecting,
    RfcommConnected,
    SlcNegotiating,
    SlcReady,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    Idle,
    Incoming,
    Outgoing,
    Active,
    Held,
    Ended,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioTransportState {
    Inactive,
    CodecNegotiating,
    ScoConnecting,
    ScoActive,
    ScoTearingDown,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AndroidClientState {
    Disconnected,
    Connecting,
    Enrolled,
    Authenticated,
    Error,
}

macro_rules! sync_transitions {
    ($type:ty, $machine:literal) => {
        impl $type {
            pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
                let allowed = matches!(
                    (*self, next),
                    (Self::Idle, Self::Syncing | Self::Error)
                        | (Self::Syncing, Self::Idle | Self::BackingOff | Self::Error)
                        | (Self::BackingOff, Self::Syncing | Self::Idle | Self::Error)
                        | (Self::Error, Self::Idle | Self::BackingOff)
                ) || *self == next;

                if allowed {
                    *self = next;
                    Ok(())
                } else {
                    Err(TransitionError {
                        machine: $machine,
                        from: self.name(),
                        to: next.name(),
                    })
                }
            }

            const fn name(self) -> &'static str {
                match self {
                    Self::Idle => "idle",
                    Self::Syncing => "syncing",
                    Self::BackingOff => "backing_off",
                    Self::Error => "error",
                }
            }
        }
    };
}

sync_transitions!(MessageSyncState, "message_sync");
sync_transitions!(ContactSyncState, "contact_sync");

impl AndroidClientState {
    pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
        use AndroidClientState::*;
        let allowed = matches!(
            (*self, next),
            (Disconnected, Connecting)
                | (Connecting, Enrolled | Authenticated | Disconnected | Error)
                | (Enrolled, Authenticated | Disconnected | Error)
                | (Authenticated, Disconnected | Error)
                | (Error, Disconnected | Connecting)
        ) || *self == next;

        if allowed {
            *self = next;
            Ok(())
        } else {
            Err(TransitionError {
                machine: "android_client",
                from: self.name(),
                to: next.name(),
            })
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Enrolled => "enrolled",
            Self::Authenticated => "authenticated",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedError {
    pub code: String,
    pub summary: String,
}

impl RedactedError {
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStatus {
    pub protocol_version: u16,
    pub bluetooth: BluetoothConnectionState,
    pub messages: MessageSyncState,
    pub contacts: ContactSyncState,
    pub hfp_control: HfpControlState,
    pub call: CallState,
    pub audio: AudioTransportState,
    pub android_client: AndroidClientState,
    pub last_error: Option<RedactedError>,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            protocol_version: 1,
            bluetooth: BluetoothConnectionState::Disconnected,
            messages: MessageSyncState::Idle,
            contacts: ContactSyncState::Idle,
            hfp_control: HfpControlState::Disconnected,
            call: CallState::Idle,
            audio: AudioTransportState::Inactive,
            android_client: AndroidClientState::Disconnected,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid {machine} transition from {from} to {to}")]
pub struct TransitionError {
    pub machine: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

macro_rules! transition_method {
    ($type:ty, $machine:literal, { $($from:pat => [$($to:pat),* $(,)?]),* $(,)? }) => {
        impl $type {
            pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
                let allowed = match (*self, next) {
                    $(($from, $($to)|*) => true,)*
                    (current, candidate) if current == candidate => true,
                    _ => false,
                };

                if allowed {
                    *self = next;
                    Ok(())
                } else {
                    Err(TransitionError {
                        machine: $machine,
                        from: self.name(),
                        to: next.name(),
                    })
                }
            }

            const fn name(self) -> &'static str {
                match self {
                    Self::Disconnected => "disconnected",
                    Self::Error => "error",
                    _ => self.non_terminal_name(),
                }
            }
        }
    };
}

impl BluetoothConnectionState {
    const fn non_terminal_name(self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::ServicesResolved => "services_resolved",
            Self::Disconnected => "disconnected",
            Self::Error => "error",
        }
    }
}

transition_method!(BluetoothConnectionState, "bluetooth", {
    BluetoothConnectionState::Disconnected => [BluetoothConnectionState::Connecting],
    BluetoothConnectionState::Connecting => [BluetoothConnectionState::Connected, BluetoothConnectionState::Disconnected, BluetoothConnectionState::Error],
    BluetoothConnectionState::Connected => [BluetoothConnectionState::ServicesResolved, BluetoothConnectionState::Disconnected, BluetoothConnectionState::Error],
    BluetoothConnectionState::ServicesResolved => [BluetoothConnectionState::Disconnected, BluetoothConnectionState::Error],
    BluetoothConnectionState::Error => [BluetoothConnectionState::Disconnected, BluetoothConnectionState::Connecting]
});

impl HfpControlState {
    pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
        use HfpControlState::*;
        let allowed = matches!(
            (*self, next),
            (Disconnected, AclConnected)
                | (AclConnected, ServicesResolved | Disconnected | Error)
                | (ServicesResolved, ProfileRegistered | Disconnected | Error)
                | (ProfileRegistered, RfcommConnecting | Disconnected | Error)
                | (RfcommConnecting, RfcommConnected | Disconnected | Error)
                | (RfcommConnected, SlcNegotiating | Disconnected | Error)
                | (SlcNegotiating, SlcReady | Disconnected | Error)
                | (SlcReady, Disconnected | Error)
                | (Error, Disconnected | AclConnected)
        ) || *self == next;

        if allowed {
            *self = next;
            Ok(())
        } else {
            Err(TransitionError {
                machine: "hfp_control",
                from: self.name(),
                to: next.name(),
            })
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::AclConnected => "acl_connected",
            Self::ServicesResolved => "services_resolved",
            Self::ProfileRegistered => "profile_registered",
            Self::RfcommConnecting => "rfcomm_connecting",
            Self::RfcommConnected => "rfcomm_connected",
            Self::SlcNegotiating => "slc_negotiating",
            Self::SlcReady => "slc_ready",
            Self::Error => "error",
        }
    }
}

impl CallState {
    pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
        use CallState::*;
        let allowed = matches!(
            (*self, next),
            (Idle, Incoming | Outgoing | Error)
                | (Incoming, Active | Ended | Error)
                | (Outgoing, Active | Ended | Error)
                | (Active, Held | Ended | Error)
                | (Held, Active | Ended | Error)
                | (Ended, Idle)
                | (Error, Idle | Ended)
        ) || *self == next;

        if allowed {
            *self = next;
            Ok(())
        } else {
            Err(TransitionError {
                machine: "call",
                from: self.name(),
                to: next.name(),
            })
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Incoming => "incoming",
            Self::Outgoing => "outgoing",
            Self::Active => "active",
            Self::Held => "held",
            Self::Ended => "ended",
            Self::Error => "error",
        }
    }
}

impl AudioTransportState {
    pub fn transition_to(&mut self, next: Self) -> Result<(), TransitionError> {
        use AudioTransportState::*;
        let allowed = matches!(
            (*self, next),
            (Inactive, CodecNegotiating | Error)
                | (CodecNegotiating, ScoConnecting | Inactive | Error)
                | (ScoConnecting, ScoActive | ScoTearingDown | Error)
                | (ScoActive, ScoTearingDown | Error)
                | (ScoTearingDown, Inactive | Error)
                | (Error, Inactive)
        ) || *self == next;

        if allowed {
            *self = next;
            Ok(())
        } else {
            Err(TransitionError {
                machine: "audio_transport",
                from: self.name(),
                to: next.name(),
            })
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::CodecNegotiating => "codec_negotiating",
            Self::ScoConnecting => "sco_connecting",
            Self::ScoActive => "sco_active",
            Self::ScoTearingDown => "sco_tearing_down",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluetooth_requires_ordered_connection_progress() {
        let mut state = BluetoothConnectionState::Disconnected;
        assert!(
            state
                .transition_to(BluetoothConnectionState::Connecting)
                .is_ok()
        );
        assert!(
            state
                .transition_to(BluetoothConnectionState::Connected)
                .is_ok()
        );
        assert!(
            state
                .transition_to(BluetoothConnectionState::ServicesResolved)
                .is_ok()
        );
    }

    #[test]
    fn bluetooth_rejects_skipping_to_services_resolved() {
        let mut state = BluetoothConnectionState::Disconnected;
        let error = state
            .transition_to(BluetoothConnectionState::ServicesResolved)
            .expect_err("transition must be rejected");
        assert_eq!(error.machine, "bluetooth");
        assert_eq!(state, BluetoothConnectionState::Disconnected);
    }

    #[test]
    fn hfp_slc_follows_control_lifecycle() {
        let mut state = HfpControlState::Disconnected;
        for next in [
            HfpControlState::AclConnected,
            HfpControlState::ServicesResolved,
            HfpControlState::ProfileRegistered,
            HfpControlState::RfcommConnecting,
            HfpControlState::RfcommConnected,
            HfpControlState::SlcNegotiating,
            HfpControlState::SlcReady,
        ] {
            state.transition_to(next).expect("valid HFP transition");
        }
    }

    #[test]
    fn call_cannot_become_active_from_idle() {
        let mut state = CallState::Idle;
        assert!(state.transition_to(CallState::Active).is_err());
    }

    #[test]
    fn incoming_call_can_be_answered_and_ended() {
        let mut state = CallState::Idle;
        state.transition_to(CallState::Incoming).unwrap();
        state.transition_to(CallState::Active).unwrap();
        state.transition_to(CallState::Ended).unwrap();
        state.transition_to(CallState::Idle).unwrap();
    }

    #[test]
    fn sco_lifecycle_returns_to_inactive() {
        let mut state = AudioTransportState::Inactive;
        state
            .transition_to(AudioTransportState::CodecNegotiating)
            .unwrap();
        state
            .transition_to(AudioTransportState::ScoConnecting)
            .unwrap();
        state.transition_to(AudioTransportState::ScoActive).unwrap();
        state
            .transition_to(AudioTransportState::ScoTearingDown)
            .unwrap();
        state.transition_to(AudioTransportState::Inactive).unwrap();
    }

    #[test]
    fn default_status_exposes_no_private_data() {
        let status = SystemStatus::default();
        assert_eq!(status.protocol_version, 1);
        assert!(status.last_error.is_none());
    }

    #[test]
    fn sync_states_back_off_before_retrying() {
        let mut messages = MessageSyncState::Idle;
        messages.transition_to(MessageSyncState::Syncing).unwrap();
        messages
            .transition_to(MessageSyncState::BackingOff)
            .unwrap();
        messages.transition_to(MessageSyncState::Syncing).unwrap();

        let mut contacts = ContactSyncState::Idle;
        assert!(
            contacts
                .transition_to(ContactSyncState::BackingOff)
                .is_err()
        );
    }

    #[test]
    fn android_client_requires_connection_before_authentication() {
        let mut state = AndroidClientState::Disconnected;
        assert!(
            state
                .transition_to(AndroidClientState::Authenticated)
                .is_err()
        );
        state.transition_to(AndroidClientState::Connecting).unwrap();
        state
            .transition_to(AndroidClientState::Authenticated)
            .unwrap();
    }
}
