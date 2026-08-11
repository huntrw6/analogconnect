use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

use analogconnect_core::SystemStatus;
use analogconnectd::{
    AppState, app,
    auth::{AuthToken, AuthTokens},
};
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8787";
const TLS_CERT_ENV: &str = "ANALOGCONNECT_TLS_CERT_PATH";
const TLS_KEY_ENV: &str = "ANALOGCONNECT_TLS_KEY_PATH";

#[derive(Debug, Eq, PartialEq)]
enum ListenerMode {
    Plaintext,
    Tls {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    init_logging();

    if let Err(error) = run().await {
        error!(error_code = "daemon_start_failed", error = %error, "daemon stopped");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let auth_token =
        env::var("ANALOGCONNECT_API_TOKEN").map_err(|_| "ANALOGCONNECT_API_TOKEN is required")?;
    let auth_token = AuthToken::new(auth_token)?;
    let auth_tokens = match env::var("ANALOGCONNECT_API_PREVIOUS_TOKEN") {
        Ok(previous) => AuthTokens::with_previous(auth_token, AuthToken::new(previous)?),
        Err(env::VarError::NotPresent) => AuthTokens::new(auth_token),
        Err(env::VarError::NotUnicode(_)) => {
            return Err("ANALOGCONNECT_API_PREVIOUS_TOKEN is not valid Unicode".into());
        }
    };
    let listen_addr = env::var("ANALOGCONNECT_LISTEN_ADDR")
        .unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned())
        .parse::<SocketAddr>()?;
    let listener_mode = listener_mode(
        optional_env(TLS_CERT_ENV)?,
        optional_env(TLS_KEY_ENV)?,
        listen_addr,
    )?;
    let state = AppState::new_with_tokens(SystemStatus::default(), auth_tokens);
    let _message_sync_task = state.start_message_sync_task();
    let _contact_sync_task = state.start_contact_sync_task();
    let _ancs_task = state.start_ancs_task();
    let router = app(state);

    match listener_mode {
        ListenerMode::Plaintext => {
            let listener = TcpListener::bind(listen_addr).await?;
            log_started(listen_addr, ListenerMode::Plaintext.transport_name());
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        ListenerMode::Tls {
            cert_path,
            key_path,
        } => {
            validate_private_key_file(&key_path)?;
            let tls_config =
                axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
                    .await
                    .map_err(|_| "TLS certificate or private key could not be loaded")?;
            let listener = std::net::TcpListener::bind(listen_addr)?;
            listener.set_nonblocking(true)?;
            let handle = axum_server::Handle::new();
            let shutdown_handle = handle.clone();
            tokio::spawn(async move {
                shutdown_signal().await;
                shutdown_handle.graceful_shutdown(Some(Duration::from_secs(10)));
            });
            log_started(listen_addr, "https");
            axum_server::from_tcp_rustls(listener, tls_config)?
                .handle(handle)
                .serve(router.into_make_service())
                .await?;
        }
    }

    info!(event = "daemon_stopped", "analogconnectd stopped cleanly");
    Ok(())
}

fn log_started(listen_addr: SocketAddr, transport: &'static str) {
    info!(
        event = "daemon_started",
        listen_addr = %listen_addr,
        transport,
        protocol_version = 1_u16,
        "analogconnectd ready"
    );
}

impl ListenerMode {
    const fn transport_name(&self) -> &'static str {
        match self {
            Self::Plaintext => "http",
            Self::Tls { .. } => "https",
        }
    }
}

fn optional_env(name: &'static str) -> Result<Option<String>, &'static str> {
    match env::var(name) {
        Ok(value) if value.is_empty() => Err("TLS path environment variables must not be empty"),
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err("TLS path environment variables must be valid Unicode")
        }
    }
}

fn listener_mode(
    cert_path: Option<String>,
    key_path: Option<String>,
    listen_addr: SocketAddr,
) -> Result<ListenerMode, &'static str> {
    match (cert_path, key_path) {
        (None, None) if listen_addr.ip().is_loopback() => Ok(ListenerMode::Plaintext),
        (None, None) => {
            Err("plaintext listener must use a loopback address; non-loopback requires TLS")
        }
        (Some(cert_path), Some(key_path)) => Ok(ListenerMode::Tls {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
        }),
        _ => Err("both TLS certificate and private key paths are required"),
    }
}

fn validate_private_key_file(key_path: &std::path::Path) -> Result<(), &'static str> {
    let metadata = std::fs::metadata(key_path).map_err(|_| "TLS private key is not readable")?;
    if !metadata.is_file() {
        return Err("TLS private key must be a regular file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("TLS private key permissions must deny group and other access");
        }
    }
    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_env("ANALOGCONNECT_LOG")
        .unwrap_or_else(|_| EnvFilter::new("analogconnectd=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(false)
        .with_span_list(false)
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls12_is_compiled_for_android_8_compatibility() {
        assert_eq!(
            rustls::version::TLS12.version,
            rustls::ProtocolVersion::TLSv1_2
        );
    }

    #[test]
    fn plaintext_listener_is_restricted_to_loopback() {
        for address in ["127.0.0.1:8787", "[::1]:8787"] {
            assert_eq!(
                listener_mode(None, None, address.parse().unwrap()).unwrap(),
                ListenerMode::Plaintext
            );
        }
        for address in ["0.0.0.0:8787", "192.168.1.10:8787", "[::]:8787"] {
            assert!(listener_mode(None, None, address.parse().unwrap()).is_err());
        }
    }

    #[test]
    fn tls_listener_accepts_loopback_and_lan_addresses() {
        for address in ["127.0.0.1:8787", "192.168.1.10:8787", "[::]:8787"] {
            assert_eq!(
                listener_mode(
                    Some("cert.pem".to_owned()),
                    Some("key.pem".to_owned()),
                    address.parse().unwrap(),
                )
                .unwrap(),
                ListenerMode::Tls {
                    cert_path: "cert.pem".into(),
                    key_path: "key.pem".into(),
                }
            );
        }
    }

    #[test]
    fn partial_tls_configuration_is_rejected() {
        let address = "127.0.0.1:8787".parse().unwrap();
        assert!(listener_mode(Some("cert.pem".to_owned()), None, address).is_err());
        assert!(listener_mode(None, Some("key.pem".to_owned()), address).is_err());
    }
}
