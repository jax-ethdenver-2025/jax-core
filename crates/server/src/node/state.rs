use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use iroh::discovery::pkarr::dht::DhtDiscovery;
use iroh::Endpoint;
use iroh::NodeId;
use iroh::SecretKey;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{Config, ConfigError};

use super::iroh::{await_relay_region, create_endpoint, BlobsService};
use super::tracker::Tracker;

#[derive(Clone)]
pub struct State {
    iroh_secret_key: SecretKey,
    eth_signer: PrivateKeySigner,
    endpoint: Arc<Endpoint>,
    blobs_service: BlobsService,
    tracker: Tracker,
}

#[derive(Debug, thiserror::Error)]
pub enum StateSetupError {
    #[error("default error: {0}")]
    Default(#[from] anyhow::Error),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

impl State {
    pub async fn from_config(config: &Config) -> Result<Self, StateSetupError> {
        // set up our endpoint
        let endpoint_socket_addr = config.endpoint_listen_addr();
        let iroh_secret_key = config.iroh_secret_key()?;
        let _endpoint = create_endpoint(*endpoint_socket_addr, iroh_secret_key.clone()).await;
        let endpoint = Arc::new(_endpoint);
        // await making sure the endpoint is setup
        await_relay_region(endpoint.as_ref().clone()).await;

        // set up a blob service
        let blobs_path = config.blobs_path();
        let blobs_service = BlobsService::load(blobs_path, endpoint.as_ref().clone())
            .await
            .map_err(StateSetupError::Default)?;

        let tracker = Tracker::new().expect("valid tracker");

        // Create state with all components
        let state = Self {
            iroh_secret_key,
            eth_signer: config.eth_signer().expect("valid eth signer"),
            endpoint,
            blobs_service,
            tracker,
        };

        Ok(state)
    }

    pub fn iroh_node_id(&self) -> NodeId {
        self.iroh_secret_key.public()
    }

    pub fn eth_address(&self) -> Address {
        self.eth_signer.address()
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn blobs_service(&self) -> &BlobsService {
        &self.blobs_service
    }

    pub fn tracker_service(&self) -> &Tracker {
        &self.tracker
    }
}
