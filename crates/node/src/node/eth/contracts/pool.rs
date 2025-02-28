use std::collections::HashSet;
use std::{str::FromStr, sync::Arc};

use alloy::primitives::{Bytes, FixedBytes};
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
use ed25519::Signature as Ed25519Signature;
use futures_util::StreamExt;
use iroh::NodeId;
use tokio::sync::{watch, Mutex};
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
    struct Signature {
        bytes32 k;
        bytes32 r;
        bytes32 s;
        bytes m;
    }

    #[sol(rpc)]
    contract RewardPool {
        function enterPool(string memory nodeId, Signature memory signature) external;
        function getHash() external view returns (string memory);
        function getAllPeers() external view returns (string[] memory);
        function deposit() external payable;
        function setBountyPerEpoch(uint256 amount) external;
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
    iroh_signature: Ed25519Signature,
}

// Define event for internal communication
#[derive(Debug, Clone)]
pub enum PoolEvent {
    #[allow(dead_code)]
    PeerAdded {
        pool_address: Address,
        hash: iroh_blobs::Hash,
        node_id: String,
    },
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
            iroh_signature: tracker.iroh_signature.clone(),
        })
    }

    // TODO: create a pool

    // TODO: get this hooked up to handlers
    pub async fn listen_events(
        &self,
        hash: iroh_blobs::Hash,
        shutdown_rx: watch::Receiver<()>,
    ) -> Result<()> {
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

    pub async fn enter_pool(&self) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let iroh_signature = self.iroh_signature.clone();
        let node_id = self.tracker.current_node_id.clone();
        let k_bytes = self.tracker.current_node_id.as_bytes();
        let r_bytes = iroh_signature.r_bytes();
        let s_bytes = iroh_signature.s_bytes();
        let m_bytes = iroh_signature.to_bytes();
        let signature = Signature {
            k: FixedBytes::from_slice(k_bytes),
            r: FixedBytes::from_slice(r_bytes),
            s: FixedBytes::from_slice(s_bytes),
            m: Bytes::copy_from_slice(&m_bytes),
        };
        let tx = contract
            .enterPool(node_id.to_string(), signature)
            .send()
            .await?;
        let _receipt = tx.watch().await?;
        Ok(())
    }

    pub async fn get_hash(&self) -> Result<iroh_blobs::Hash> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let hash = contract.getHash().call().await?._0;
        let hash = iroh_blobs::Hash::from_str(&hash)?;
        Ok(hash)
    }

    pub async fn get_peers(&self) -> Result<Vec<NodeId>> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let peers = contract.getAllPeers().call().await?._0;
        let mut peer_set = HashSet::new();
        for peer in peers {
            if let Ok(node_id) = peer.parse::<NodeId>() {
                peer_set.insert(node_id);
            }
        }
        Ok(peer_set.into_iter().collect())
    }
}

// TODO: jank as hell to have this here
pub async fn get_peers(address: Address, ws_url: &Url) -> Result<HashSet<NodeId>> {
    let provider = ProviderBuilder::new()
        .with_chain(alloy_chains::NamedChain::AnvilHardhat)
        .on_ws(WsConnect::new(ws_url.as_str()))
        .await?;
    let contract = RewardPool::new(address, provider);
    let peers = contract.getAllPeers().call().await?._0;
    let mut peer_set = HashSet::new();
    for peer in peers {
        if let Ok(node_id) = peer.parse::<NodeId>() {
            peer_set.insert(node_id);
        }
    }
    Ok(peer_set)
}
