pub mod ancs_groups;
pub mod ancs_transport;
pub mod audio;
pub mod auth;
pub mod contacts;
pub mod conversations;
pub mod hfp;
pub mod media_auth;
pub mod messages;
mod process;

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use analogconnect_core::{ContactSource, SystemStatus};
use audio::{
    AudioBridge, AudioBridgeSummary, AudioStateBackend, LiveAudioBridge, PipeWireAudioStateBackend,
    PwDumpRunner, ScoNodeError, ScoNodeLocator, ScoTeardownWatchdog,
};
use auth::{AuthToken, AuthTokens, MutationLimiter};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    routing::get,
};
use contacts::{ContactItem, ContactPage, ContactStore, ContactSummary, ImsgContactSource};
use conversations::{
    ConversationAliases, ConversationError, ConversationItem, ConversationKindResponse,
    ConversationPage, ConversationRepository, ImsgConversationRepository, MAX_PAGE_SIZE,
    MessageDirection, MessageDirectionResponse, MessageItem, MessagePage, PageCursor,
};
use futures_util::{SinkExt, StreamExt};
use hfp::{
    BusctlRunner, HfpCommandBackend, HfpStateBackend, WirePlumberBackend, WirePlumberBackendError,
};
use media_auth::{MediaSessionRegistry, OsRandomSource};
use messages::{
    ImsgMapBackend, MapSendBackend, MessageOperationId, MessageOperationRegistry,
    MessageOperationStart, MessageSyncCoordinator, MessageSyncScheduler, MessageSyncSummary,
    OutboundMessage,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone)]
pub struct AppState {
    status: Arc<RwLock<SystemStatus>>,
    contact_summary: Arc<RwLock<ContactSummary>>,
    contact_store: Arc<ContactStore>,
    message_summary: Arc<RwLock<MessageSyncSummary>>,
    message_sync_coordinator: Option<Arc<MessageSyncCoordinator<ImsgMapBackend>>>,
    audio_bridge: Arc<AudioBridge>,
    auth_tokens: Arc<AuthTokens>,
    message_sender: Arc<dyn MapSendBackend>,
    message_operations: Arc<Mutex<MessageOperationRegistry>>,
    conversation_repository: ConversationRepository,
    conversation_aliases: Arc<Mutex<ConversationAliases>>,
    call_backend: Arc<dyn HfpCommandBackend<Error = WirePlumberBackendError>>,
    hfp_state_backend: Option<Arc<dyn HfpStateBackend<Error = WirePlumberBackendError>>>,
    audio_state_backend: Option<Arc<dyn AudioStateBackend<Error = ScoNodeError>>>,
    sco_teardown_watchdog: Arc<std::sync::Mutex<ScoTeardownWatchdog>>,
    mutation_limiter: Arc<MutationLimiter>,
    media_sessions: Arc<MediaSessionRegistry>,
    media_backend: Arc<dyn MediaBackend>,
    started_at: Instant,
}

impl AppState {
    pub fn new(status: SystemStatus, auth_token: AuthToken) -> Self {
        Self::new_with_tokens(status, AuthTokens::new(auth_token))
    }

    pub fn new_with_tokens(status: SystemStatus, auth_tokens: AuthTokens) -> Self {
        Self::with_backends_and_observer(
            status,
            auth_tokens,
            Arc::new(ImsgMapBackend::default()),
            Arc::new(WirePlumberBackend::new(BusctlRunner::default())),
            Some(Arc::new(WirePlumberBackend::new(BusctlRunner::default()))),
            Some(Arc::new(PipeWireAudioStateBackend::new(
                PwDumpRunner::default(),
            ))),
        )
        .with_conversation_repository(ConversationRepository::Imsg(Arc::new(
            ImsgConversationRepository::new(),
        )))
        .with_message_sync_coordinator()
    }

    pub fn with_message_sender(
        status: SystemStatus,
        auth_token: AuthToken,
        message_sender: Arc<dyn MapSendBackend>,
    ) -> Self {
        Self::with_backends(
            status,
            AuthTokens::new(auth_token),
            message_sender,
            Arc::new(WirePlumberBackend::new(BusctlRunner::default())),
        )
    }

    pub fn with_backends(
        status: SystemStatus,
        auth_tokens: AuthTokens,
        message_sender: Arc<dyn MapSendBackend>,
        call_backend: Arc<dyn HfpCommandBackend<Error = WirePlumberBackendError>>,
    ) -> Self {
        Self::with_backends_and_observer(
            status,
            auth_tokens,
            message_sender,
            call_backend,
            None,
            None,
        )
    }

    pub fn with_backends_and_observer(
        status: SystemStatus,
        auth_tokens: AuthTokens,
        message_sender: Arc<dyn MapSendBackend>,
        call_backend: Arc<dyn HfpCommandBackend<Error = WirePlumberBackendError>>,
        hfp_state_backend: Option<Arc<dyn HfpStateBackend<Error = WirePlumberBackendError>>>,
        audio_state_backend: Option<Arc<dyn AudioStateBackend<Error = ScoNodeError>>>,
    ) -> Self {
        Self {
            status: Arc::new(RwLock::new(status)),
            contact_summary: Arc::new(RwLock::new(ContactSummary::default())),
            contact_store: Arc::new(
                ContactStore::in_memory().expect("in-memory contact store initialization failed"),
            ),
            message_summary: Arc::new(RwLock::new(MessageSyncSummary::default())),
            message_sync_coordinator: None,
            audio_bridge: Arc::new(
                AudioBridge::new(64).expect("fixed audio queue capacity is valid"),
            ),
            auth_tokens: Arc::new(auth_tokens),
            message_sender,
            message_operations: Arc::new(Mutex::new(MessageOperationRegistry::new(256))),
            conversation_repository: ConversationRepository::unavailable(),
            conversation_aliases: Arc::new(Mutex::new(ConversationAliases::default())),
            call_backend,
            hfp_state_backend,
            audio_state_backend,
            sco_teardown_watchdog: Arc::new(std::sync::Mutex::new(ScoTeardownWatchdog::new(
                Duration::from_secs(10),
            ))),
            mutation_limiter: Arc::new(MutationLimiter::new(10, Duration::from_secs(60))),
            media_sessions: Arc::new(MediaSessionRegistry::new()),
            media_backend: Arc::new(PipeWireMediaBackend),
            started_at: Instant::now(),
        }
    }

    #[must_use]
    pub fn with_conversation_repository(mut self, repository: ConversationRepository) -> Self {
        self.conversation_repository = repository;
        self
    }

    #[must_use]
    pub fn with_contact_store(mut self, store: Arc<ContactStore>) -> Self {
        self.contact_store = store;
        self
    }

    fn with_message_sync_coordinator(mut self) -> Self {
        self.message_sync_coordinator = Some(Arc::new(MessageSyncCoordinator::new(
            ImsgMapBackend::default(),
            MessageSyncScheduler::new(Duration::from_secs(30), Duration::from_secs(90)),
        )));
        self
    }

