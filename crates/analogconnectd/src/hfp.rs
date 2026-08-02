use std::sync::Mutex;

use analogconnect_core::{CallCommand, CallState, Gain};
use thiserror::Error;

pub trait HfpCommandBackend: Send + Sync {
    type Error;

    fn execute(&self, command: &CallCommand) -> Result<(), Self::Error>;
}

pub trait AtTransport: Send + Sync {
    type Error;

    /// Sends one complete AT command. Implementations must not log command text.
    fn send(&self, command: &str) -> Result<(), Self::Error>;
}

#[derive(Debug, Error)]
pub enum AtBackendError {
    #[error("HFP transport failed")]
    Transport,
    #[error("HFP microphone state lock was poisoned")]
    LockPoisoned,
}

struct MicrophoneState {
    gain: Gain,
    muted: bool,
}

/// Encodes validated call commands as HFP 1.8 AT operations.
pub struct AtCommandBackend<T> {
    transport: T,
    microphone: Mutex<MicrophoneState>,
}

impl<T> AtCommandBackend<T> {
    pub fn new(transport: T) -> Result<Self, analogconnect_core::HfpArgumentError> {
        Ok(Self {
            transport,
            microphone: Mutex::new(MicrophoneState {
                gain: Gain::new(15)?,
                muted: false,
            }),
        })
    }
}

impl<T> HfpCommandBackend for AtCommandBackend<T>
where
    T: AtTransport,
{
    type Error = AtBackendError;

    fn execute(&self, command: &CallCommand) -> Result<(), Self::Error> {
        match command {
            CallCommand::Answer => self.send("ATA"),
            CallCommand::Reject | CallCommand::HangUp => self.send("AT+CHUP"),
            CallCommand::Dial(target) => self.send(&format!("ATD{};", target.as_str())),
            CallCommand::SendDtmf(tone) => self.send(&format!("AT+VTS={}", tone.value())),
            CallCommand::SetSpeakerGain(gain) => self.send(&format!("AT+VGS={}", gain.value())),
            CallCommand::SetMicrophoneGain(gain) => {
                let mut microphone = self
                    .microphone
                    .lock()
                    .map_err(|_| AtBackendError::LockPoisoned)?;
                if !microphone.muted {
                    self.send(&format!("AT+VGM={}", gain.value()))?;
                }
                microphone.gain = *gain;
                Ok(())
            }
            CallCommand::SetMicrophoneMuted(muted) => {
                let mut microphone = self
                    .microphone
                    .lock()
                    .map_err(|_| AtBackendError::LockPoisoned)?;
                let gain = if *muted { 0 } else { microphone.gain.value() };
                self.send(&format!("AT+VGM={gain}"))?;
                microphone.muted = *muted;
                Ok(())
            }
        }
    }
}

impl<T> AtCommandBackend<T>
where
    T: AtTransport,
{
    fn send(&self, command: &str) -> Result<(), AtBackendError> {
        self.transport
            .send(command)
            .map_err(|_| AtBackendError::Transport)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CallControlError {
    #[error("call command is not valid in the current state")]
    InvalidState,
    #[error("HFP backend rejected the command")]
    Backend,
    #[error("call-control state lock was poisoned")]
    LockPoisoned,
}

/// Serializes HFP mutations and changes local state only after backend success.
pub struct CallController<B> {
    backend: B,
    state: Mutex<CallState>,
}

impl<B> CallController<B>
where
    B: HfpCommandBackend,
{
    #[must_use]
    pub fn new(backend: B, initial_state: CallState) -> Self {
        Self {
            backend,
            state: Mutex::new(initial_state),
        }
    }

    pub fn state(&self) -> Result<CallState, CallControlError> {
        self.state
            .lock()
            .map(|state| *state)
            .map_err(|_| CallControlError::LockPoisoned)
    }

    pub fn execute(&self, command: &CallCommand) -> Result<CallState, CallControlError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CallControlError::LockPoisoned)?;
        let next = next_state(*state, command).ok_or(CallControlError::InvalidState)?;

        self.backend
            .execute(command)
            .map_err(|_| CallControlError::Backend)?;
        if let Some(next) = next {
            state
                .transition_to(next)
                .map_err(|_| CallControlError::InvalidState)?;
        }
        Ok(*state)
    }
}

