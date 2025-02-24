use axum::Router;
use tokio::sync::watch;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;

use crate::api;
use crate::app::{AppState, AppStateSetupError, Config};
use crate::health;

mod error_handlers;

const HEALTH_PREFIX: &str = "/_status";
const API_PREFIX: &str = "/api/v0";

pub async fn run(
    config: Config,
    state: AppState,
    mut shutdown_rx: watch::Receiver<()>,
) -> Result<(), HttpServerError> {
    let listen_addr = config.listen_addr();
    let log_level = *config.log_level();
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

    tracing::info!(addr = ?listen_addr, "server listening");
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    axum::serve(listener, root_router)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
        })
        .await?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error("an error occurred running the HTTP server: {0}")]
    ServingFailed(#[from] std::io::Error),

    #[error("state initialization failed: {0}")]
    StateInitializationFailed(#[from] AppStateSetupError),
}
