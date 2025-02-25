use iroh::discovery::pkarr::dht::DhtDiscovery;
use iroh::Endpoint;
use iroh::NodeId;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;

use crate::app::{Config, ConfigError};
use crate::server::services::{BlobService, TrackerService};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ServerState {
    node_id: Option<NodeId>,
    endpoint: Arc<Endpoint>,
    blob_service: BlobService,
    tracker_service: TrackerService,
}

#[derive(Debug, thiserror::Error)]
pub enum ServerStateSetupError {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
}

impl ServerState {
    pub async fn from_config(config: &Config) -> Result<Self, ServerStateSetupError> {
        // Get the node ID from config during initialization
        let node_id = config.node_id().ok();

        // Create endpoint first
        let endpoint = Self::create_endpoint_static(config).await?;
        let endpoint_arc = Arc::new(endpoint);

        // Create services
        let blob_service = BlobService::new(config.blobs_path(), endpoint_arc.as_ref().clone())
            .await
            .map_err(ServerStateSetupError::Default)?;
            
        let tracker_service = TrackerService::new(config.tracker_path(), endpoint_arc.as_ref().clone())
            .await
            .map_err(ServerStateSetupError::Default)?;

        // Create state with all components
        let state = Self {
            node_id,
            endpoint: endpoint_arc,
            blob_service,
            tracker_service,
        };

        // Wait for relay region assignment
        state.await_relay_region().await?;

        Ok(state)
    }

    // Static version that doesn't require &mut self
    async fn create_endpoint_static(config: &Config) -> Result<Endpoint, ServerStateSetupError> {
        // Use the configured endpoint listen address instead of letting the system choose
        let addr = *config.endpoint_listen_addr();

        // Create DHT discovery with our secret key for P2P discovery
        let mainline_discovery = DhtDiscovery::builder()
            .secret_key(config.key()?)
            .build()
            .map_err(ServerStateSetupError::Default)?;

        // Convert the SocketAddr to a SocketAddrV4
        let addr_v4 = SocketAddrV4::new(
            addr.ip()
                .to_string()
                .parse::<Ipv4Addr>()
                .unwrap_or(Ipv4Addr::UNSPECIFIED),
            addr.port(),
        );

        // Create the endpoint with our key and discovery
        let endpoint = Endpoint::builder()
            .secret_key(config.key()?)
            .discovery(Box::new(mainline_discovery))
            .bind_addr_v4(addr_v4) // Pass the SocketAddrV4 directly
            .bind()
            .await
            .map_err(ServerStateSetupError::Default)?;

        Ok(endpoint)
    }

    // Helper to wait for DERP relay assignment (optional)
    pub async fn await_relay_region(&self) -> Result<(), ServerStateSetupError> {
        let t0 = std::time::Instant::now();
        loop {
            let addr = self.endpoint
                .node_addr()
                .await
                .map_err(ServerStateSetupError::Default)?;

            if addr.relay_url().is_some() {
                break;
            }
            if t0.elapsed() > std::time::Duration::from_secs(10) {
                return Err(ServerStateSetupError::Default(anyhow::anyhow!(
                    "timeout waiting for DERP region"
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        Ok(())
    }

    pub fn node_id(&self) -> Result<NodeId, ConfigError> {
        match self.node_id {
            Some(id) => Ok(id),
            None => Err(ConfigError::MissingConfig),
        }
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn blob_service(&self) -> &BlobService {
        &self.blob_service
    }

    pub fn tracker_service(&self) -> &TrackerService {
        &self.tracker_service
    }
}
