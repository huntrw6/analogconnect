use std::{env, net::SocketAddr};

use analogconnect_core::SystemStatus;
use analogconnectd::{AppState, app, auth::AuthToken};
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8787";

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
    let listen_addr = env::var("ANALOGCONNECT_LISTEN_ADDR")
        .unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned())
        .parse::<SocketAddr>()?;
    let listener = TcpListener::bind(listen_addr).await?;

    info!(
        event = "daemon_started",
        listen_addr = %listen_addr,
        protocol_version = 1_u16,
        "analogconnectd ready"
    );

    axum::serve(
        listener,
        app(AppState::new(SystemStatus::default(), auth_token)),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!(event = "daemon_stopped", "analogconnectd stopped cleanly");
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
