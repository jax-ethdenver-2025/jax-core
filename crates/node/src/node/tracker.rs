use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::str::FromStr;

use alloy::primitives::Address;
use anyhow::Result;
use iroh::NodeId;
use iroh_blobs::get::Stats;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{Hash, HashAndFormat};
use tokio::sync::RwLock;
use tokio::sync::watch;
use url::Url;
use alloy::signers::local::PrivateKeySigner;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::node::eth::contracts::{FactoryContract, PoolContract, FactoryEvent, PoolEvent};

use super::create_ephemeral_endpoint;
use super::iroh::probe_complete;
use super::iroh::BlobsService;

// NOTE (amiller68): prolly makes no sense to hash on both the
//  address and hash, but im not sure what else to do here
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PoolKey {
    pub hash: Hash,
    pub address: Address,
}

/// Simple in-memory store for network state
#[derive(Clone)]
pub struct Tracker {
    // just a map of pool key to the original node id
    pools: Arc<RwLock<HashMap<PoolKey, NodeId>>>,
    // map of pool key to set of nodes peering on this pool
    pool_peers: Arc<RwLock<HashMap<PoolKey, HashSet<NodeId>>>>,
    // Shutdown signal
    shutdown_rx: watch::Receiver<()>,
    // Ethereum connection details
    eth_ws_url: Arc<Url>,
    eth_private_key: Arc<PrivateKeySigner>,
    // Track active pool listeners
    pool_listeners: Arc<RwLock<HashSet<Address>>>,
    // Factory contract
    factory_contract: Arc<RwLock<Option<FactoryContract>>>,
    // Channels for contract events
    factory_event_rx: Arc<Mutex<Option<mpsc::Receiver<FactoryEvent>>>>,
    factory_event_tx: mpsc::Sender<FactoryEvent>,
    pool_event_rx: Arc<Mutex<Option<mpsc::Receiver<PoolEvent>>>>,
    #[allow(dead_code)]
    pool_event_tx: mpsc::Sender<PoolEvent>,
    blobs_service: Arc<BlobsService>,
}

