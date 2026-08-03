use std::{path::PathBuf, sync::Mutex};

use analogconnect_core::{CallCommand, CallState, Gain, HfpControlState};
use thiserror::Error;

use crate::process::run_bounded;

pub trait HfpCommandBackend: Send + Sync {
    type Error;

    fn execute(&self, command: &CallCommand) -> Result<(), Self::Error>;
}

pub trait HfpStateBackend: Send + Sync {
    type Error;

    fn snapshot(&self) -> Result<HfpStatusSnapshot, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HfpStatusSnapshot {
    pub control: HfpControlState,
    pub call: CallState,
}

pub trait AtTransport: Send + Sync {
    type Error;

    /// Sends one complete AT command. Implementations must not log command text.
    fn send(&self, command: &str) -> Result<(), Self::Error>;
}

const TELEPHONY_SERVICE: &str = "org.pipewire.Telephony";
const TELEPHONY_ROOT: &str = "/org/pipewire/Telephony";
const AUDIO_GATEWAY_INTERFACE: &str = "org.pipewire.Telephony.AudioGateway1";
const CALL_INTERFACE: &str = "org.pipewire.Telephony.Call1";
const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const MAX_HELPER_OUTPUT_BYTES: usize = 1024 * 1024;

pub trait DbusCommandRunner: Send + Sync {
    type Error;

    /// Runs busctl without logging arguments or output. Output may contain only
    /// WirePlumber's numeric telephony object paths and must not be persisted.
    fn run(&self, arguments: &[&str]) -> Result<String, Self::Error>;
}

pub struct BusctlRunner {
    executable: PathBuf,
}

impl BusctlRunner {
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for BusctlRunner {
    fn default() -> Self {
        Self::new("busctl")
    }
}

impl DbusCommandRunner for BusctlRunner {
    type Error = ();

