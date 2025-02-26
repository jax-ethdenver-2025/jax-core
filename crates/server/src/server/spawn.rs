use futures::future::join_all;
use tokio::time::timeout;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const FINAL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

use axum::Router;
use iroh::protocol::Router as IrohRouter;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::watch::Receiver as WatchReceiver;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::app::Config;

use super::api;
use super::error_handlers;
use super::health;
use super::state::{ServerState, ServerStateSetupError};
use super::utils;

const HEALTH_PREFIX: &str = "/_status";
const API_PREFIX: &str = "/api/v0";

pub async fn http_server(
    remote_listen_addr: SocketAddr,
    log_level: tracing::Level,
    state: ServerState,
    mut shutdown_rx: WatchReceiver<()>,
) -> Result<(), HttpServerError> {
    let trace_layer = TraceLayer::new_for_http()
        .on_response(
            DefaultOnResponse::new()
                .include_headers(false)
                .level(log_level)
                .latency_unit(LatencyUnit::Micros),
        )
        .on_failure(DefaultOnFailure::new().latency_unit(LatencyUnit::Micros));

    let root_router = Router::new()
        .nest(API_PREFIX, api::router(state.clone()))
        .nest(HEALTH_PREFIX, health::router(state.clone()))
        .with_state(state)
        .fallback(error_handlers::not_found_handler)
        .layer(trace_layer);

    tracing::info!(addr = ?remote_listen_addr, "server listening");
    let listener = tokio::net::TcpListener::bind(remote_listen_addr).await?;

    axum::serve(listener, root_router)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
        })
        .await?;

    Ok(())
}

async fn iroh_router(
    state: ServerState,
    mut shutdown_rx: WatchReceiver<()>,
) -> Result<(), IrohRouterError> {
    // Get the resources needed for the router
    let endpoint = state.endpoint().clone();
    let blobs = state.blob_service().get_inner_blobs().clone();

    // Build the Iroh router
    let router = IrohRouter::builder(endpoint)
        .accept(iroh_blobs::ALPN, blobs)
        .spawn()
        .await
        .map_err(IrohRouterError::RouterSetupFailed)?;

    tracing::info!("Iroh router started");

    // Wait for shutdown signal
    let _ = shutdown_rx.changed().await;

    // Gracefully shut down the router
    tracing::info!("Shutting down Iroh router");
    router
        .shutdown()
        .await
        .map_err(IrohRouterError::ShutdownFailed)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum IrohRouterError {
    #[error("failed to set up Iroh router: {0}")]
    RouterSetupFailed(anyhow::Error),

    #[error("failed to shut down Iroh router: {0}")]
    ShutdownFailed(anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error("an error occurred running the HTTP server: {0}")]
    ServingFailed(#[from] std::io::Error),

    #[error("state initialization failed: {0}")]
    StateInitializationFailed(#[from] ServerStateSetupError),
}

pub async fn spawn(config: &Config) {
    // Set up logging
    // TODO: conditional text decoration depending on the environment
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let env_filter = EnvFilter::builder()
        .with_default_directive((*config.log_level()).into())
        .from_env_lossy();

    let stderr_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    utils::register_panic_logger();
    utils::report_version();

    let node_id = config.node_id().unwrap();
    tracing::info!("spawning server managing node id: {}", node_id);

    // Create the app state
    let state = match ServerState::from_config(config).await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("error creating server state: {}", e);
            std::process::exit(3);
        }
    };

    let (graceful_waiter, shutdown_rx) = utils::graceful_shutdown_blocker();
    let mut handles = Vec::new();

    // Start HTTP server
    let http_state = state.clone();
    let http_rx = shutdown_rx.clone();
    let remote_addr = *config.remote_listen_addr();
    let log_level = *config.log_level();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server(remote_addr, log_level, http_state, http_rx).await {
            tracing::error!("HTTP server error: {}", e);
        }
    });
    handles.push(http_handle);

    // Start Iroh router
    let iroh_state = state.clone();
    let iroh_rx = shutdown_rx.clone();
    let iroh_handle = tokio::spawn(async move {
        if let Err(e) = iroh_router(iroh_state, iroh_rx).await {
            tracing::error!("Iroh router error: {}", e);
        }
    });
    handles.push(iroh_handle);

    // Start event listener
    let event_state = state.clone();
    let mut event_rx = shutdown_rx.clone();
    let tracker_handle = tokio::spawn(async move {
        let tracker_service = event_state.tracker_service();
        
        // Start the tracker service
        if let Err(e) = tracker_service.start_listening().await {
            tracing::error!("Tracker service error: {}", e);
            return;
        }
        
        // Keep the task alive until shutdown signal
        let _ = event_rx.changed().await;
        tracing::info!("Shutting down tracker service");
    });
    handles.push(tracker_handle);

    let _ = graceful_waiter.await;

    if timeout(FINAL_SHUTDOWN_TIMEOUT, join_all(handles))
        .await
        .is_err()
    {
        tracing::error!(
            "Failed to shut down within {} seconds",
            FINAL_SHUTDOWN_TIMEOUT.as_secs()
        );
        std::process::exit(4);
    }
}
