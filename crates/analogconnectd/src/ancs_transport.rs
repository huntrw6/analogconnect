use std::collections::VecDeque;

const MESSAGES_APP: &[u8] = b"com.apple.MobileSMS";
const ATTRIBUTE_IDS: [u8; 7] = [0, 1, 2, 4, 5, 6, 7];
const MAX_RESPONSE_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationEventKind {
    Added,
    Modified,
    Removed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotificationSourceEvent {
    pub kind: NotificationEventKind,
    pub uid: [u8; 4],
}

impl NotificationSourceEvent {
    #[must_use]
    pub fn parse(value: &[u8]) -> Option<Self> {
        let [event_id, _flags, _category, _count, a, b, c, d] = value else {
            return None;
        };
        let kind = match event_id {
            0 => NotificationEventKind::Added,
            1 => NotificationEventKind::Modified,
            2 => NotificationEventKind::Removed,
            _ => return None,
        };
        Some(Self {
            kind,
            uid: [*a, *b, *c, *d],
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AncsNotificationMetadata {
    pub uid: [u8; 4],
    pub title: String,
    pub subtitle: String,
    pub positive_action_label: String,
    pub negative_action_label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationAction {
    Positive,
    Negative,
}

/// Bearer boundary for a future operator-controlled ANCS action experiment. Production code does
/// not call this interface automatically; callers must select an action for a retained UID.
pub trait AncsActionInvoker: Send + Sync {
    fn invoke_notification_action(
        &self,
        uid: [u8; 4],
        action: NotificationAction,
    ) -> Result<(), &'static str>;
}

#[must_use]
pub fn notification_action_request(uid: [u8; 4], action: NotificationAction) -> [u8; 6] {
    let action_id = match action {
        NotificationAction::Positive => 0,
        NotificationAction::Negative => 1,
    };
    [2, uid[0], uid[1], uid[2], uid[3], action_id]
}

/// Stateful ANCS protocol core. A bearer supplies Notification/Data Source bytes and writes the
/// returned Control Point requests; this type owns replay suppression and fragment reassembly.
#[derive(Default)]
pub struct AncsProtocolConsumer {
    queued: VecDeque<[u8; 4]>,
    completed: VecDeque<[u8; 4]>,
    pending: Option<[u8; 4]>,
    response: Vec<u8>,
}

impl AncsProtocolConsumer {
    #[must_use]
    pub fn notification_source(&mut self, value: &[u8]) -> Option<Vec<u8>> {
        let event = NotificationSourceEvent::parse(value)?;
        if event.kind == NotificationEventKind::Removed {
            self.queued.retain(|uid| *uid != event.uid);
            return None;
        }
        if self.completed.contains(&event.uid)
            || self.pending == Some(event.uid)
            || self.queued.contains(&event.uid)
        {
            return None;
        }
        if self.pending.is_none() {
            self.pending = Some(event.uid);
            self.response.clear();
            return Some(attribute_request(event.uid));
        }
        if self.queued.len() < 32 {
            self.queued.push_back(event.uid);
        }
        None
    }

    pub fn data_source(
        &mut self,
        fragment: &[u8],
    ) -> (Option<AncsNotificationMetadata>, Option<Vec<u8>>) {
        if self.pending.is_none()
            || self.response.len().saturating_add(fragment.len()) > MAX_RESPONSE_BYTES
        {
            self.reset_pending();
            return (None, self.start_next());
        }
        self.response.extend_from_slice(fragment);
        let Some((values, consumed)) = parse_attributes(&self.response) else {
            return (None, None);
        };
        let Some(uid) = self.pending.take() else {
            return (None, None);
        };
        self.response.drain(..consumed);
        self.completed.push_back(uid);
        while self.completed.len() > 64 {
            let _ = self.completed.pop_front();
        }
        let metadata = (values[0] == MESSAGES_APP).then(|| AncsNotificationMetadata {
            uid,
            title: String::from_utf8_lossy(&values[1]).into_owned(),
            subtitle: String::from_utf8_lossy(&values[2]).into_owned(),
            positive_action_label: String::from_utf8_lossy(&values[5]).into_owned(),
            negative_action_label: String::from_utf8_lossy(&values[6]).into_owned(),
        });
        (metadata, self.start_next())
    }

    fn reset_pending(&mut self) {
        self.pending = None;
        self.response.clear();
    }

    fn start_next(&mut self) -> Option<Vec<u8>> {
        self.reset_pending();
        let uid = self.queued.pop_front()?;
        self.pending = Some(uid);
        Some(attribute_request(uid))
    }
}

#[must_use]
pub fn attribute_request(uid: [u8; 4]) -> Vec<u8> {
    let mut request = vec![0];
    request.extend_from_slice(&uid);
    request.push(0);
    for id in [1_u8, 2] {
        request.push(id);
        request.extend_from_slice(&256_u16.to_le_bytes());
    }
    request.extend_from_slice(&[4, 5, 6, 7]);
    request
}

fn parse_attributes(data: &[u8]) -> Option<(Vec<Vec<u8>>, usize)> {
    if data.len() < 5 || data[0] != 0 {
        return None;
    }
    let mut offset = 5;
    let mut values = Vec::with_capacity(ATTRIBUTE_IDS.len());
    for expected in ATTRIBUTE_IDS {
        if *data.get(offset)? != expected {
            return None;
        }
        let length = usize::from(u16::from_le_bytes([
            *data.get(offset + 1)?,
            *data.get(offset + 2)?,
        ]));
        let start = offset.checked_add(3)?;
        let end = start.checked_add(length)?;
        values.push(data.get(start..end)?.to_vec());
        offset = end;
    }
    Some((values, offset))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconnectBackoff {
    failures: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupervisorCommand {
    Connect,
    SubscribeNotificationSource,
    SubscribeDataSource,
    WriteControlPoint(Vec<u8>),
    RetryAfterSeconds(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupervisorState {
    Disconnected,
    SubscribingNotificationSource,
    SubscribingDataSource,
    Ready,
}

/// Transport-neutral ANCS lifecycle supervisor. The BlueZ bearer executes commands and returns
/// connection/notification bytes; all notification metadata emitted here is body-free.
pub struct AncsSupervisor {
    state: SupervisorState,
    protocol: AncsProtocolConsumer,
    backoff: ReconnectBackoff,
}

impl Default for AncsSupervisor {
    fn default() -> Self {
        Self {
            state: SupervisorState::Disconnected,
            protocol: AncsProtocolConsumer::default(),
            backoff: ReconnectBackoff::default(),
        }
    }
}

impl AncsSupervisor {
    #[must_use]
    pub const fn start(&self) -> SupervisorCommand {
        SupervisorCommand::Connect
    }

    #[must_use]
    pub fn connected(&mut self) -> SupervisorCommand {
        self.protocol = AncsProtocolConsumer::default();
        self.state = SupervisorState::SubscribingNotificationSource;
        SupervisorCommand::SubscribeNotificationSource
    }

    #[must_use]
    pub fn notification_source_subscribed(&mut self) -> SupervisorCommand {
        self.state = SupervisorState::SubscribingDataSource;
        SupervisorCommand::SubscribeDataSource
    }

    pub fn data_source_subscribed(&mut self) {
        self.state = SupervisorState::Ready;
        self.backoff.connected();
    }

    #[must_use]
    pub fn disconnected(&mut self) -> SupervisorCommand {
        self.state = SupervisorState::Disconnected;
        self.protocol = AncsProtocolConsumer::default();
        SupervisorCommand::RetryAfterSeconds(self.backoff.failed())
    }

    #[must_use]
    pub fn notification_source(&mut self, value: &[u8]) -> Option<SupervisorCommand> {
        (self.state == SupervisorState::Ready)
            .then(|| self.protocol.notification_source(value))
            .flatten()
            .map(SupervisorCommand::WriteControlPoint)
    }

    pub fn data_source(
        &mut self,
        value: &[u8],
    ) -> (Option<AncsNotificationMetadata>, Option<SupervisorCommand>) {
        if self.state != SupervisorState::Ready {
            return (None, None);
        }
        let (metadata, request) = self.protocol.data_source(value);
        (metadata, request.map(SupervisorCommand::WriteControlPoint))
    }
}

impl ReconnectBackoff {
    #[must_use]
    pub const fn new() -> Self {
        Self { failures: 0 }
    }

    pub fn connected(&mut self) {
        self.failures = 0;
    }

    #[must_use]
    pub fn failed(&mut self) -> u64 {
        self.failures = self.failures.saturating_add(1).min(6);
        1_u64 << self.failures
    }
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(uid: [u8; 4], app: &[u8], title: &[u8], subtitle: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0];
        bytes.extend_from_slice(&uid);
        for (id, value) in [
            (0, app),
            (1, title),
            (2, subtitle),
            (4, b"12"),
            (5, b"date"),
            (6, b"Reply"),
            (7, b"Clear"),
        ] {
            bytes.push(id);
            bytes.extend_from_slice(&(value.len() as u16).to_le_bytes());
            bytes.extend_from_slice(value);
        }
        bytes
    }

    #[test]
    fn request_never_contains_message_attribute_and_source_is_strict() {
        let request = attribute_request([1, 2, 3, 4]);
        assert_eq!(request, [0, 1, 2, 3, 4, 0, 1, 0, 1, 2, 0, 1, 4, 5, 6, 7]);
        assert_eq!(NotificationSourceEvent::parse(&[0; 7]), None);
    }

    #[test]
    fn retained_uid_builds_explicit_positive_action_without_invoking_it() {
        let uid = [9, 8, 7, 6];
        assert_eq!(
            notification_action_request(uid, NotificationAction::Positive),
            [2, 9, 8, 7, 6, 0]
        );
        assert_eq!(
            notification_action_request(uid, NotificationAction::Negative),
            [2, 9, 8, 7, 6, 1]
        );
    }

    #[test]
    fn fragments_reassemble_and_duplicates_and_replay_are_suppressed() {
        let uid = [1, 2, 3, 4];
        let source = [0, 0, 4, 1, 1, 2, 3, 4];
        let mut consumer = AncsProtocolConsumer::default();
        assert!(consumer.notification_source(&source).is_some());
        assert!(consumer.notification_source(&source).is_none());
        let response = response(uid, MESSAGES_APP, b"Sender", b"Group");
        let split = response.len() / 2;
        assert_eq!(consumer.data_source(&response[..split]), (None, None));
        let (metadata, next) = consumer.data_source(&response[split..]);
        assert_eq!(metadata.unwrap().subtitle, "Group");
        assert_eq!(next, None);
        assert!(consumer.notification_source(&source).is_none());
    }

    #[test]
    fn non_messages_are_filtered_and_backoff_is_bounded_and_resettable() {
        let mut consumer = AncsProtocolConsumer::default();
        let _ = consumer.notification_source(&[0, 0, 1, 1, 9, 0, 0, 0]);
        let (metadata, _) = consumer.data_source(&response(
            [9, 0, 0, 0],
            b"com.example.other",
            b"private",
            b"private",
        ));
        assert_eq!(metadata, None);
        let mut backoff = ReconnectBackoff::new();
        assert_eq!(
            (0..8).map(|_| backoff.failed()).collect::<Vec<_>>(),
            [2, 4, 8, 16, 32, 64, 64, 64]
        );
        backoff.connected();
        assert_eq!(backoff.failed(), 2);
    }

    #[test]
    fn supervisor_subscribes_in_order_and_resets_protocol_on_reconnect() {
        let uid = [1, 2, 3, 4];
        let source = [0, 0, 4, 1, 1, 2, 3, 4];
        let mut supervisor = AncsSupervisor::default();
        assert_eq!(supervisor.start(), SupervisorCommand::Connect);
        assert_eq!(supervisor.notification_source(&source), None);
        assert_eq!(
            supervisor.connected(),
            SupervisorCommand::SubscribeNotificationSource
        );
        assert_eq!(
            supervisor.notification_source_subscribed(),
            SupervisorCommand::SubscribeDataSource
        );
        supervisor.data_source_subscribed();
        assert!(matches!(
            supervisor.notification_source(&source),
            Some(SupervisorCommand::WriteControlPoint(_))
        ));
        let (metadata, _) =
            supervisor.data_source(&response(uid, MESSAGES_APP, b"Sender", b"Group"));
        assert_eq!(metadata.unwrap().subtitle, "Group");
        assert_eq!(
            supervisor.disconnected(),
            SupervisorCommand::RetryAfterSeconds(2)
        );
        assert_eq!(supervisor.notification_source(&source), None);
        assert_eq!(
            supervisor.connected(),
            SupervisorCommand::SubscribeNotificationSource
        );
    }

    #[test]
    fn partial_subscription_failures_back_off_until_ready() {
        let mut supervisor = AncsSupervisor::default();
        let _ = supervisor.connected();
        assert_eq!(
            supervisor.disconnected(),
            SupervisorCommand::RetryAfterSeconds(2)
        );
        let _ = supervisor.connected();
        assert_eq!(
            supervisor.disconnected(),
            SupervisorCommand::RetryAfterSeconds(4)
        );
        let _ = supervisor.connected();
        let _ = supervisor.notification_source_subscribed();
        supervisor.data_source_subscribed();
        assert_eq!(
            supervisor.disconnected(),
            SupervisorCommand::RetryAfterSeconds(2)
        );
    }
}