impl Tracker {
    /// Create a new tracker service
    pub fn new(
        shutdown_rx: watch::Receiver<()>, 
        eth_ws_url: Url,
        eth_private_key: PrivateKeySigner,
        blobs_service: BlobsService
    ) -> Result<Self> {
        let (factory_event_tx, factory_event_rx) = mpsc::channel(100);
        let (pool_event_tx, pool_event_rx) = mpsc::channel(100);
        
        Ok(Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            pool_peers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_rx,
            eth_ws_url: Arc::new(eth_ws_url),
            eth_private_key: Arc::new(eth_private_key),
            pool_listeners: Arc::new(RwLock::new(HashSet::new())),
            factory_contract: Arc::new(RwLock::new(None)),
            factory_event_rx: Arc::new(Mutex::new(Some(factory_event_rx))),
            factory_event_tx,
            pool_event_rx: Arc::new(Mutex::new(Some(pool_event_rx))),
            pool_event_tx,
            blobs_service: Arc::new(blobs_service),
        })
    }

    /// Initialize the factory contract
    pub async fn init_factory(&self, factory_address: &Address) -> Result<()> {
        let factory = FactoryContract::new(
            factory_address,
            &self.eth_ws_url,
            &self.eth_private_key,
            self.factory_event_tx.clone(),
        ).await?;
        
        *self.factory_contract.write().await = Some(factory);
        Ok(())
    }

    /// Start listening for events from contracts
    pub async fn start_event_listeners(&self) -> Result<()> {
        // Start factory listener
        if let Some(factory) = self.factory_contract.read().await.as_ref() {
            factory.listen_events(self.shutdown_rx.clone()).await?;
        }
        
        // Start event processor
        let mut factory_rx = self.factory_event_rx.lock().await.take()
            .expect("Factory event receiver should be available");
        let mut pool_rx = self.pool_event_rx.lock().await.take()
            .expect("Pool event receiver should be available");
        let tracker = self.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = factory_rx.recv() => {
                        match event {
                            FactoryEvent::PoolCreated { pool_address, hash, node_id } => {
                                if let (Ok(hash), Ok(node_id)) = (
                                    iroh_blobs::Hash::from_str(&hash), 
                                    iroh::NodeId::from_str(&node_id)
                                ) {
                                    let key = PoolKey { hash, address: pool_address };
                                    // TODO (amiller68): handle errors here
                                    tracker.add_pool(key, node_id).await.expect("failed to add pool");
                                }
                            }
                        }
                    }
                    Some(event) = pool_rx.recv() => {
                        match event {
                            PoolEvent::PeerAdded { pool_address, hash, node_id } => {
                                if let Ok(node_id) = iroh::NodeId::from_str(&node_id) {
                                    let key = PoolKey { hash, address: pool_address };
                                    tracker.add_pool_peer(key, node_id).await;
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Shutting down event listeners");
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn create_pool(&self, hash: Hash, node_id: NodeId) -> Result<()> {
        if let Some(factory) = self.factory_contract.read().await.as_ref() {
            factory.create_pool(hash, node_id).await?;
        }
        Ok(())
    }

    pub async fn add_pool(&self, key: PoolKey, node_id: NodeId) -> Result<()> {
        self.pools.write().await.insert(key.clone(), node_id);

        // Create ticket and download blob
        let ticket = BlobTicket::new(
            node_id.into(),
            key.hash,
            iroh_blobs::BlobFormat::Raw
        ).expect("valid ticket");
        
        if let Err(e) = self.blobs_service.download_blob(&ticket).await {
            tracing::warn!("Failed to download blob for pool {}: {}", key.address, e);
        }

        // just enter the pool here, not sure if thats best
        self.enter_pool(key.clone(), node_id).await?;
        
        // Check if we already have a listener for this pool
        let mut listeners = self.pool_listeners.write().await;
        if !listeners.contains(&key.address) {
            // Create a new pool contract and start listening
            if let Ok(pool_contract) = PoolContract::new(
                key.address,
                &self.eth_ws_url,
                &self.eth_private_key,
                self,
            ).await {
                if let Ok(_) = pool_contract.listen_events(key.hash, self.shutdown_rx.clone()).await {
                    listeners.insert(key.address);
                    tracing::info!("Started listener for pool {} with hash {}", key.address, key.hash);
                }
            }
        }
        Ok(())
    }

    pub async fn add_pool_peer(&self, key: PoolKey, node_id: NodeId) {
        self.pool_peers.write().await.entry(key).or_insert_with(HashSet::new).insert(node_id);
    }

    pub async fn get_peers_for_hash(&self, hash: Hash) -> Result<Vec<NodeId>> {
        let map = self.pool_peers.read().await;
        let mut nodes = Vec::new();

        for (k, node_set) in map.iter() {
            if k.hash == hash {
                nodes.extend(node_set.iter().cloned());
            }
        }

        Ok(nodes)
    }

    /// Get global known peers for a given hash, regardless of pool
    pub async fn get_pool_peers(&self, key: PoolKey) -> Result<Vec<NodeId>> {
        let map = self.pool_peers.read().await;
        let mut nodes = Vec::new();

        // Look for this hash with any format
        for (k, node_set) in map.iter() {
            if k.hash == key.hash && k.address == key.address {
                nodes.extend(node_set.iter().cloned());
            }
        }

        Ok(nodes)
    }

    pub async fn enter_pool(&self, key: PoolKey, node_id: NodeId) -> Result<()> {
        // Check if the pool exists
        let pools = self.pools.read().await;
        if !pools.contains_key(&key) {
            return Err(anyhow::anyhow!("Pool does not exist: {}", key.address));
        }
        
        // Create a pool contract and enter the pool
        let pool_contract = PoolContract::new(
            key.address,
            &self.eth_ws_url,
            &self.eth_private_key,
            self,
        ).await?;
        
        // Enter the pool
        pool_contract.enter_pool(node_id).await?;
        
        // Add the peer to our local tracking
        self.add_pool_peer(key, node_id).await;
        
        Ok(())
    }

    pub async fn probe_node(ticket: BlobTicket) -> Result<Stats> {
        let ephemeral_endpoint = create_ephemeral_endpoint().await;
        let hash_and_format = HashAndFormat {
            hash: ticket.hash(),
            format: ticket.format(),
        };
        let stats = probe_complete(&ephemeral_endpoint, &ticket.node_addr().node_id, &hash_and_format).await?;
        Ok(stats)
    }
}