    pub fn start_message_sync_task(&self) -> Option<tokio::task::JoinHandle<()>> {
        let coordinator = self.message_sync_coordinator.clone()?;
        Some(tokio::spawn(async move {
            let started_at = Instant::now();
            loop {
                let tick_coordinator = coordinator.clone();
                let result = tokio::task::spawn_blocking(move || {
                    tick_coordinator.tick(started_at.elapsed())
                })
                .await;
                if !matches!(result, Ok(Ok(()))) {
                    tracing::warn!(
                        event = "message_sync_failed",
                        "message synchronization failed with redacted diagnostics"
                    );
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }))
    }

    pub fn start_contact_sync_task(&self) -> tokio::task::JoinHandle<()> {
        let store = self.contact_store.clone();
        let shared_summary = self.contact_summary.clone();
        tokio::spawn(async move {
            loop {
                let tick_store = store.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let contacts = ImsgContactSource::default().pull_all().map_err(|_| ())?;
                    let mut summary = tick_store.replace_all(&contacts).map_err(|_| ())?;
                    summary.last_sync_unix_seconds = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|duration| duration.as_secs());
                    Ok::<_, ()>(summary)
                })
                .await;
                match result {
                    Ok(Ok(summary)) => {
                        *shared_summary.write().await = summary;
                    }
                    Ok(Err(())) | Err(_) => {
                        tracing::warn!(
                            event = "contact_sync_failed",
                            "contact synchronization failed with redacted diagnostics"
                        );
                    }
                }
                tokio::time::sleep(Duration::from_secs(15 * 60)).await;
            }
        })
    }

    async fn refresh_runtime_status(&self) {
        let hfp_snapshot = if let Some(backend) = self.hfp_state_backend.clone() {
            Some(tokio::task::spawn_blocking(move || backend.snapshot()).await)
        } else {
            None
        };
        let audio_snapshot = if let Some(backend) = self.audio_state_backend.clone() {
            Some(tokio::task::spawn_blocking(move || backend.snapshot()).await)
        } else {
            None
        };
        let mut status = self.status.write().await;
        let mut hfp_transport = None;
        if let Some(snapshot) = hfp_snapshot {
            match snapshot {
                Ok(Ok(snapshot)) => {
                    status.hfp_control = snapshot.control;
                    status.call = snapshot.call;
                    hfp_transport = Some(snapshot.transport);
                }
                Ok(Err(WirePlumberBackendError::Unavailable)) => {
                    status.hfp_control = analogconnect_core::HfpControlState::Disconnected;
                    status.call = analogconnect_core::CallState::Idle;
                    hfp_transport = Some(analogconnect_core::AudioTransportState::Inactive);
                }
                Ok(Err(_)) | Err(_) => {
                    status.hfp_control = analogconnect_core::HfpControlState::Error;
                    status.call = analogconnect_core::CallState::Error;
                    hfp_transport = Some(analogconnect_core::AudioTransportState::Error);
                }
            }
        }
        if let Some(snapshot) = audio_snapshot {
            match snapshot {
                Ok(Ok(nodes)) => {
                    status.audio = match (nodes, hfp_transport) {
                        (analogconnect_core::AudioTransportState::Inactive, _) => {
                            analogconnect_core::AudioTransportState::Inactive
                        }
                        (_, Some(analogconnect_core::AudioTransportState::Inactive)) => {
                            analogconnect_core::AudioTransportState::Inactive
                        }
                        (
                            analogconnect_core::AudioTransportState::ScoActive,
                            Some(analogconnect_core::AudioTransportState::ScoActive) | None,
                        ) => analogconnect_core::AudioTransportState::ScoActive,
                        _ => analogconnect_core::AudioTransportState::Error,
                    };
                }
                Ok(Err(_)) | Err(_) => {
                    status.audio = analogconnect_core::AudioTransportState::Error;
                }
            }
        }
        if let Ok(mut watchdog) = self.sco_teardown_watchdog.lock() {
            let observation = watchdog.observe(status.call, status.audio, Instant::now());
            status.audio = observation.state;
            if observation.stalled {
                status.last_error = Some(analogconnect_core::RedactedError::new(
                    "sco_teardown_stalled",
                    "call audio did not stop after the call ended",
                ));
            } else if status
                .last_error
                .as_ref()
                .is_some_and(|error| error.code == "sco_teardown_stalled")
            {
                status.last_error = None;
            }
        } else {
            status.audio = analogconnect_core::AudioTransportState::Error;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub protocol_version: u16,
    pub daemon_version: &'static str,
    pub uptime_seconds: u64,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/contacts/summary", get(contact_summary))
        .route("/api/v2/contacts/search", axum::routing::post(contact_page))
        .route("/api/v1/messages/summary", get(message_summary))
        .route("/api/v1/messages", axum::routing::post(send_message))
        .route("/api/v2/conversations", get(conversation_page))
        .route(
            "/api/v2/conversations/messages",
            axum::routing::post(conversation_messages),
        )
        .route(
            "/api/v1/calls/commands",
            axum::routing::post(execute_call_command),
        )
        .route("/api/v1/audio/summary", get(audio_summary))
        .route(
            "/api/v1/audio/sessions",
            axum::routing::post(create_audio_session),
        )
        .route("/api/v1/audio/stream", get(open_audio_stream))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        daemon_version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SystemStatus>, StatusCode> {
    authorize(&state, &headers)?;
    state.refresh_runtime_status().await;
    Ok(Json(state.status.read().await.clone()))
}

async fn contact_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ContactSummary>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(state.contact_summary.read().await.clone()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactPageRequest {
    query: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
}

async fn contact_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(HeaderMap, Json<ContactPage>), StatusCode> {
    authorize(&state, &headers)?;
    if body.len() > 1024 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let request: ContactPageRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let query = request.query.unwrap_or_default();
    if query.chars().count() > 100 || query.contains(['\r', '\n', '\0']) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let cursor = PageCursor::parse(request.cursor.as_deref()).map_err(conversation_status)?;
    let limit = page_limit(request.limit)?;
    let fetch_limit =
        u16::try_from(limit.saturating_add(1)).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let offset = u16::try_from(cursor.offset()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let store = state.contact_store.clone();
    let mut contacts =
        tokio::task::spawn_blocking(move || store.search_names_page(&query, fetch_limit, offset))
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let has_more = contacts.len() > limit;
    contacts.truncate(limit);
    let items = contacts
        .into_iter()
        .map(ContactItem::from)
        .collect::<Vec<_>>();
    let next_cursor = has_more.then(|| cursor.advance(items.len()).encode());
    Ok((
        private_response_headers(),
        Json(ContactPage { items, next_cursor }),
    ))
}

async fn message_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MessageSyncSummary>, StatusCode> {
    authorize(&state, &headers)?;
    if let Some(coordinator) = &state.message_sync_coordinator {
        return coordinator
            .summary()
            .map(Json)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(Json(state.message_summary.read().await.clone()))
}

#[derive(Debug, Serialize)]
struct SendMessageResponse {
    accepted: bool,
    duplicate: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SendMessageRequest {
    recipient: String,
    body: String,
    operation_id: Option<String>,
}

async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<SendMessageResponse>), StatusCode> {
    authorize(&state, &headers)?;
    authorize_mutation(&state)?;
    if body.len() > 4096 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let request: SendMessageRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let message = OutboundMessage::new(request.recipient, request.body)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let operation_id = request
        .operation_id
        .map(MessageOperationId::new)
        .transpose()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if let Some(operation_id) = &operation_id {
        let start = state
            .message_operations
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .begin(operation_id);
        match start {
            MessageOperationStart::AcceptedDuplicate => {
                return Ok((
                    StatusCode::ACCEPTED,
                    Json(SendMessageResponse {
                        accepted: true,
                        duplicate: true,
                    }),
                ));
            }
            MessageOperationStart::InFlight => return Err(StatusCode::CONFLICT),
            MessageOperationStart::New => {}
        }
    }
    let sender = state.message_sender.clone();
    let result = tokio::task::spawn_blocking(move || sender.send(&message))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .and_then(|result| result.map_err(|_| StatusCode::BAD_GATEWAY));
    if let Some(operation_id) = operation_id {
        let mut operations = state
            .message_operations
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if result.is_ok() {
            operations.accepted(operation_id);
        } else {
            operations.failed(&operation_id);
        }
    }
    result?;
    Ok((
        StatusCode::ACCEPTED,
        Json(SendMessageResponse {
            accepted: true,
            duplicate: false,
        }),
    ))
    .inspect(|_| tracing::info!(event = "message_send_accepted", "message command accepted"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConversationPageQuery {
    cursor: Option<String>,
    limit: Option<usize>,
}

async fn conversation_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ConversationPageQuery>,
) -> Result<(HeaderMap, Json<ConversationPage>), StatusCode> {
    authorize(&state, &headers)?;
    let cursor = PageCursor::parse(query.cursor.as_deref()).map_err(conversation_status)?;
    let limit = page_limit(query.limit)?;
    let rows = state
        .conversation_repository
        .conversations()
        .await
        .map_err(conversation_status)?;
    let start = cursor.offset().min(rows.len());
    let end = start.saturating_add(limit).min(rows.len());
    let mut display_names = Vec::with_capacity(end.saturating_sub(start));
    for row in &rows[start..end] {
        let display_name = if row.participant_count <= 1 && !row.is_ancs_group {
            let store = state.contact_store.clone();
            let address = row.display_address.clone();
            tokio::task::spawn_blocking(move || store.lookup_display_name(&address))
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        } else {
            None
        };
        display_names.push(display_name);
    }
    let mut aliases = state
        .conversation_aliases
        .lock()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut items = Vec::with_capacity(end.saturating_sub(start));
    for (row, display_name) in rows[start..end].iter().zip(display_names) {
        let is_group = row.is_ancs_group || row.participant_count > 1;
        let kind = if row.identity_conflict {
            ConversationKindResponse::Ambiguous
        } else if is_group {
            ConversationKindResponse::Group
        } else {
            ConversationKindResponse::Private
        };
        let title = row
            .group_title
            .clone()
            .or_else(|| display_name.clone())
            .unwrap_or_else(|| row.display_address.clone());
        let conversation_id = if row.is_ancs_group {
            aliases
                .expose_stable_id(&row.conversation_key)
                .map_err(conversation_status)?
        } else {
            aliases
                .id_for(&row.conversation_key, &mut OsRandomSource)
                .map_err(conversation_status)?
        };
        let can_reply = !is_group && !row.identity_conflict;
        items.push(ConversationItem {
            conversation_id,
            display_address: row.display_address.clone(),
            display_name,
            is_group,
            reply_supported: can_reply,
            latest_unix_millis: row.latest_unix_millis,
            message_count: row.message_count,
            unread_count: row.unread_count,
            latest_outgoing_state: row.latest_outgoing_state.clone(),
            latest_preview: row.latest_preview.clone(),
            latest_sender: row.latest_sender.clone(),
            latest_sent: row.latest_sent,
            kind,
            title,
            can_reply,
            identity_conflict: row.identity_conflict,
        });
    }
    let next_cursor = (end < rows.len()).then(|| cursor.advance(items.len()).encode());
    Ok((
        private_response_headers(),
        Json(ConversationPage { items, next_cursor }),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConversationMessagesRequest {
    conversation_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
}

async fn conversation_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(HeaderMap, Json<MessagePage>), StatusCode> {
    authorize(&state, &headers)?;
    if body.len() > 4096 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let request: ConversationMessagesRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let cursor = PageCursor::parse(request.cursor.as_deref()).map_err(conversation_status)?;
    let limit = page_limit(request.limit)?;
    let conversation_key = state
        .conversation_aliases
        .lock()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .key_for(&request.conversation_id)
        .map(str::to_owned)
        .ok_or(StatusCode::GONE)?;
    let offset = u16::try_from(cursor.offset()).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let fetch_limit =
        u16::try_from(limit.saturating_add(1)).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let mut rows = state
        .conversation_repository
        .messages_for(&conversation_key, fetch_limit, offset)
        .await
        .map_err(conversation_status)?;
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let items: Vec<_> = rows
        .iter()
        .map(|row| MessageItem {
            message_id: format!("{:016x}", row.local_id),
            timestamp_unix_millis: row.timestamp_unix_millis,
            direction: match row.direction {
                MessageDirection::Received => MessageDirectionResponse::Received,
                MessageDirection::Sent => MessageDirectionResponse::Sent,
            },
            peer_address: row.address.clone(),
            body: row.body.clone(),
            read: row.read,
            outgoing_state: row.outgoing_state.clone(),
        })
        .collect();
    let next_cursor = has_more.then(|| cursor.advance(items.len()).encode());
    Ok((
        private_response_headers(),
        Json(MessagePage { items, next_cursor }),
    ))
}

fn page_limit(requested: Option<usize>) -> Result<usize, StatusCode> {
    let limit = requested.unwrap_or(conversations::DEFAULT_PAGE_SIZE);
    (1..=MAX_PAGE_SIZE)
        .contains(&limit)
        .then_some(limit)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)
}

const fn conversation_status(error: ConversationError) -> StatusCode {
    match error {
        ConversationError::InvalidCursor => StatusCode::UNPROCESSABLE_ENTITY,
        ConversationError::Expired => StatusCode::GONE,
        ConversationError::Unavailable | ConversationError::RandomUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

fn private_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum CallCommandRequest {
    Answer,
    Reject,
    HangUp,
    Dial { target: String },
    SendDtmf { tone: String },
}

impl CallCommandRequest {
    fn validate(self) -> Result<analogconnect_core::CallCommand, StatusCode> {
        use analogconnect_core::{CallCommand, DialTarget, DtmfTone};
        match self {
            Self::Answer => Ok(CallCommand::Answer),
            Self::Reject => Ok(CallCommand::Reject),
            Self::HangUp => Ok(CallCommand::HangUp),
            Self::Dial { target } => DialTarget::parse(&target)
                .map(CallCommand::Dial)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY),
            Self::SendDtmf { tone } => {
                let mut characters = tone.chars();
                let value = characters.next().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                if characters.next().is_some() {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
                DtmfTone::parse(value)
                    .map(CallCommand::SendDtmf)
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct CallCommandResponse {
    accepted: bool,
}

async fn execute_call_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<CallCommandResponse>), StatusCode> {
    authorize(&state, &headers)?;
    authorize_mutation(&state)?;
    if body.len() > 1024 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let request: CallCommandRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let command = request.validate()?;
    let backend = state.call_backend.clone();
    tokio::task::spawn_blocking(move || backend.execute(&command))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(CallCommandResponse { accepted: true }),
    ))
    .inspect(|_| tracing::info!(event = "call_command_accepted", "call command accepted"))
}

async fn audio_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AudioBridgeSummary>, StatusCode> {
    authorize(&state, &headers)?;
    state
        .audio_bridge
        .summary()
        .map(Json)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

#[derive(Serialize)]
struct MediaSessionResponse {
    session_id: String,
    token: String,
    lifetime_seconds: u64,
    audio_format: &'static str,
}

async fn create_audio_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap, Json<MediaSessionResponse>), StatusCode> {
    use analogconnect_core::{AudioTransportState, CallState};

    authorize(&state, &headers)?;
    authorize_mutation(&state)?;
    state.refresh_runtime_status().await;
    let status = state.status.read().await;
    if status.call != CallState::Active || status.audio != AudioTransportState::ScoActive {
        return Err(StatusCode::CONFLICT);
    }
    drop(status);

    let enrollment = state
        .media_sessions
        .issue(&mut OsRandomSource, Duration::from_secs(60))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let response = MediaSessionResponse {
        session_id: enrollment.session_id().to_owned(),
        token: enrollment.token().to_owned(),
        lifetime_seconds: enrollment.lifetime_seconds(),
        audio_format: "hfp_wideband",
    };
    tracing::info!(
        event = "media_session_issued",
        lifetime_seconds = response.lifetime_seconds,
        "media session issued"
    );
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    Ok((StatusCode::CREATED, response_headers, Json(response)))
}

const MEDIA_SESSION_HEADER: &str = "x-analogconnect-session";
const MAX_MEDIA_FRAMES_PER_MESSAGE: usize = 4;
const MAX_MEDIA_PACKET_BYTES: usize = 264;
const MAX_MEDIA_MESSAGE_BYTES: usize = MAX_MEDIA_PACKET_BYTES * MAX_MEDIA_FRAMES_PER_MESSAGE;

trait ActiveMedia: Send + Sync {
    fn failure_code(&self) -> Option<&'static str>;
}

trait MediaBackend: Send + Sync {
    fn start(&self, bridge: Arc<AudioBridge>) -> Result<Box<dyn ActiveMedia>, ()>;
}

struct PipeWireMediaBackend;

impl MediaBackend for PipeWireMediaBackend {
    fn start(&self, bridge: Arc<AudioBridge>) -> Result<Box<dyn ActiveMedia>, ()> {
        let nodes = ScoNodeLocator::new(PwDumpRunner::default())
            .locate()
            .map_err(|_| ())?;
        LiveAudioBridge::start(
            "pw-cat",
            nodes,
            analogconnect_core::AudioFormat::HFP_WIDEBAND,
            bridge,
        )
        .map(|active| Box::new(active) as Box<dyn ActiveMedia>)
        .map_err(|_| ())
    }
}

impl ActiveMedia for LiveAudioBridge {
    fn failure_code(&self) -> Option<&'static str> {
        LiveAudioBridge::failure_code(self)
    }
}

async fn open_audio_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<axum::response::Response, StatusCode> {
    let lease = claim_media_session(&state, &headers)?;
    let audio_bridge = Arc::clone(&state.audio_bridge);
    let media_backend = Arc::clone(&state.media_backend);
    Ok(upgrade
        .max_message_size(MAX_MEDIA_MESSAGE_BYTES)
        .max_frame_size(MAX_MEDIA_MESSAGE_BYTES)
        .on_upgrade(move |socket| media_socket(socket, lease, audio_bridge, media_backend)))
}

fn claim_media_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<media_auth::MediaSessionLease, StatusCode> {
    let session_id = headers
        .get(MEDIA_SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .media_sessions
        .claim(session_id, token, Instant::now())
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

async fn media_socket(
    socket: WebSocket,
    lease: media_auth::MediaSessionLease,
    audio_bridge: Arc<AudioBridge>,
    media_backend: Arc<dyn MediaBackend>,
) {
    if audio_bridge.reset().is_err() {
        let mut socket = socket;
        let _ = socket.send(Message::Close(None)).await;
        return;
    }
    let live_bridge = match media_backend.start(Arc::clone(&audio_bridge)) {
        Ok(bridge) => bridge,
        Err(_) => {
            let mut socket = socket;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let (mut sender, mut receiver) = socket.split();
    let media_started = Instant::now();
    let (control_sender, mut control_receiver) = tokio::sync::mpsc::channel(4);
    let receive_loop = async {
        while lease.is_active() {
            match receiver.next().await {
                Some(Ok(Message::Binary(packet))) if packet.len() <= MAX_MEDIA_MESSAGE_BYTES => {
                    if receive_uplink_packet(&audio_bridge, &packet).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    if control_sender.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => break,
            }
        }
    };
    let send_loop = async {
        let mut playout = tokio::time::interval(Duration::from_micros(15_000));
        playout.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        while lease.is_active() {
            tokio::select! {
                control = control_receiver.recv() => {
                    let Some(control) = control else { break };
                    if sender.send(control).await.is_err() {
                        break;
                    }
                }
                _ = playout.tick() => {
                if live_bridge.failure_code().is_some() {
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
                for _ in 0..16 {
                    match take_downlink_batch(&audio_bridge, media_started.elapsed()) {
                        Ok(Some(packet)) => {
                            if sender.send(Message::Binary(packet.into())).await.is_err() {
                                return;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            let _ = sender.send(Message::Close(None)).await;
                            return;
                        }
                    }
                }
            }
            }
        }
    };
    tokio::select! {
        () = receive_loop => {}
        () = send_loop => {}
    };
}

fn receive_uplink_packet(bridge: &AudioBridge, bytes: &[u8]) -> Result<(), MediaStreamError> {
    let frames = decode_audio_batch(bytes)?;
    for frame in frames {
        bridge
            .uplink
            .push(frame)
            .map_err(|_| MediaStreamError::QueueUnavailable)?;
    }
    Ok(())
}

fn decode_audio_batch(
    bytes: &[u8],
) -> Result<Vec<analogconnect_core::AudioFrame>, MediaStreamError> {
    let mut frames = Vec::with_capacity(MAX_MEDIA_FRAMES_PER_MESSAGE);
    let mut offset = 0;
    let mut expected_format = None;
    let mut expected_sequence = None;
    while offset < bytes.len() && frames.len() < MAX_MEDIA_FRAMES_PER_MESSAGE {
        if bytes.len() - offset < 6 {
            return Err(MediaStreamError::InvalidPacket);
        }
        let packet_bytes = match bytes[offset + 5] {
            1 => 144,
            2 => 264,
            _ => return Err(MediaStreamError::InvalidPacket),
        };
        let end = offset
            .checked_add(packet_bytes)
            .filter(|end| *end <= bytes.len())
            .ok_or(MediaStreamError::InvalidPacket)?;
        let packet = analogconnect_core::AudioPacket::decode(&bytes[offset..end])
            .map_err(|_| MediaStreamError::InvalidPacket)?;
        if expected_format.is_some_and(|format| format != packet.frame().format())
            || expected_sequence.is_some_and(|sequence| sequence != packet.frame().sequence())
        {
            return Err(MediaStreamError::InvalidPacket);
        }
        expected_format = Some(packet.frame().format());
        expected_sequence = packet.frame().sequence().checked_add(1);
        if expected_sequence.is_none() && end != bytes.len() {
            return Err(MediaStreamError::InvalidPacket);
        }
        frames.push(packet.frame().clone());
        offset = end;
    }
    if frames.is_empty() || offset != bytes.len() {
        return Err(MediaStreamError::InvalidPacket);
    }
    Ok(frames)
}

fn take_downlink_batch(
    bridge: &AudioBridge,
    capture_time: Duration,
) -> Result<Option<Vec<u8>>, MediaStreamError> {
    let capture_time_micros = u64::try_from(capture_time.as_micros()).unwrap_or(u64::MAX);
    let mut batch = Vec::with_capacity(MAX_MEDIA_MESSAGE_BYTES);
    for _ in 0..MAX_MEDIA_FRAMES_PER_MESSAGE {
        let Some(frame) = bridge
            .downlink
            .pop()
            .map_err(|_| MediaStreamError::QueueUnavailable)?
        else {
            break;
        };
        batch.extend_from_slice(
            &analogconnect_core::AudioPacket::new(capture_time_micros, frame)
                .encode()
                .map_err(|_| MediaStreamError::InvalidPacket)?,
        );
    }
    Ok((!batch.is_empty()).then_some(batch))
}

#[derive(Debug)]
enum MediaStreamError {
    InvalidPacket,
    QueueUnavailable,
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let candidate = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .auth_tokens
        .matches(candidate)
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn authorize_mutation(state: &AppState) -> Result<(), StatusCode> {
    match state.mutation_limiter.allow() {
        Ok(true) => Ok(()),
        Ok(false) => Err(StatusCode::TOO_MANY_REQUESTS),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;

    struct MockMessageSender {
        calls: AtomicUsize,
    }

    struct MockCallBackend {
        calls: AtomicUsize,
    }

    struct MockHfpObserver {
        snapshot: Result<hfp::HfpStatusSnapshot, WirePlumberBackendError>,
    }

    struct MockAudioObserver {
        snapshot: Result<analogconnect_core::AudioTransportState, ScoNodeError>,
    }

    struct MockActiveMedia;

    struct MockMediaBackend;

    impl ActiveMedia for MockActiveMedia {
        fn failure_code(&self) -> Option<&'static str> {
            None
        }
    }

    impl MediaBackend for MockMediaBackend {
        fn start(&self, _bridge: Arc<AudioBridge>) -> Result<Box<dyn ActiveMedia>, ()> {
            Ok(Box::new(MockActiveMedia))
        }
    }

    impl HfpStateBackend for MockHfpObserver {
        type Error = WirePlumberBackendError;

        fn snapshot(&self) -> Result<hfp::HfpStatusSnapshot, Self::Error> {
            self.snapshot
        }
    }

    impl AudioStateBackend for MockAudioObserver {
        type Error = ScoNodeError;

        fn snapshot(&self) -> Result<analogconnect_core::AudioTransportState, Self::Error> {
            self.snapshot
        }
    }

    impl HfpCommandBackend for MockCallBackend {
        type Error = WirePlumberBackendError;

        fn execute(&self, _command: &analogconnect_core::CallCommand) -> Result<(), Self::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    impl MapSendBackend for MockMessageSender {
        fn send(
            &self,
            _message: &messages::OutboundMessage,
        ) -> Result<(), messages::OutboundMessageError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn test_token_text() -> String {
        [
            "synthetic",
            "-test",
            "-token",
            "-not",
            "-a",
            "-credential",
            "-0001",
        ]
        .concat()
    }

    fn test_state() -> AppState {
        AppState::with_backends(
            SystemStatus::default(),
            AuthTokens::new(AuthToken::new(test_token_text()).unwrap()),
            Arc::new(MockMessageSender {
                calls: AtomicUsize::new(0),
            }),
            Arc::new(MockCallBackend {
                calls: AtomicUsize::new(0),
            }),
        )
    }

    fn authorized_request(uri: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("authorization", format!("Bearer {}", test_token_text()))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn health_endpoint_is_versioned_and_healthy() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["protocol_version"], PROTOCOL_VERSION);
        assert_eq!(json["daemon_version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn status_endpoint_returns_explicit_independent_states() {
        let response = app(test_state())
            .oneshot(authorized_request("/api/v1/status"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["bluetooth"], "disconnected");
        assert_eq!(json["hfp_control"], "disconnected");
        assert_eq!(json["call"], "idle");
        assert_eq!(json["audio"], "inactive");
    }

    #[tokio::test]
    async fn status_endpoint_refreshes_live_hfp_state_and_fails_closed() {
        use analogconnect_core::{CallState, HfpControlState};

        for (snapshot, expected_control, expected_call) in [
            (
                Ok(hfp::HfpStatusSnapshot {
                    control: HfpControlState::SlcReady,
                    call: CallState::Active,
                    transport: analogconnect_core::AudioTransportState::ScoActive,
                }),
                "slc_ready",
                "active",
            ),
            (
                Err(WirePlumberBackendError::Unavailable),
                "disconnected",
                "idle",
            ),
            (Err(WirePlumberBackendError::Ambiguous), "error", "error"),
        ] {
            let state = AppState::with_backends_and_observer(
                SystemStatus::default(),
                AuthTokens::new(AuthToken::new(test_token_text()).unwrap()),
                Arc::new(MockMessageSender {
                    calls: AtomicUsize::new(0),
                }),
                Arc::new(MockCallBackend {
                    calls: AtomicUsize::new(0),
                }),
                Some(Arc::new(MockHfpObserver { snapshot })),
                None,
            );
            let response = app(state)
                .oneshot(authorized_request("/api/v1/status"))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
            let json: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json["hfp_control"], expected_control);
            assert_eq!(json["call"], expected_call);
        }
    }

    #[tokio::test]
    async fn live_runtime_snapshots_enable_media_issuance_without_static_state() {
        use analogconnect_core::{AudioTransportState, CallState, HfpControlState};

        let state = AppState::with_backends_and_observer(
            SystemStatus::default(),
            AuthTokens::new(AuthToken::new(test_token_text()).unwrap()),
            Arc::new(MockMessageSender {
                calls: AtomicUsize::new(0),
            }),
            Arc::new(MockCallBackend {
                calls: AtomicUsize::new(0),
            }),
            Some(Arc::new(MockHfpObserver {
                snapshot: Ok(hfp::HfpStatusSnapshot {
                    control: HfpControlState::SlcReady,
                    call: CallState::Active,
                    transport: AudioTransportState::ScoActive,
                }),
            })),
            Some(Arc::new(MockAudioObserver {
                snapshot: Ok(AudioTransportState::ScoActive),
            })),
        );
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/audio/sessions")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn idle_gateway_transport_overrides_retained_sco_nodes() {
        use analogconnect_core::{AudioTransportState, CallState, HfpControlState};

        let state = AppState::with_backends_and_observer(
            SystemStatus::default(),
            AuthTokens::new(AuthToken::new(test_token_text()).unwrap()),
            Arc::new(MockMessageSender {
                calls: AtomicUsize::new(0),
            }),
            Arc::new(MockCallBackend {
                calls: AtomicUsize::new(0),
            }),
            Some(Arc::new(MockHfpObserver {
                snapshot: Ok(hfp::HfpStatusSnapshot {
                    control: HfpControlState::SlcReady,
                    call: CallState::Idle,
                    transport: AudioTransportState::Inactive,
                }),
            })),
            Some(Arc::new(MockAudioObserver {
                snapshot: Ok(AudioTransportState::ScoActive),
            })),
        );
        let response = app(state)
            .oneshot(authorized_request("/api/v1/status"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["hfp_control"], "slc_ready");
        assert_eq!(json["call"], "idle");
        assert_eq!(json["audio"], "inactive");
    }

    #[tokio::test]
    async fn stalled_sco_teardown_fails_closed_with_fixed_diagnostic() {
        use analogconnect_core::{AudioTransportState, CallState, HfpControlState};

        let mut state = AppState::with_backends_and_observer(
            SystemStatus::default(),
            AuthTokens::new(AuthToken::new(test_token_text()).unwrap()),
            Arc::new(MockMessageSender {
                calls: AtomicUsize::new(0),
            }),
            Arc::new(MockCallBackend {
                calls: AtomicUsize::new(0),
            }),
            Some(Arc::new(MockHfpObserver {
                snapshot: Ok(hfp::HfpStatusSnapshot {
                    control: HfpControlState::SlcReady,
                    call: CallState::Idle,
                    transport: AudioTransportState::ScoActive,
                }),
            })),
            Some(Arc::new(MockAudioObserver {
                snapshot: Ok(AudioTransportState::ScoActive),
            })),
        );
        state.sco_teardown_watchdog = Arc::new(std::sync::Mutex::new(ScoTeardownWatchdog::new(
            Duration::ZERO,
        )));
        let response = app(state)
            .oneshot(authorized_request("/api/v1/status"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["call"], "idle");
        assert_eq!(json["audio"], "error");
        assert_eq!(json["last_error"]["code"], "sco_teardown_stalled");
        assert_eq!(
            json["last_error"]["summary"],
            "call audio did not stop after the call ended"
        );
    }

    #[tokio::test]
    async fn unversioned_endpoint_is_not_found() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn contact_summary_does_not_expose_contact_data() {
        let response = app(test_state())
            .oneshot(authorized_request("/api/v1/contacts/summary"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["contact_count"], 0);
        assert_eq!(json["phone_count"], 0);
        assert_eq!(json.as_object().unwrap().len(), 3);
    }

    fn contact_test_store() -> Arc<ContactStore> {
        let store = ContactStore::in_memory().unwrap();
        store
            .replace_all(&[
                analogconnect_core::Contact {
                    display_name: Some("Example Alpha".to_owned()),
                    phones: vec![
                        analogconnect_core::PhoneNumber::parse("+1 202 555 0101").unwrap(),
                    ],
                },
                analogconnect_core::Contact {
                    display_name: Some("Example Beta".to_owned()),
                    phones: vec![
                        analogconnect_core::PhoneNumber::parse("+1 202 555 0102").unwrap(),
                    ],
                },
            ])
            .unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn contact_pages_use_private_bodies_pagination_and_no_store_headers() {
        let unauthorized = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/contacts/search")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"sensitive"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let state = test_state().with_contact_store(contact_test_store());
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/contacts/search")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"Example","limit":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
        assert_eq!(json["items"][0]["display_name"], "Example Alpha");
        assert_eq!(
            json["items"][0]["phone_numbers"].as_array().unwrap().len(),
            1
        );
        assert!(json["next_cursor"].as_str().is_some());
        let sentinel = "synthetic-sensitive-sentinel";
        assert!(
            !format!(
                "{:?}",
                ContactItem {
                    display_name: Some(sentinel.to_owned()),
                    phone_numbers: vec![sentinel.to_owned()],
                }
            )
            .contains(sentinel)
        );
    }

    #[tokio::test]
    async fn conversation_page_resolves_name_without_replacing_reply_address() {
        let state = test_state()
            .with_contact_store(contact_test_store())
            .with_conversation_repository(ConversationRepository::InMemory(Arc::new(
                conversations::InMemoryConversationRepository::new(
                    vec![conversations::StoredConversation {
                        conversation_key: "synthetic-contact-thread".to_owned(),
                        display_address: "+1 202 555 0101".to_owned(),
                        participant_count: 1,
                        latest_unix_millis: 20,
                        message_count: 1,
                        unread_count: 0,
                        latest_outgoing_state: None,
                        latest_preview: "Synthetic preview".to_owned(),
                        latest_sender: "+1 202 555 0101".to_owned(),
                        latest_sent: false,
                        group_title: None,
                        is_ancs_group: false,
                        identity_conflict: false,
                    }],
                    vec![],
                ),
            )));
        let response = app(state)
            .oneshot(authorized_request("/api/v2/conversations"))
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"][0]["display_name"], "Example Alpha");
        assert_eq!(json["items"][0]["display_address"], "+1 202 555 0101");
    }

    #[tokio::test]
    async fn ancs_group_and_conflict_api_are_stable_titled_and_fail_closed() {
        let group_id = format!("ancs-v1-{}", "a".repeat(64));
        let conflict_id = format!("ancs-v1-{}", "b".repeat(64));
        let state = test_state().with_conversation_repository(ConversationRepository::InMemory(
            Arc::new(conversations::InMemoryConversationRepository::new(
                vec![
                    conversations::StoredConversation {
                        conversation_key: group_id.clone(),
                        display_address: "synthetic-latest-sender".to_owned(),
                        participant_count: 2,
                        latest_unix_millis: 20,
                        message_count: 2,
                        unread_count: 1,
                        latest_outgoing_state: None,
                        latest_preview: "Synthetic group preview".to_owned(),
                        latest_sender: "synthetic-latest-sender".to_owned(),
                        latest_sent: false,
                        group_title: Some("Synthetic Group Title".to_owned()),
                        is_ancs_group: true,
                        identity_conflict: false,
                    },
                    conversations::StoredConversation {
                        conversation_key: conflict_id.clone(),
                        display_address: "synthetic-other-sender".to_owned(),
                        participant_count: 1,
                        latest_unix_millis: 10,
                        message_count: 1,
                        unread_count: 0,
                        latest_outgoing_state: None,
                        latest_preview: "Synthetic conflict preview".to_owned(),
                        latest_sender: "synthetic-other-sender".to_owned(),
                        latest_sent: false,
                        group_title: Some("Synthetic Conflict Title".to_owned()),
                        is_ancs_group: true,
                        identity_conflict: true,
                    },
                ],
                vec![],
            )),
        ));
        let response = app(state)
            .oneshot(authorized_request("/api/v2/conversations"))
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let group = &json["items"][0];
        assert_eq!(group["conversation_id"], group_id);
        assert_eq!(group["title"], "Synthetic Group Title");
        assert_ne!(group["title"], group["display_address"]);
        assert_eq!(group["kind"], "group");
        assert_eq!(group["can_reply"], false);
        assert_eq!(group["reply_supported"], false);
        let conflict = &json["items"][1];
        assert_eq!(conflict["conversation_id"], conflict_id);
        assert_eq!(conflict["kind"], "ambiguous");
        assert_eq!(conflict["can_reply"], false);
        assert_eq!(conflict["reply_supported"], false);
        assert_eq!(conflict["identity_conflict"], true);
    }

    #[tokio::test]
    async fn message_summary_does_not_expose_message_data() {
        let response = app(test_state())
            .oneshot(authorized_request("/api/v1/messages/summary"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["mode"], "polling");
        assert_eq!(json["successful_syncs"], 0);
        assert_eq!(json.as_object().unwrap().len(), 5);
    }

    fn conversation_test_state() -> AppState {
        test_state().with_conversation_repository(ConversationRepository::InMemory(Arc::new(
            conversations::InMemoryConversationRepository::new(
                vec![
                    conversations::StoredConversation {
                        conversation_key: "synthetic-key-new".to_owned(),
                        display_address: "synthetic-address-new".to_owned(),
                        participant_count: 1,
                        latest_unix_millis: 20,
                        message_count: 2,
                        unread_count: 1,
                        latest_outgoing_state: None,
                        latest_preview: "Synthetic new preview".to_owned(),
                        latest_sender: "synthetic-address-new".to_owned(),
                        latest_sent: false,
                        group_title: None,
                        is_ancs_group: false,
                        identity_conflict: false,
                    },
                    conversations::StoredConversation {
                        conversation_key: "synthetic-key-old".to_owned(),
                        display_address: "synthetic-address-old".to_owned(),
                        participant_count: 2,
                        latest_unix_millis: 10,
                        message_count: 1,
                        unread_count: 0,
                        latest_outgoing_state: Some("sent_confirmed".to_owned()),
                        latest_preview: "Synthetic old preview".to_owned(),
                        latest_sender: "synthetic-address-old".to_owned(),
                        latest_sent: true,
                        group_title: None,
                        is_ancs_group: false,
                        identity_conflict: false,
                    },
                ],
                vec![
                    conversations::StoredMessage {
                        local_id: 2,
                        address: "synthetic-address-new".to_owned(),
                        conversation_key: "synthetic-key-new".to_owned(),
                        timestamp_unix_millis: 20,
                        direction: conversations::MessageDirection::Received,
                        body: "synthetic-private-new".to_owned(),
                        read: false,
                        outgoing_state: None,
                    },
                    conversations::StoredMessage {
                        local_id: 1,
                        address: "synthetic-address-new".to_owned(),
                        conversation_key: "synthetic-key-new".to_owned(),
                        timestamp_unix_millis: 15,
                        direction: conversations::MessageDirection::Sent,
                        body: "synthetic-private-old".to_owned(),
                        read: true,
                        outgoing_state: Some("sent_confirmed".to_owned()),
                    },
                ],
            ),
        )))
    }

    #[tokio::test]
    async fn conversation_pages_are_authenticated_bounded_and_not_cacheable() {
        let unauthorized = app(conversation_test_state())
            .oneshot(
                Request::builder()
                    .uri("/api/v2/conversations?limit=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let first = app(conversation_test_state())
            .oneshot(authorized_request("/api/v2/conversations?limit=1"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(first.headers()["cache-control"], "no-store");
        assert_eq!(first.headers()["pragma"], "no-cache");
        let body = to_bytes(first.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
        assert_eq!(json["items"][0]["display_address"], "synthetic-address-new");
        assert_eq!(json["items"][0]["unread_count"], 1);
        assert_eq!(json["items"][0]["is_group"], false);
        assert_eq!(json["items"][0]["reply_supported"], true);
        assert!(json["next_cursor"].as_str().is_some());
        assert_eq!(
            json["items"][0]["conversation_id"].as_str().unwrap().len(),
            32
        );
    }

    #[tokio::test]
    async fn conversation_history_uses_opaque_id_and_private_request_body() {
        let application = app(conversation_test_state());
        let conversations = application
            .clone()
            .oneshot(authorized_request("/api/v2/conversations"))
            .await
            .unwrap();
        let body = to_bytes(conversations.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let conversation_id = json["items"][0]["conversation_id"].as_str().unwrap();

        let response = application
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/conversations/messages")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"conversation_id":"{conversation_id}","limit":1}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
        assert_eq!(json["items"][0]["body"], "synthetic-private-new");
        assert_eq!(json["items"][0]["direction"], "received");
        assert_eq!(json["items"][0]["peer_address"], "synthetic-address-new");
        assert!(json["next_cursor"].as_str().is_some());
    }

    #[tokio::test]
    async fn conversation_routes_fail_closed_for_unavailable_store_and_expired_alias() {
        let unavailable = app(test_state())
            .oneshot(authorized_request("/api/v2/conversations"))
            .await
            .unwrap();
        assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);

        let expired = app(conversation_test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/conversations/messages")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::from(
                        r#"{"conversation_id":"00000000000000000000000000000000"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(expired.status(), StatusCode::GONE);
    }

    #[tokio::test]
    async fn outbound_message_returns_only_aggregate_acceptance() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/messages")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"recipient":"5550100","body":"synthetic message"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "accepted": true, "duplicate": false })
        );
    }

    #[tokio::test]
    async fn outbound_message_operation_id_suppresses_duplicate_transport_calls() {
        let sender = Arc::new(MockMessageSender {
            calls: AtomicUsize::new(0),
        });
        let application = app(AppState::with_message_sender(
            SystemStatus::default(),
            AuthToken::new(test_token_text()).unwrap(),
            sender.clone(),
        ));
        let payload = r#"{"recipient":"5550100","body":"synthetic message","operation_id":"01010101010101010101010101010101"}"#;

        for duplicate in [false, true] {
            let response = application
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/messages")
                        .header("authorization", format!("Bearer {}", test_token_text()))
                        .header("content-type", "application/json")
                        .body(Body::from(payload))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::ACCEPTED);
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            let json: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json["duplicate"], duplicate);
        }
        assert_eq!(sender.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn outbound_message_rejects_invalid_operation_id_without_transport_call() {
        let sender = Arc::new(MockMessageSender {
            calls: AtomicUsize::new(0),
        });
        let response = app(AppState::with_message_sender(
            SystemStatus::default(),
            AuthToken::new(test_token_text()).unwrap(),
            sender.clone(),
        ))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/messages")
                .header("authorization", format!("Bearer {}", test_token_text()))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"recipient":"5550100","body":"synthetic message","operation_id":"invalid"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(sender.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn outbound_message_rejects_invalid_input_without_echoing_it() {
        let private_body = "synthetic private body marker";
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/messages")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"recipient":"invalid","body":"{private_body}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains(private_body));
    }

    #[tokio::test]
    async fn authenticated_mutations_are_rate_limited() {
        let application = app(test_state());
        for attempt in 0..11 {
            let response = application
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/messages")
                        .header("authorization", format!("Bearer {}", test_token_text()))
                        .body(Body::from(
                            r#"{"recipient":"5550100","body":"synthetic message"}"#,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            let expected = if attempt < 10 {
                StatusCode::ACCEPTED
            } else {
                StatusCode::TOO_MANY_REQUESTS
            };
            assert_eq!(response.status(), expected);
        }
    }

    #[tokio::test]
    async fn call_command_returns_only_aggregate_acceptance() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/calls/commands")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::from(r#"{"action":"hang_up"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({ "accepted": true }));
    }

    #[tokio::test]
    async fn invalid_dial_target_is_not_echoed() {
        let marker = "private-invalid-target";
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/calls/commands")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::from(format!(
                        r#"{{"action":"dial","target":"{marker}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains(marker));
    }

    #[tokio::test]
    async fn audio_summary_contains_no_samples() {
        let response = app(test_state())
            .oneshot(authorized_request("/api/v1/audio/summary"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["uplink"]["depth"], 0);
        assert_eq!(json["downlink"]["depth"], 0);
    }

    #[tokio::test]
    async fn media_session_requires_active_call_and_sco() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/audio/sessions")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(
            to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn media_session_response_is_bounded_and_claimable() {
        use analogconnect_core::{AudioTransportState, CallState};

        let state = test_state();
        {
            let mut status = state.status.write().await;
            status.call = CallState::Active;
            status.audio = AudioTransportState::ScoActive;
        }
        let response = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/audio/sessions")
                    .header("authorization", format!("Bearer {}", test_token_text()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["pragma"], "no-cache");
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 4);
        assert_eq!(json["session_id"].as_str().unwrap().len(), 32);
        assert_eq!(json["token"].as_str().unwrap().len(), 64);
        assert_eq!(json["lifetime_seconds"], 60);
        assert_eq!(json["audio_format"], "hfp_wideband");
        assert!(
            state
                .media_sessions
                .claim(
                    json["session_id"].as_str().unwrap(),
                    json["token"].as_str().unwrap(),
                    Instant::now()
                )
                .is_ok()
        );
    }

    #[tokio::test]
    async fn media_stream_requires_a_real_upgrade_request() {
        let response = app(test_state())
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audio/stream")
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
    }

    #[test]
    fn media_stream_claims_one_time_session_credentials() {
        let state = test_state();
        let enrollment = state
            .media_sessions
            .issue(&mut OsRandomSource, Duration::from_secs(60))
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            MEDIA_SESSION_HEADER,
            HeaderValue::from_str(enrollment.session_id()).unwrap(),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", enrollment.token())).unwrap(),
        );
        let lease = claim_media_session(&state, &headers).unwrap();
        assert_eq!(
            claim_media_session(&state, &headers).unwrap_err(),
            StatusCode::UNAUTHORIZED
        );
        drop(lease);
        assert!(claim_media_session(&state, &headers).is_ok());
    }

    #[test]
    fn media_stream_moves_valid_packets_through_bounded_queues() {
        use analogconnect_core::{AudioFormat, AudioFrame, AudioPacket};

        let bridge = AudioBridge::new(2).unwrap();
        let format = AudioFormat::HFP_WIDEBAND;
        let uplink =
            AudioFrame::new(7, format, vec![0; usize::from(format.samples_per_channel)]).unwrap();
        let uplink_bytes = AudioPacket::new(12_000, uplink.clone()).encode().unwrap();
        receive_uplink_packet(&bridge, &uplink_bytes).unwrap();
        assert_eq!(bridge.uplink.pop().unwrap(), Some(uplink));

        let downlink =
            AudioFrame::new(8, format, vec![0; usize::from(format.samples_per_channel)]).unwrap();
        bridge.downlink.push(downlink.clone()).unwrap();
        let encoded = take_downlink_batch(&bridge, Duration::from_micros(22_000))
            .unwrap()
            .unwrap();
        let decoded = AudioPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.capture_time_micros(), 22_000);
        assert_eq!(decoded.frame(), &downlink);
        assert!(
            take_downlink_batch(&bridge, Duration::from_micros(30_000))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn media_stream_rejects_malformed_uplink_without_queueing_it() {
        let bridge = AudioBridge::new(2).unwrap();
        assert!(receive_uplink_packet(&bridge, b"not an audio packet").is_err());
        assert_eq!(
            bridge.summary().unwrap(),
            audio::AudioBridgeSummary::default()
        );
    }

    #[tokio::test]
    async fn real_websocket_upgrade_moves_authenticated_packets_both_directions() {
        use analogconnect_core::{AudioFormat, AudioFrame, AudioPacket};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        use tokio_tungstenite::tungstenite::{
            Message as ClientMessage, http::HeaderValue as WsHeaderValue,
        };

        let mut state = test_state();
        state.media_backend = Arc::new(MockMediaBackend);
        let enrollment = state
            .media_sessions
            .issue(&mut OsRandomSource, Duration::from_secs(60))
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = app(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await });

        let url = format!("ws://{address}/api/v1/audio/stream");
        let mut request = url.clone().into_client_request().unwrap();
        request.headers_mut().insert(
            MEDIA_SESSION_HEADER,
            WsHeaderValue::from_str(enrollment.session_id()).unwrap(),
        );
        request.headers_mut().insert(
            "authorization",
            WsHeaderValue::from_str(&format!("Bearer {}", enrollment.token())).unwrap(),
        );
        let (mut socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        let mut duplicate_request = url.into_client_request().unwrap();
        duplicate_request.headers_mut().insert(
            MEDIA_SESSION_HEADER,
            WsHeaderValue::from_str(enrollment.session_id()).unwrap(),
        );
        duplicate_request.headers_mut().insert(
            "authorization",
            WsHeaderValue::from_str(&format!("Bearer {}", enrollment.token())).unwrap(),
        );
        assert!(
            tokio_tungstenite::connect_async(duplicate_request)
                .await
                .is_err()
        );

        let format = AudioFormat::HFP_WIDEBAND;
        let uplink =
            AudioFrame::new(5, format, vec![0; usize::from(format.samples_per_channel)]).unwrap();
        socket
            .send(ClientMessage::Binary(
                AudioPacket::new(10_000, uplink.clone())
                    .encode()
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if state.audio_bridge.uplink.summary().unwrap().enqueued == 1 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(state.audio_bridge.uplink.pop().unwrap(), Some(uplink));

        let downlink =
            AudioFrame::new(6, format, vec![0; usize::from(format.samples_per_channel)]).unwrap();
        state.audio_bridge.downlink.push(downlink.clone()).unwrap();
        let received = tokio::time::timeout(Duration::from_secs(1), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let ClientMessage::Binary(bytes) = received else {
            panic!("expected binary downlink frame");
        };
        assert_eq!(AudioPacket::decode(&bytes).unwrap().frame(), &downlink);

        let (mut upload_socket, mut download_socket) = socket.split();
        let upload = async {
            let mut interval = tokio::time::interval(Duration::from_micros(30_000));
            for batch_index in 0..20_u64 {
                interval.tick().await;
                let mut batch = Vec::new();
                for offset in 0..4_u64 {
                    let sequence = 100 + batch_index * 4 + offset;
                    let frame = AudioFrame::new(
                        sequence,
                        format,
                        vec![0; usize::from(format.samples_per_channel)],
                    )
                    .unwrap();
                    batch.extend_from_slice(
                        &AudioPacket::new(sequence * 7_500, frame).encode().unwrap(),
                    );
                }
                upload_socket
                    .send(ClientMessage::Binary(batch.into()))
                    .await
                    .unwrap();
            }
        };
        let produce = async {
            let mut interval = tokio::time::interval(Duration::from_millis(240));
            for burst in 0..3_u64 {
                interval.tick().await;
                for offset in 0..32_u64 {
                    let sequence = 200 + burst * 32 + offset;
                    state
                        .audio_bridge
                        .downlink
                        .push(
                            AudioFrame::new(
                                sequence,
                                format,
                                vec![0; usize::from(format.samples_per_channel)],
                            )
                            .unwrap(),
                        )
                        .unwrap();
                }
            }
        };
        let download = async {
            let mut received_frames = 0;
            while received_frames < 70 {
                match download_socket.next().await {
                    Some(Ok(ClientMessage::Binary(bytes))) => {
                        received_frames += decode_audio_batch(&bytes).unwrap().len();
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => panic!("sustained downlink failed: {error}"),
                    None => break,
                }
            }
            received_frames
        };
        let (_, _, sustained_downlink) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(upload, produce, download)
        })
        .await
        .unwrap();
        assert!(sustained_downlink >= 70);
        server.abort();
    }

    #[tokio::test]
    async fn non_health_endpoints_require_authentication() {
        for (method, uri) in [
            ("GET", "/api/v1/status"),
            ("GET", "/api/v1/contacts/summary"),
            ("GET", "/api/v1/messages/summary"),
            ("POST", "/api/v1/messages"),
            ("POST", "/api/v1/calls/commands"),
            ("GET", "/api/v1/audio/summary"),
            ("POST", "/api/v1/audio/sessions"),
        ] {
            let response = app(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");

            let response = app(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .header(
                            "authorization",
                            "Bearer definitely-wrong-synthetic-token-value-0001",
                        )
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
        }
    }

    #[tokio::test]
    async fn previous_token_remains_valid_during_staged_rotation() {
        let current = "current-current-current-current-0001";
        let previous = "previous-previous-previous-prev-0001";
        let mut state = test_state();
        state.auth_tokens = Arc::new(AuthTokens::with_previous(
            AuthToken::new(current).unwrap(),
            AuthToken::new(previous).unwrap(),
        ));
        for token in [current, previous] {
            let response = app(state.clone())
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/status")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