    fn run(&self, arguments: &[&str]) -> Result<String, Self::Error> {
        let output = run_bounded(
            &self.executable,
            arguments,
            HELPER_TIMEOUT,
            MAX_HELPER_OUTPUT_BYTES,
        )
        .map_err(|_| ())?;
        String::from_utf8(output).map_err(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WirePlumberBackendError {
    #[error("WirePlumber telephony service is unavailable")]
    Unavailable,
    #[error("WirePlumber telephony object state is ambiguous")]
    Ambiguous,
    #[error("call command is unsupported by the WirePlumber telephony API")]
    Unsupported,
    #[error("WirePlumber rejected the call command")]
    Rejected,
}

/// Controls the native HFP backend through PipeWire's supported Telephony D-Bus
/// service, preserving WirePlumber's ownership of the RFCOMM connection.
pub struct WirePlumberBackend<R> {
    runner: R,
}

impl<R> WirePlumberBackend<R> {
    #[must_use]
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R> WirePlumberBackend<R>
where
    R: DbusCommandRunner,
{
    fn paths(&self) -> Result<TelephonyPaths, WirePlumberBackendError> {
        let output = self
            .runner
            .run(&["--user", "tree", TELEPHONY_SERVICE])
            .map_err(|_| WirePlumberBackendError::Unavailable)?;
        TelephonyPaths::parse(&output)
    }

    fn call(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        values: &[&str],
    ) -> Result<(), WirePlumberBackendError> {
        let mut arguments = vec!["--user", "call", TELEPHONY_SERVICE, path, interface, method];
        arguments.extend_from_slice(values);
        self.runner
            .run(&arguments)
            .map(|_| ())
            .map_err(|_| WirePlumberBackendError::Rejected)
    }

    fn call_state(&self, path: &str) -> Result<LiveCallState, WirePlumberBackendError> {
        let output = self
            .runner
            .run(&[
                "--user",
                "get-property",
                TELEPHONY_SERVICE,
                path,
                CALL_INTERFACE,
                "State",
            ])
            .map_err(|_| WirePlumberBackendError::Unavailable)?;
        LiveCallState::parse(&output).ok_or(WirePlumberBackendError::Ambiguous)
    }
}

impl<R> HfpCommandBackend for WirePlumberBackend<R>
where
    R: DbusCommandRunner,
{
    type Error = WirePlumberBackendError;

    fn execute(&self, command: &CallCommand) -> Result<(), Self::Error> {
        let paths = self.paths()?;
        match command {
            CallCommand::Answer => {
                let call = paths.only_call()?;
                if self.call_state(call)? != LiveCallState::Incoming {
                    return Err(WirePlumberBackendError::Rejected);
                }
                self.call(call, CALL_INTERFACE, "Answer", &[])
            }
            CallCommand::Reject | CallCommand::HangUp => {
                if paths.calls.is_empty() {
                    return Err(WirePlumberBackendError::Rejected);
                }
                self.call(&paths.gateway, AUDIO_GATEWAY_INTERFACE, "HangupAll", &[])
            }
            CallCommand::Dial(target) => {
                if !paths.calls.is_empty() {
                    return Err(WirePlumberBackendError::Rejected);
                }
                self.call(
                    &paths.gateway,
                    AUDIO_GATEWAY_INTERFACE,
                    "Dial",
                    &["s", target.as_str()],
                )
            }
            CallCommand::SendDtmf(tone) => {
                let call = paths.only_call()?;
                if self.call_state(call)? != LiveCallState::Active {
                    return Err(WirePlumberBackendError::Rejected);
                }
                let value = tone.value().to_string();
                self.call(
                    &paths.gateway,
                    AUDIO_GATEWAY_INTERFACE,
                    "SendTones",
                    &["s", &value],
                )
            }
            CallCommand::SetMicrophoneMuted(_)
            | CallCommand::SetSpeakerGain(_)
            | CallCommand::SetMicrophoneGain(_) => Err(WirePlumberBackendError::Unsupported),
        }
    }
}

impl<R> HfpStateBackend for WirePlumberBackend<R>
where
    R: DbusCommandRunner,
{
    type Error = WirePlumberBackendError;

    fn snapshot(&self) -> Result<HfpStatusSnapshot, Self::Error> {
        let paths = self.paths()?;
        let mut states = Vec::with_capacity(paths.calls.len());
        for call in &paths.calls {
            states.push(self.call_state(call)?);
        }
        Ok(HfpStatusSnapshot {
            control: HfpControlState::SlcReady,
            call: aggregate_call_state(&states),
        })
    }
}

fn aggregate_call_state(states: &[LiveCallState]) -> CallState {
    if states.contains(&LiveCallState::Active) {
        CallState::Active
    } else if states.contains(&LiveCallState::Incoming) {
        CallState::Incoming
    } else if states
        .iter()
        .any(|state| matches!(state, LiveCallState::Dialing | LiveCallState::Alerting))
    {
        CallState::Outgoing
    } else if states.contains(&LiveCallState::Held) {
        CallState::Held
    } else if states.contains(&LiveCallState::Disconnected) {
        CallState::Ended
    } else {
        CallState::Idle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveCallState {
    Incoming,
    Dialing,
    Alerting,
    Active,
    Held,
    Disconnected,
}

impl LiveCallState {
    fn parse(output: &str) -> Option<Self> {
        match output.trim() {
            "s \"incoming\"" => Some(Self::Incoming),
            "s \"dialing\"" => Some(Self::Dialing),
            "s \"alerting\"" => Some(Self::Alerting),
            "s \"active\"" => Some(Self::Active),
            "s \"held\"" => Some(Self::Held),
            "s \"disconnected\"" => Some(Self::Disconnected),
            _ => None,
        }
    }
}

struct TelephonyPaths {
    gateway: String,
    calls: Vec<String>,
}

impl TelephonyPaths {
    fn parse(output: &str) -> Result<Self, WirePlumberBackendError> {
        let mut gateways = Vec::new();
        let mut calls = Vec::new();
        for token in output.split_whitespace() {
            let Some(path) = token.strip_prefix(TELEPHONY_ROOT) else {
                continue;
            };
            if let Some(id) = path.strip_prefix("/ag")
                && !id.is_empty()
                && id.chars().all(|character| character.is_ascii_digit())
            {
                gateways.push(format!("{TELEPHONY_ROOT}{path}"));
                continue;
            }
            if let Some((gateway_id, call_id)) = path
                .strip_prefix("/ag")
                .and_then(|rest| rest.split_once("/call"))
                && !gateway_id.is_empty()
                && !call_id.is_empty()
                && gateway_id
                    .chars()
                    .all(|character| character.is_ascii_digit())
                && call_id.chars().all(|character| character.is_ascii_digit())
            {
                calls.push(format!("{TELEPHONY_ROOT}{path}"));
            }
        }
        gateways.sort();
        gateways.dedup();
        calls.sort();
        calls.dedup();
        if gateways.is_empty() {
            return Err(WirePlumberBackendError::Unavailable);
        }
        if gateways.len() != 1 {
            return Err(WirePlumberBackendError::Ambiguous);
        }
        let gateway = gateways.remove(0);
        calls.retain(|call| call.starts_with(&format!("{gateway}/call")));
        Ok(Self { gateway, calls })
    }

    fn only_call(&self) -> Result<&str, WirePlumberBackendError> {
        if self.calls.len() == 1 {
            Ok(&self.calls[0])
        } else {
            Err(WirePlumberBackendError::Ambiguous)
        }
    }
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

    struct MockDbusRunner {
        tree: String,
        state: String,
        commands: Mutex<Vec<Vec<String>>>,
        succeeds: bool,
    }

    impl DbusCommandRunner for MockDbusRunner {
        type Error = ();

        fn run(&self, arguments: &[&str]) -> Result<String, Self::Error> {
            self.commands.lock().unwrap().push(
                arguments
                    .iter()
                    .map(|argument| (*argument).to_owned())
                    .collect(),
            );
            if !self.succeeds {
                return Err(());
            }
            if arguments.contains(&"tree") {
                Ok(self.tree.clone())
            } else if arguments.contains(&"get-property") {
                Ok(self.state.clone())
            } else {
                Ok(String::new())
            }
        }
    }

    fn dbus_backend(tree: &str) -> WirePlumberBackend<MockDbusRunner> {
        WirePlumberBackend::new(MockDbusRunner {
            tree: tree.to_owned(),
            state: "s \"incoming\"".to_owned(),
            commands: Mutex::new(Vec::new()),
            succeeds: true,
        })
    }

    #[test]
    fn wireplumber_backend_discovers_numeric_private_paths() {
        let backend = dbus_backend(
            "└─ /org/pipewire/Telephony\n  └─ /org/pipewire/Telephony/ag7\n    └─ /org/pipewire/Telephony/ag7/call3\n",
        );
        backend.execute(&CallCommand::Answer).unwrap();
        let commands = backend.runner.commands.lock().unwrap();
        assert_eq!(
            commands[2],
            [
                "--user",
                "call",
                TELEPHONY_SERVICE,
                "/org/pipewire/Telephony/ag7/call3",
                CALL_INTERFACE,
                "Answer",
            ]
        );
    }

    #[test]
    fn wireplumber_backend_uses_gateway_methods_and_redacted_errors() {
        let backend = dbus_backend("└─ /org/pipewire/Telephony/ag2\n");
        let target_text = "+12025550101";
        backend
            .execute(&CallCommand::Dial(DialTarget::parse(target_text).unwrap()))
            .unwrap();
        let commands = backend.runner.commands.lock().unwrap();
        assert_eq!(commands[1].last().unwrap(), target_text);
        assert!(!format!("{:?}", WirePlumberBackendError::Rejected).contains(target_text));
        assert!(
            !WirePlumberBackendError::Rejected
                .to_string()
                .contains(target_text)
        );

        let hangup = dbus_backend(
            "└─ /org/pipewire/Telephony/ag2\n  └─ /org/pipewire/Telephony/ag2/call1\n",
        );
        hangup.execute(&CallCommand::HangUp).unwrap();
        let commands = hangup.runner.commands.lock().unwrap();
        assert_eq!(commands[1].last().unwrap(), "HangupAll");
    }

    #[test]
    fn wireplumber_backend_refuses_ambiguous_or_unsupported_control() {
        let disconnected = dbus_backend("");
        assert_eq!(
            disconnected.execute(&CallCommand::HangUp),
            Err(WirePlumberBackendError::Unavailable)
        );
        let ambiguous =
            dbus_backend("├─ /org/pipewire/Telephony/ag0\n└─ /org/pipewire/Telephony/ag1\n");
        assert_eq!(
            ambiguous.execute(&CallCommand::HangUp),
            Err(WirePlumberBackendError::Ambiguous)
        );

        let backend = dbus_backend("└─ /org/pipewire/Telephony/ag0\n");
        assert_eq!(
            backend.execute(&CallCommand::SetSpeakerGain(Gain::new(8).unwrap())),
            Err(WirePlumberBackendError::Unsupported)
        );
    }

    #[test]
    fn wireplumber_backend_gates_commands_on_live_state() {
        let mut backend = dbus_backend(
            "└─ /org/pipewire/Telephony/ag0\n  └─ /org/pipewire/Telephony/ag0/call0\n",
        );
        backend.runner.state = "s \"active\"".to_owned();
        assert_eq!(
            backend.execute(&CallCommand::Answer),
            Err(WirePlumberBackendError::Rejected)
        );
        backend
            .execute(&CallCommand::SendDtmf(DtmfTone::parse('5').unwrap()))
            .unwrap();
    }

    #[test]
    fn wireplumber_snapshot_reduces_private_paths_to_aggregate_state() {
        let private_marker = "PRIVATE-CALL-PATH-MARKER";
        let backend = WirePlumberBackend::new(MockDbusRunner {
            tree: format!(
                "└─ /org/pipewire/Telephony/ag8\n  └─ /org/pipewire/Telephony/ag8/call4 {private_marker}\n"
            ),
            state: "s \"active\"".to_owned(),
            commands: Mutex::new(Vec::new()),
            succeeds: true,
        });
        let snapshot = backend.snapshot().unwrap();
        assert_eq!(snapshot.control, HfpControlState::SlcReady);
        assert_eq!(snapshot.call, CallState::Active);
        assert!(!format!("{snapshot:?}").contains(private_marker));

        let idle = dbus_backend("└─ /org/pipewire/Telephony/ag8\n");
        assert_eq!(idle.snapshot().unwrap().call, CallState::Idle);
    }

    #[test]
    fn aggregate_call_state_has_privacy_safe_multi_call_precedence() {
        assert_eq!(aggregate_call_state(&[]), CallState::Idle);
        assert_eq!(
            aggregate_call_state(&[LiveCallState::Held, LiveCallState::Incoming]),
            CallState::Incoming
        );
        assert_eq!(
            aggregate_call_state(&[LiveCallState::Incoming, LiveCallState::Active]),
            CallState::Active
        );
        assert_eq!(
            aggregate_call_state(&[LiveCallState::Alerting]),
            CallState::Outgoing
        );
        assert_eq!(
            aggregate_call_state(&[LiveCallState::Disconnected]),
            CallState::Ended
        );
    }
}
