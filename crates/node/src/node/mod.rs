use std::time::Duration;

use futures::future::join_all;
use tokio::time::timeout;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const FINAL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

use crate::config::Config;

mod eth;
mod http;
mod iroh;
mod state;
mod tracker;
mod utils;

use http::http_server;
use iroh::router as iroh_router;
pub use iroh::create_ephemeral_endpoint;
pub use state::State;

pub struct Node;

impl Node {
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

        let (graceful_waiter, shutdown_rx) = utils::graceful_shutdown_blocker();

        // Create the app state with shutdown receiver
        let state = match State::from_config(config, shutdown_rx.clone()).await {
            Ok(state) => state,
            Err(e) => {
                eprintln!("error creating server state: {}", e);
                std::process::exit(3);
            }
        };

        let node_id = state.iroh_node_id();
        tracing::info!("managing iroh node id: {}", node_id);
        let address = state.eth_address();
        tracing::info!("managing eth address: {}", address);

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
        let iroh_endpoint = state.endpoint().clone();
        let blobs_service = state.blobs_service().clone();
        let iroh_rx = shutdown_rx.clone();
        let iroh_handle = tokio::spawn(async move {
            if let Err(e) = iroh_router(iroh_endpoint, blobs_service, iroh_rx).await {
                tracing::error!("Iroh router error: {}", e);
            }
        });
        handles.push(iroh_handle);

        // Start factory event listener
        let tracker = state.tracker().clone();
        let mut event_rx = shutdown_rx.clone();
        let factory_events_handle = tokio::spawn(async move {
            // Start the factory event listener
            if let Err(e) = tracker.start_event_listeners().await {
                tracing::error!("Factory contract event listener error: {}", e);
                return;
            }

            // Keep the task alive until shutdown signal
            let _ = event_rx.changed().await;
            tracing::info!("Shutting down factory contract event listener");
        });
        handles.push(factory_events_handle);

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
}
