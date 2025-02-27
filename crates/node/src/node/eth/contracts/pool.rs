use std::sync::Arc;

use alloy::{
    eips::BlockNumberOrTag,
    network::EthereumWallet,
    primitives::{Address, Log},
    providers::ProviderBuilder,
    providers::{Provider, WsConnect},
    rpc::types::Filter,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use futures_util::StreamExt;
use iroh::NodeId;
use tokio::sync::{Mutex, watch};
use url::Url;

use crate::node::tracker::Tracker;

// Define the PeerAdded event
sol!(
    event PeerAdded(string nodeId);
);

// TODO: make this use the sol file
// Define the RewardPool contract interface
sol! {
    #[sol(rpc)]
    contract RewardPool {
        function enterPool(string memory nodeId) external;
    }
}

/// Simple in-memory service that tracks content announcements
#[derive(Clone)]
pub struct PoolContract {
    ws_url: Url,
    private_key: PrivateKeySigner,
    address: Address,
    provider: Arc<Mutex<Arc<dyn Provider>>>,
    tracker: Tracker,
}

// Define event for internal communication
#[derive(Debug, Clone)]
pub enum PoolEvent {
    #[allow(dead_code)]
    PeerAdded {
        pool_address: Address,
        hash: iroh_blobs::Hash,
        node_id: String,
    }
}

impl PoolContract {
    /// Create a new tracker service
    pub async fn new(
        address: Address,
        ws_url: &Url,
        private_key: &PrivateKeySigner,
        tracker: &Tracker,
    ) -> Result<Self> {
        let ws = WsConnect::new(ws_url.as_str());
        let provider = Arc::new(
            ProviderBuilder::new()
                .with_chain(alloy_chains::NamedChain::AnvilHardhat)
                .wallet(EthereumWallet::from(private_key.clone()))
                .on_ws(ws)
                .await?,
        );
        // We're ignoring the data_dir for now - this is a pure in-memory implementation
        Ok(Self {
            address,
            ws_url: ws_url.clone(),
            private_key: private_key.clone(),
            provider: Arc::new(Mutex::new(provider)),
            tracker: tracker.clone(),
        })
    }

    // TODO: create a pool

    // TODO: get this hooked up to handlers
    pub async fn listen_events(&self, hash: iroh_blobs::Hash, shutdown_rx: watch::Receiver<()>) -> Result<()> {
        let filter = Filter::new()
            .address(self.address)
            .from_block(BlockNumberOrTag::Latest);

        let provider = self.provider.lock().await;
        let watch = provider.subscribe_logs(&filter).await?;
        let mut stream = watch.into_stream();

        let provider_clone = provider.clone();
        let tracker = self.tracker.clone();
        let pool_address = self.address;
        let pool_hash = hash;
        let mut shutdown = shutdown_rx;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(log) = stream.next() => {
                        let primitive_log = Log::from(log);
                        if let Ok(event) = PeerAdded::decode_log(&primitive_log, true) {
                            let node_id = event.nodeId.clone();
                            if let Ok(node_id) = node_id.parse::<NodeId>() {
                                // Create a pool key and add the peer
                                let key = crate::node::tracker::PoolKey {
                                    hash: pool_hash,
                                    address: pool_address,
                                };
                                tracker.add_pool_peer(key, node_id).await;
                                tracing::info!("Added peer {} to pool {}", node_id, pool_address);
                            }
                        }
                    }
                    _ = shutdown.changed() => {
                        tracing::info!("Shutting down pool contract listener for {}", pool_address);
                        break;
                    }
                }
            }
            let _provider = provider_clone;
        });

        Ok(())
    }

    pub async fn enter_pool(&self, node_id: NodeId) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let tx = contract.enterPool(node_id.to_string()).send().await?;
        let _receipt = tx.watch().await?;
        Ok(())
    }
}
