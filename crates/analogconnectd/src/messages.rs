use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use analogconnect_core::MessageSyncState;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSyncMode {
    Notifications,
    #[default]
    Polling,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageSyncSummary {
    pub state: MessageSyncState,
    pub mode: MessageSyncMode,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub last_success_unix_seconds: Option<u64>,
}

impl Default for MessageSyncSummary {
    fn default() -> Self {
        Self {
            state: MessageSyncState::Idle,
            mode: MessageSyncMode::Polling,
            successful_syncs: 0,
            failed_syncs: 0,
            last_success_unix_seconds: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapEventKind {
    NewMessage,
    MessageShift,
    MessageDeleted,
    ReadStatusChanged,
    DeliveryStateChanged,
    MemoryStateChanged,
    Unknown,
}

impl MapEventKind {
    #[must_use]
    pub fn from_wire_name(name: &str) -> Self {
        match name {
            "NewMessage" => Self::NewMessage,
            "MessageShift" => Self::MessageShift,
            "MessageDeleted" => Self::MessageDeleted,
            "ReadStatusChanged" => Self::ReadStatusChanged,
            "DeliverySuccess" | "DeliveryFailure" | "SendingSuccess" | "SendingFailure" => {
                Self::DeliveryStateChanged
            }
            "MemoryFull" | "MemoryAvailable" => Self::MemoryStateChanged,
            _ => Self::Unknown,
        }
    }

    const fn requires_sync(self) -> bool {
        matches!(
            self,
            Self::NewMessage
                | Self::MessageShift
                | Self::MessageDeleted
                | Self::ReadStatusChanged
                | Self::DeliveryStateChanged
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    Notification,
    Poll,
}

/// Determines when to ask imsg's encrypted store to synchronize.
pub struct MessageSyncScheduler {
    mode: MessageSyncMode,
    poll_interval: Duration,
    notification_timeout: Duration,
    last_notification_at: Option<Duration>,
    next_poll_at: Duration,
}

impl MessageSyncScheduler {
    #[must_use]
    pub fn new(poll_interval: Duration, notification_timeout: Duration) -> Self {
        Self {
            mode: MessageSyncMode::Polling,
            poll_interval,
            notification_timeout,
            last_notification_at: None,
            next_poll_at: Duration::ZERO,
        }
    }

    #[must_use]
    pub const fn mode(&self) -> MessageSyncMode {
        self.mode
    }

    pub fn notification(&mut self, now: Duration, wire_name: &str) -> Option<SyncTrigger> {
        self.mode = MessageSyncMode::Notifications;
        self.last_notification_at = Some(now);
        self.next_poll_at = now.saturating_add(self.poll_interval);
        MapEventKind::from_wire_name(wire_name)
            .requires_sync()
            .then_some(SyncTrigger::Notification)
    }

    pub fn tick(&mut self, now: Duration) -> Option<SyncTrigger> {
        if let Some(last) = self.last_notification_at
            && now.saturating_sub(last) >= self.notification_timeout
        {
            self.mode = MessageSyncMode::Polling;
        }

        if self.mode == MessageSyncMode::Polling && now >= self.next_poll_at {
            self.next_poll_at = now.saturating_add(self.poll_interval);
            return Some(SyncTrigger::Poll);
        }
        None
    }
}

pub trait MapSyncBackend: Send + Sync {
    type Error;

    fn sync_inbox(&self) -> Result<(), Self::Error>;
}

#[derive(Debug, Error)]
pub enum MessageCoordinatorError {
    #[error("message synchronization failed")]
    SyncFailed,
    #[error("message synchronization lock was poisoned")]
    LockPoisoned,
}

pub struct MessageSyncCoordinator<B> {
    backend: B,
    scheduler: Mutex<MessageSyncScheduler>,
    summary: Mutex<MessageSyncSummary>,
}

impl<B> MessageSyncCoordinator<B>
where
    B: MapSyncBackend,
{
    #[must_use]
    pub fn new(backend: B, scheduler: MessageSyncScheduler) -> Self {
        Self {
            backend,
            scheduler: Mutex::new(scheduler),
            summary: Mutex::new(MessageSyncSummary::default()),
        }
    }

    pub fn notification(
        &self,
        now: Duration,
        wire_name: &str,
    ) -> Result<(), MessageCoordinatorError> {
        let trigger = {
            let mut scheduler = self
                .scheduler
                .lock()
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
            let trigger = scheduler.notification(now, wire_name);
            self.summary
                .lock()
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?
                .mode = scheduler.mode();
            trigger
        };
        if trigger.is_some() {
            self.synchronize()?;
        }
        Ok(())
    }

    pub fn tick(&self, now: Duration) -> Result<(), MessageCoordinatorError> {
        let trigger = {
            let mut scheduler = self
                .scheduler
                .lock()
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
            let trigger = scheduler.tick(now);
            self.summary
                .lock()
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?
                .mode = scheduler.mode();
            trigger
        };
        if trigger.is_some() {
            self.synchronize()?;
        }
        Ok(())
    }

    pub fn summary(&self) -> Result<MessageSyncSummary, MessageCoordinatorError> {
        self.summary
            .lock()
            .map(|summary| summary.clone())
            .map_err(|_| MessageCoordinatorError::LockPoisoned)
    }

    fn synchronize(&self) -> Result<(), MessageCoordinatorError> {
        {
            let mut summary = self
                .summary
                .lock()
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
            summary
                .state
                .transition_to(MessageSyncState::Syncing)
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
        }

        let result = self.backend.sync_inbox();
        let mut summary = self
            .summary
            .lock()
            .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
        if result.is_ok() {
            summary.successful_syncs = summary.successful_syncs.saturating_add(1);
            summary.last_success_unix_seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs());
            summary
                .state
                .transition_to(MessageSyncState::Idle)
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
            Ok(())
        } else {
            summary.failed_syncs = summary.failed_syncs.saturating_add(1);
            summary
                .state
                .transition_to(MessageSyncState::BackingOff)
                .map_err(|_| MessageCoordinatorError::LockPoisoned)?;
            Err(MessageCoordinatorError::SyncFailed)
        }
    }
}

#[derive(Debug, Error)]
pub enum ImsgMapError {
    #[error("MAP client could not be started")]
    Spawn,
    #[error("MAP synchronization failed")]
    Failed,
}

/// Uses imsg as the owner of encrypted message persistence and incremental cursors.
/// Command output is discarded and never included in logs or errors.
pub struct ImsgMapBackend {
    executable: PathBuf,
}

#[derive(Clone, Deserialize)]
pub struct OutboundMessage {
    pub(crate) recipient: String,
    pub(crate) body: String,
}

impl std::fmt::Debug for OutboundMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("OutboundMessage { recipient: [redacted], body: [redacted] }")
    }
}

impl OutboundMessage {
    pub fn new(recipient: String, body: String) -> Result<Self, OutboundMessageError> {
        let recipient = recipient.trim().to_owned();
        if recipient.len() < 3
            || recipient.len() > 32
            || !recipient
                .chars()
                .all(|character| character.is_ascii_digit() || "+-() ".contains(character))
            || !recipient
                .chars()
                .any(|character| character.is_ascii_digit())
        {
            return Err(OutboundMessageError::InvalidRecipient);
        }
        if body.is_empty() || body.len() > 2_000 || body.contains('\0') {
            return Err(OutboundMessageError::InvalidBody);
        }
        Ok(Self { recipient, body })
    }

    fn recipient(&self) -> &str {
        &self.recipient
    }

    fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OutboundMessageError {
    #[error("recipient must be a valid phone-number-like value")]
    InvalidRecipient,
    #[error("message body must contain between 1 and 2000 bytes")]
    InvalidBody,
    #[error("message transport failed")]
    TransportFailed,
}

pub trait MapSendBackend: Send + Sync {
    fn send(&self, message: &OutboundMessage) -> Result<(), OutboundMessageError>;
}

impl MapSendBackend for ImsgMapBackend {
    fn send(&self, message: &OutboundMessage) -> Result<(), OutboundMessageError> {
        let status = Command::new(&self.executable)
            .arg("send")
            .arg(message.recipient())
            .arg(message.body())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| OutboundMessageError::TransportFailed)?;
        status
            .success()
            .then_some(())
            .ok_or(OutboundMessageError::TransportFailed)
    }
}

impl ImsgMapBackend {
    #[must_use]
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for ImsgMapBackend {
    fn default() -> Self {
        Self::new("imsg")
    }
}

impl MapSyncBackend for ImsgMapBackend {
    type Error = ImsgMapError;

    fn sync_inbox(&self) -> Result<(), Self::Error> {
        let status = Command::new(&self.executable)
            .args(["sync", "--folder", "inbox"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| ImsgMapError::Spawn)?;
        if status.success() {
            Ok(())
        } else {
            Err(ImsgMapError::Failed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockBackend {
        calls: AtomicUsize,
        succeeds: bool,
    }

    impl MapSyncBackend for MockBackend {
        type Error = ();

        fn sync_inbox(&self) -> Result<(), Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.succeeds.then_some(()).ok_or(())
        }
    }

    #[test]
    fn starts_with_polling_so_notifications_are_not_assumed() {
        let scheduler = MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90));
        assert_eq!(scheduler.mode(), MessageSyncMode::Polling);
    }

    #[test]
    fn relevant_notification_triggers_sync_and_suppresses_polling() {
        let mut scheduler =
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90));
        assert_eq!(
            scheduler.notification(Duration::from_secs(5), "NewMessage"),
            Some(SyncTrigger::Notification)
        );
        assert_eq!(scheduler.mode(), MessageSyncMode::Notifications);
        assert_eq!(scheduler.tick(Duration::from_secs(35)), None);
    }

    #[test]
    fn notification_silence_falls_back_to_polling() {
        let mut scheduler =
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90));
        scheduler.notification(Duration::from_secs(5), "MemoryAvailable");
        assert_eq!(
            scheduler.tick(Duration::from_secs(95)),
            Some(SyncTrigger::Poll)
        );
        assert_eq!(scheduler.mode(), MessageSyncMode::Polling);
    }

    #[test]
    fn unknown_events_refresh_health_without_triggering_sync() {
        let mut scheduler =
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90));
        assert_eq!(
            scheduler.notification(Duration::from_secs(10), "FutureEvent"),
            None
        );
        assert_eq!(scheduler.mode(), MessageSyncMode::Notifications);
    }

    #[test]
    fn coordinator_counts_success_without_storing_payloads() {
        let coordinator = MessageSyncCoordinator::new(
            MockBackend {
                calls: AtomicUsize::new(0),
                succeeds: true,
            },
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90)),
        );
        coordinator
            .notification(Duration::from_secs(1), "NewMessage")
            .unwrap();
        let summary = coordinator.summary().unwrap();
        assert_eq!(summary.successful_syncs, 1);
        assert_eq!(summary.failed_syncs, 0);
        assert_eq!(summary.state, MessageSyncState::Idle);
    }

    #[test]
    fn coordinator_backs_off_after_redacted_failure() {
        let coordinator = MessageSyncCoordinator::new(
            MockBackend {
                calls: AtomicUsize::new(0),
                succeeds: false,
            },
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90)),
        );
        assert!(matches!(
            coordinator.tick(Duration::ZERO),
            Err(MessageCoordinatorError::SyncFailed)
        ));
        let summary = coordinator.summary().unwrap();
        assert_eq!(summary.failed_syncs, 1);
        assert_eq!(summary.state, MessageSyncState::BackingOff);
    }

    #[test]
    fn outbound_message_validates_and_redacts_private_values() {
        let recipient = "+1 (555) 010-0200".to_owned();
        let body = "synthetic message body".to_owned();
        let message = OutboundMessage::new(recipient.clone(), body.clone()).unwrap();
        let debug = format!("{message:?}");
        assert!(!debug.contains(&recipient));
        assert!(!debug.contains(&body));
        assert_eq!(
            debug,
            "OutboundMessage { recipient: [redacted], body: [redacted] }"
        );
    }

    #[test]
    fn outbound_message_rejects_invalid_and_oversized_input() {
        assert_eq!(
            OutboundMessage::new("not-a-number".to_owned(), "hello".to_owned()).unwrap_err(),
            OutboundMessageError::InvalidRecipient
        );
        assert_eq!(
            OutboundMessage::new("5550100".to_owned(), String::new()).unwrap_err(),
            OutboundMessageError::InvalidBody
        );
        assert_eq!(
            OutboundMessage::new("5550100".to_owned(), "x".repeat(2_001)).unwrap_err(),
            OutboundMessageError::InvalidBody
        );
    }
}
