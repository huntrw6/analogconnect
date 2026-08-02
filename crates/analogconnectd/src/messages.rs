use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

use analogconnect_core::MessageSyncState;
use serde::Serialize;
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
}
