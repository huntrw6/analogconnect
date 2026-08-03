pub mod audio;
pub mod auth;
pub mod contacts;
pub mod hfp;
pub mod media_auth;
pub mod messages;
mod process;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use analogconnect_core::SystemStatus;
use audio::{
    AudioBridge, AudioBridgeSummary, AudioStateBackend, LiveAudioBridge, PipeWireAudioStateBackend,
    PwDumpRunner, ScoNodeError, ScoNodeLocator, ScoTeardownWatchdog,
};
use auth::{AuthToken, AuthTokens, MutationLimiter};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    routing::get,
};
use contacts::ContactSummary;
use futures_util::{SinkExt, StreamExt};
use hfp::{
    BusctlRunner, HfpCommandBackend, HfpStateBackend, WirePlumberBackend, WirePlumberBackendError,
};
use media_auth::{MediaSessionRegistry, OsRandomSource};
use messages::{ImsgMapBackend, MapSendBackend, MessageSyncSummary, OutboundMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone)]
pub struct AppState {
    status: Arc<RwLock<SystemStatus>>,
    contact_summary: Arc<RwLock<ContactSummary>>,
    message_summary: Arc<RwLock<MessageSyncSummary>>,
    audio_bridge: Arc<AudioBridge>,
    auth_tokens: Arc<AuthTokens>,
    message_sender: Arc<dyn MapSendBackend>,
    call_backend: Arc<dyn HfpCommandBackend<Error = WirePlumberBackendError>>,
    hfp_state_backend: Option<Arc<dyn HfpStateBackend<Error = WirePlumberBackendError>>>,
    audio_state_backend: Option<Arc<dyn AudioStateBackend<Error = ScoNodeError>>>,
    sco_teardown_watchdog: Arc<std::sync::Mutex<ScoTeardownWatchdog>>,
    mutation_limiter: Arc<MutationLimiter>,
    media_sessions: Arc<MediaSessionRegistry>,
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
            message_summary: Arc::new(RwLock::new(MessageSyncSummary::default())),
            audio_bridge: Arc::new(
                AudioBridge::new(8).expect("fixed audio queue capacity is valid"),
            ),
            auth_tokens: Arc::new(auth_tokens),
            message_sender,
            call_backend,
            hfp_state_backend,
            audio_state_backend,
            sco_teardown_watchdog: Arc::new(std::sync::Mutex::new(ScoTeardownWatchdog::new(
                Duration::from_secs(10),
            ))),
            mutation_limiter: Arc::new(MutationLimiter::new(10, Duration::from_secs(60))),
            media_sessions: Arc::new(MediaSessionRegistry::new()),
            started_at: Instant::now(),
        }
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
        .route("/api/v1/messages/summary", get(message_summary))
        .route("/api/v1/messages", axum::routing::post(send_message))
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

async fn message_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MessageSyncSummary>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(state.message_summary.read().await.clone()))
}

#[derive(Debug, Serialize)]
struct SendMessageResponse {
    accepted: bool,
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
    let request: OutboundMessage =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let message = OutboundMessage::new(request.recipient, request.body)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let sender = state.message_sender.clone();
    tokio::task::spawn_blocking(move || sender.send(&message))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(SendMessageResponse { accepted: true }),
    ))
    .inspect(|_| tracing::info!(event = "message_send_accepted", "message command accepted"))
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
const MAX_MEDIA_PACKET_BYTES: usize = 512;

async fn open_audio_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<axum::response::Response, StatusCode> {
    let lease = claim_media_session(&state, &headers)?;
    let audio_bridge = Arc::clone(&state.audio_bridge);
    Ok(upgrade
        .max_message_size(MAX_MEDIA_PACKET_BYTES)
        .max_frame_size(MAX_MEDIA_PACKET_BYTES)
        .on_upgrade(move |socket| media_socket(socket, lease, audio_bridge)))
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
) {
    let nodes = match ScoNodeLocator::new(PwDumpRunner::default()).locate() {
        Ok(nodes) => nodes,
        Err(_) => {
            let mut socket = socket;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let _live_bridge = match LiveAudioBridge::start(
        "pw-cat",
        nodes,
        analogconnect_core::AudioFormat::HFP_WIDEBAND,
        Arc::clone(&audio_bridge),
    ) {
        Ok(bridge) => bridge,
        Err(_) => {
            let mut socket = socket;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let (mut sender, mut receiver) = socket.split();
    let media_started = Instant::now();
    let mut playout = tokio::time::interval(Duration::from_micros(7_500));
    playout.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    while lease.is_active() {
        tokio::select! {
            message = receiver.next() => match message {
                Some(Ok(Message::Binary(packet))) if packet.len() <= MAX_MEDIA_PACKET_BYTES => {
                    if receive_uplink_packet(&audio_bridge, &packet).is_err() {
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    if sender.send(Message::Pong(payload)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
            },
            _ = playout.tick() => {
                if _live_bridge.failure_code().is_some() {
                    let _ = sender.send(Message::Close(None)).await;
                    break;
                }
                match take_downlink_packet(&audio_bridge, media_started.elapsed()) {
                    Ok(Some(packet)) => {
                        if sender.send(Message::Binary(packet.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => {
                        let _ = sender.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        }
    }
}

fn receive_uplink_packet(bridge: &AudioBridge, bytes: &[u8]) -> Result<(), MediaStreamError> {
    let packet = analogconnect_core::AudioPacket::decode(bytes)
        .map_err(|_| MediaStreamError::InvalidPacket)?;
    bridge
        .uplink
        .push(packet.frame().clone())
        .map_err(|_| MediaStreamError::QueueUnavailable)
}

fn take_downlink_packet(
    bridge: &AudioBridge,
    capture_time: Duration,
) -> Result<Option<Vec<u8>>, MediaStreamError> {
    let Some(frame) = bridge
        .downlink
        .pop()
        .map_err(|_| MediaStreamError::QueueUnavailable)?
    else {
        return Ok(None);
    };
    let capture_time_micros = u64::try_from(capture_time.as_micros()).unwrap_or(u64::MAX);
    analogconnect_core::AudioPacket::new(capture_time_micros, frame)
        .encode()
        .map(Some)
        .map_err(|_| MediaStreamError::InvalidPacket)
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
        assert_eq!(json, serde_json::json!({ "accepted": true }));
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
        let encoded = take_downlink_packet(&bridge, Duration::from_micros(22_000))
            .unwrap()
            .unwrap();
        let decoded = AudioPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.capture_time_micros(), 22_000);
        assert_eq!(decoded.frame(), &downlink);
        assert!(
            take_downlink_packet(&bridge, Duration::from_micros(30_000))
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