fn next_state(state: CallState, command: &CallCommand) -> Option<Option<CallState>> {
    use CallCommand::{
        Answer, Dial, HangUp, Reject, SendDtmf, SetMicrophoneGain, SetMicrophoneMuted,
        SetSpeakerGain,
    };
    use CallState::{Active, Ended, Held, Idle, Incoming, Outgoing};

    match (state, command) {
        (Incoming, Answer) => Some(Some(Active)),
        (Incoming, Reject) => Some(Some(Ended)),
        (Incoming | Outgoing | Active | Held, HangUp) => Some(Some(Ended)),
        (Idle, Dial(_)) => Some(Some(Outgoing)),
        (Active, SendDtmf(_)) => Some(None),
        (Active | Held, SetMicrophoneMuted(_)) => Some(None),
        (Active | Held, SetSpeakerGain(_) | SetMicrophoneGain(_)) => Some(None),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analogconnect_core::{DialTarget, DtmfTone};
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    struct MockBackend {
        calls: AtomicUsize,
        succeeds: bool,
    }

    impl HfpCommandBackend for MockBackend {
        type Error = ();

        fn execute(&self, _command: &CallCommand) -> Result<(), Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.succeeds.then_some(()).ok_or(())
        }
    }

    fn controller(state: CallState, succeeds: bool) -> CallController<MockBackend> {
        CallController::new(
            MockBackend {
                calls: AtomicUsize::new(0),
                succeeds,
            },
            state,
        )
    }

    #[test]
    fn answer_requires_incoming_and_transitions_after_success() {
        let idle = controller(CallState::Idle, true);
        assert_eq!(
            idle.execute(&CallCommand::Answer),
            Err(CallControlError::InvalidState)
        );

        let incoming = controller(CallState::Incoming, true);
        assert_eq!(
            incoming.execute(&CallCommand::Answer),
            Ok(CallState::Active)
        );
    }

    #[test]
    fn backend_failure_preserves_call_state() {
        let incoming = controller(CallState::Incoming, false);
        assert_eq!(
            incoming.execute(&CallCommand::Reject),
            Err(CallControlError::Backend)
        );
        assert_eq!(incoming.state().unwrap(), CallState::Incoming);
    }

    #[test]
    fn dial_and_dtmf_are_state_checked() {
        let target = DialTarget::parse(&["+1", "202", "555", "0101"].concat()).unwrap();
        let idle = controller(CallState::Idle, true);
        assert_eq!(
            idle.execute(&CallCommand::Dial(target)),
            Ok(CallState::Outgoing)
        );

        let active = controller(CallState::Active, true);
        let tone = DtmfTone::parse('5').unwrap();
        assert_eq!(
            active.execute(&CallCommand::SendDtmf(tone)),
            Ok(CallState::Active)
        );
    }

    #[test]
    fn hangup_is_rejected_when_idle() {
        let idle = controller(CallState::Idle, true);
        assert_eq!(
            idle.execute(&CallCommand::HangUp),
            Err(CallControlError::InvalidState)
        );
    }

    #[derive(Default)]
    struct MockTransport {
        commands: Mutex<Vec<String>>,
    }

    impl AtTransport for MockTransport {
        type Error = ();

        fn send(&self, command: &str) -> Result<(), Self::Error> {
            self.commands.lock().unwrap().push(command.to_owned());
            Ok(())
        }
    }

    #[test]
    fn at_backend_encodes_validated_commands() {
        let backend = AtCommandBackend::new(MockTransport::default()).unwrap();
        backend.execute(&CallCommand::Answer).unwrap();
        backend.execute(&CallCommand::HangUp).unwrap();
        let target_text = ["+1", "202", "555", "0101"].concat();
        let target = DialTarget::parse(&target_text).unwrap();
        backend.execute(&CallCommand::Dial(target)).unwrap();

        let commands = backend.transport.commands.lock().unwrap();
        assert_eq!(commands[0], "ATA");
        assert_eq!(commands[1], "AT+CHUP");
        assert_eq!(commands[2], format!("ATD{target_text};"));
    }

    #[test]
    fn mute_restores_last_microphone_gain() {
        let backend = AtCommandBackend::new(MockTransport::default()).unwrap();
        backend
            .execute(&CallCommand::SetMicrophoneGain(Gain::new(7).unwrap()))
            .unwrap();
        backend
            .execute(&CallCommand::SetMicrophoneMuted(true))
            .unwrap();
        backend
            .execute(&CallCommand::SetMicrophoneGain(Gain::new(9).unwrap()))
            .unwrap();
        backend
            .execute(&CallCommand::SetMicrophoneMuted(false))
            .unwrap();

        let commands = backend.transport.commands.lock().unwrap();
        assert_eq!(commands.as_slice(), ["AT+VGM=7", "AT+VGM=0", "AT+VGM=9"]);
    }
}
