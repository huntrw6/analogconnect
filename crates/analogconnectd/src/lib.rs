pub mod audio;
pub mod auth;
pub mod contacts;
pub mod hfp;
pub mod messages;

use std::{sync::Arc, time::Instant};

use analogconnect_core::SystemStatus;
use audio::AudioBridgeSummary;
use auth::AuthToken;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use contacts::ContactSummary;
use messages::MessageSyncSummary;
use serde::Serialize;
use tokio::sync::RwLock;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone)]
pub struct AppState {
    status: Arc<RwLock<SystemStatus>>,
    contact_summary: Arc<RwLock<ContactSummary>>,
    message_summary: Arc<RwLock<MessageSyncSummary>>,
    audio_summary: Arc<RwLock<AudioBridgeSummary>>,
    auth_token: Arc<AuthToken>,
    started_at: Instant,
}

impl AppState {
    pub fn new(status: SystemStatus, auth_token: AuthToken) -> Self {
        Self {
            status: Arc::new(RwLock::new(status)),
            contact_summary: Arc::new(RwLock::new(ContactSummary::default())),
            message_summary: Arc::new(RwLock::new(MessageSyncSummary::default())),
            audio_summary: Arc::new(RwLock::new(AudioBridgeSummary::default())),
            auth_token: Arc::new(auth_token),
            started_at: Instant::now(),
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
        .route("/api/v1/audio/summary", get(audio_summary))
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

async fn audio_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AudioBridgeSummary>, StatusCode> {
    authorize(&state, &headers)?;
    Ok(Json(state.audio_summary.read().await.clone()))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let candidate = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .auth_token
        .matches(candidate)
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;

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
        AppState::new(
            SystemStatus::default(),
            AuthToken::new(test_token_text()).unwrap(),
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
    async fn non_health_endpoints_require_authentication() {
        for uri in [
            "/api/v1/status",
            "/api/v1/contacts/summary",
            "/api/v1/messages/summary",
            "/api/v1/audio/summary",
        ] {
            let response = app(test_state())
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");

            let response = app(test_state())
                .oneshot(
                    Request::builder()
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
}
