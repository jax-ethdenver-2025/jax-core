use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::Arc;
use std::str::FromStr;

use alloy::primitives::Address;
use anyhow::Result;
use async_trait::async_trait;
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

use crate::node::eth::contracts::{FactoryContract, PoolContract, FactoryEvent, PoolEvent, get_historical_peers};

use super::create_ephemeral_endpoint;
use super::iroh::probe_complete;
use super::iroh::BlobsService;

use jax_eigen_trust::{EigenTrust, TrustFetcher};

// NOTE (amiller68): prolly makes no sense to hash on both the
//  address and hash, but im not sure what else to do here
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolKey {
    pub hash: Hash,
    pub address: Address,
}

/// Simple in-memory store for network state
#[derive(Clone)]
pub struct Tracker {
    // Track all known pools
    pools: Arc<RwLock<HashSet<PoolKey>>>,
    // Track original creators of pools
    pool_creators: Arc<RwLock<HashMap<PoolKey, NodeId>>>,
    // Per-pool trust tracking
    pool_trust: Arc<RwLock<HashMap<PoolKey, EigenTrust<NetworkTrustFetcher>>>>,
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
    current_node_id: NodeId,
}

// Simplified NetworkTrustFetcher for per-pool trust
pub struct NetworkTrustFetcher {
    pool_key: PoolKey,
    peers: Arc<RwLock<HashSet<NodeId>>>,
    // Track successful/failed interactions
    interactions: Arc<RwLock<HashMap<(NodeId, NodeId), (u64, u64)>>>, // (successes, failures)
    eth_ws_url: Arc<Url>,
}

impl NetworkTrustFetcher {
    pub fn new(pool_key: PoolKey, eth_ws_url: Arc<Url>) -> Self {
        Self {
            pool_key,
            peers: Arc::new(RwLock::new(HashSet::new())),
            interactions: Arc::new(RwLock::new(HashMap::new())),
            eth_ws_url,
        }
    }

    pub async fn record_interaction(&self, from: NodeId, to: NodeId, success: bool) {
        let mut interactions = self.interactions.write().await;
        let (successes, failures) = interactions.entry((from, to)).or_insert((0, 0));
        if success {
            *successes += 1;
        } else {
            *failures += 1;
        }
    }

    pub async fn add_peer(&self, peer: NodeId) {
        self.peers.write().await.insert(peer);
    }

    async fn get_historical_peers(&self) -> Result<HashSet<NodeId>> {
        get_historical_peers(self.pool_key.address, &self.eth_ws_url).await
    }
}

#[async_trait]
impl TrustFetcher for NetworkTrustFetcher {
    type NodeId = NodeId;
    
    async fn fetch_trust(&self, i: &NodeId, j: &NodeId) -> Result<f64> {
        let interactions = self.interactions.read().await;
        
        // Calculate trust based on successful vs total interactions
        if let Some((successes, failures)) = interactions.get(&(*i, *j)) {
            let total = *successes as f64 + *failures as f64;
            if total > 0.0 {
                Ok(*successes as f64 / total)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }
    
    async fn discover_peers(&self, _: &NodeId) -> Result<HashSet<NodeId>> {
        let current_peers = self.peers.read().await.clone();
        
        if let Ok(historical_peers) = self.get_historical_peers().await {
            let mut all_peers = current_peers;
            all_peers.extend(historical_peers);
            return Ok(all_peers);
        }
        
        Ok(current_peers)
    }
}

// Add this enum to track probe outcomes
#[derive(Debug)]
pub enum ProbeResult {
    Success(Stats),
    Timeout(std::time::Duration),
    Error(anyhow::Error),
}

impl Tracker {
    /// Create a new tracker service
    pub fn new(
        shutdown_rx: watch::Receiver<()>, 
        eth_ws_url: Url,
        iroh_node_id: NodeId,
        eth_private_key: PrivateKeySigner,
        blobs_service: BlobsService
    ) -> Result<Self> {
        let (factory_event_tx, factory_event_rx) = mpsc::channel(100);
        let (pool_event_tx, pool_event_rx) = mpsc::channel(100);
        
        let tracker = Self {
            pools: Arc::new(RwLock::new(HashSet::new())),
            pool_creators: Arc::new(RwLock::new(HashMap::new())),
            pool_trust: Arc::new(RwLock::new(HashMap::new())),
            shutdown_rx: shutdown_rx.clone(),
            eth_ws_url: Arc::new(eth_ws_url),
            eth_private_key: Arc::new(eth_private_key),
            pool_listeners: Arc::new(RwLock::new(HashSet::new())),
            factory_contract: Arc::new(RwLock::new(None)),
            factory_event_rx: Arc::new(Mutex::new(Some(factory_event_rx))),
            factory_event_tx,
            pool_event_rx: Arc::new(Mutex::new(Some(pool_event_rx))),
            pool_event_tx,
            blobs_service: Arc::new(blobs_service),
            current_node_id: iroh_node_id,
        };

        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            tracker_clone.start_background_jobs().await;
        });

        Ok(tracker)
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
        tracing::info!("Creating pool {} with creator {}", hash, node_id);
        if let Some(factory) = self.factory_contract.read().await.as_ref() {
            factory.create_pool(hash, node_id).await?;
        }
        Ok(())
    }

    pub async fn add_pool(&self, key: PoolKey, creator: NodeId) -> Result<()> {
        tracing::info!("Adding pool {} with creator {}", key.address, creator);
        self.pools.write().await.insert(key.clone());
        self.pool_creators.write().await.insert(key.clone(), creator);
        
        // Create new EigenTrust instance for this pool
        let network_fetcher = NetworkTrustFetcher::new(key.clone(), self.eth_ws_url.clone());
        let pool_eigen = EigenTrust::new(creator, network_fetcher);
        self.pool_trust.write().await.insert(key.clone(), pool_eigen);

        // Create ticket and download blob
        let ticket = BlobTicket::new(
            creator.into(),
            key.hash,
            iroh_blobs::BlobFormat::Raw
        ).expect("valid ticket");
        
        if let Err(e) = self.blobs_service.download_blob(&ticket).await {
            tracing::warn!("Failed to download blob for pool {}: {}", key.address, e);
        }

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

        // If this is the creator node, wait a moment to ensure contract is ready
        if creator == self.current_node_id {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        // Enter the pool
        self.enter_pool(key.clone(), self.current_node_id).await?;
        
        Ok(())
    }

    pub async fn add_pool_peer(&self, key: PoolKey, node_id: NodeId) {
        tracing::info!("Adding peer {} to pool {}", node_id, key.address);
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            // Add peer to the pool's trust network with zero initial trust
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher.add_peer(node_id).await;
                // Record initial interaction with zero trust
                fetcher.record_interaction(self.current_node_id, node_id, false).await;
            }
            // Set initial local trust to 0
            eigen.update_local_trust(node_id, 0.0, 1.0);
        }
    }

    pub async fn get_peers_for_hash(&self, hash: Hash) -> Result<Vec<NodeId>> {
        let pool_trust = self.pool_trust.read().await;
        let mut nodes = Vec::new();

        for (key, eigen) in pool_trust.iter() {
            if key.hash == hash {
                if let Some(fetcher) = eigen.get_fetcher() {
                    nodes.extend(fetcher.peers.read().await.iter().cloned());
                }
            }
        }

        Ok(nodes)
    }

    /// Get global known peers for a given hash, regardless of pool
    pub async fn get_pool_peers(&self, key: PoolKey) -> Result<Vec<NodeId>> {
        let pool_trust = self.pool_trust.read().await;
        if let Some(eigen) = pool_trust.get(&key) {
            if let Some(fetcher) = eigen.get_fetcher() {
                return Ok(fetcher.peers.read().await.iter().cloned().collect());
            }
        }
        Ok(Vec::new())
    }

    pub async fn enter_pool(&self, key: PoolKey, node_id: NodeId) -> Result<()> {
        // Check if the pool exists
        let pools = self.pools.read().await;
        if !pools.contains(&key) {
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
        
        // Add a timeout of 10 seconds
        let probe_future = probe_complete(&ephemeral_endpoint, &ticket.node_addr().node_id, &hash_and_format);
        match tokio::time::timeout(std::time::Duration::from_secs(2), probe_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Probe timed out after 10 seconds"))
        }
    }

    /// Record successful download from a peer in a specific pool
    pub async fn record_successful_download(&self, key: PoolKey, provider: NodeId) -> Result<()> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher.record_interaction(self.current_node_id, provider, true).await;
            }
            eigen.update_local_trust(provider, 1.0, 0.1);
        }
        Ok(())
    }

    /// Record failed download from a peer in a specific pool
    pub async fn record_failed_download(&self, key: PoolKey, provider: NodeId) -> Result<()> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher.record_interaction(self.current_node_id, provider, false).await;
            }
            eigen.update_local_trust(provider, 0.0, 0.1);
        }
        Ok(())
    }

    pub async fn get_pool_trust(&self, key: &PoolKey) -> Result<Option<HashMap<NodeId, f64>>> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(key) {
            Ok(Some(eigen.compute_global_trust().await?))
        } else {
            Ok(None)
        }
    }

    /// Get all known pools with trust scores for their peers
    pub async fn list_pools_with_trust(&self) -> Result<BTreeMap<PoolKey, HashMap<NodeId, f64>>> {
        let mut result = BTreeMap::new();
        let mut pool_trust = self.pool_trust.write().await;
        
        for (key, eigen) in pool_trust.iter_mut() {
            result.insert(key.clone(), eigen.compute_global_trust().await?);
        }
        
        Ok(result)
    }

    // Modify the update_local_trust method to be more aggressive with failures
    pub async fn update_local_trust(&self, key: PoolKey, node_id: NodeId, success: bool) -> Result<()> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            // Update trust fetcher interactions
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher.record_interaction(self.current_node_id, node_id, success).await;
            }
            
            // Make trust changes more dramatic
            // Success = high trust (1.0)
            // Failure = very low trust (0.0)
            // Increase weight to 0.5 for faster trust changes
            let trust_value = if success { 1.0 } else { 0.0 };
            eigen.update_local_trust(node_id, trust_value, 0.5); // Increased weight from 0.1 to 0.5
        }
        Ok(())
    }

    // Modify probe_and_update_trust to be stricter about timeouts
    pub async fn probe_and_update_trust(&self, key: PoolKey, node_id: NodeId) -> Result<ProbeResult> {
        let ticket = BlobTicket::new(
            node_id.into(),
            key.hash,
            iroh_blobs::BlobFormat::Raw
        ).expect("valid ticket");

        // Reduce timeout threshold to 2 seconds
        let probe_result = match Self::probe_node(ticket).await {
            Ok(stats) => {
                // More aggressive timeout threshold - now 2 seconds instead of 10
                if stats.elapsed.as_millis() < 2000 {
                    ProbeResult::Success(stats)
                } else {
                    ProbeResult::Timeout(stats.elapsed)
                }
            },
            Err(e) => ProbeResult::Error(e),
        };

        // Update trust based on probe result
        let success = matches!(probe_result, ProbeResult::Success(_));
        
        // For timeouts and errors, update trust as a failure
        if !success {
            // Call update_local_trust with failure
            self.update_local_trust(key.clone(), node_id, false).await?;
            
            // Additional penalty for complete failures (errors)
            if matches!(probe_result, ProbeResult::Error(_)) {
                // Apply an extra penalty by updating trust again
                self.update_local_trust(key, node_id, false).await?;
            }
        } else {
            self.update_local_trust(key, node_id, true).await?;
        }

        Ok(probe_result)
    }

    // Modify get_trust_for_hash to use the same trust computation as list_pools_with_trust
    pub async fn get_trust_for_hash(&self, hash: Hash) -> Result<HashMap<NodeId, f64>> {
        let mut combined_trust = HashMap::new();
        let mut pool_trust = self.pool_trust.write().await;

        for (key, eigen) in pool_trust.iter_mut() {
            if key.hash == hash {
                // Use compute_global_trust directly instead of averaging
                let trust_scores = eigen.compute_global_trust().await?;
                for (node_id, score) in trust_scores {
                    combined_trust.insert(node_id, score);
                }
                // Break after first pool since we want the exact trust scores
                break;
            }
        }

        Ok(combined_trust)
    }

    // Add this method to probe all nodes in a pool
    pub async fn probe_pool(&self, key: PoolKey) -> Result<Vec<(NodeId, ProbeResult)>> {
        tracing::info!("Probing pool {}", key.address);
        let mut results = Vec::new();
        let peers = self.get_pool_peers(key.clone()).await?;

        tracing::info!("Pool {} has {} peers", key.address, peers.len());
        
        for node_id in peers {
            tracing::info!("Probing node {}", node_id);
            match self.probe_and_update_trust(key.clone(), node_id).await {
                Ok(probe_result) => {
                    tracing::info!("Probed node {} with result {:?}", node_id, probe_result);
                    results.push((node_id, probe_result));
                }
                Err(e) => {
                    tracing::warn!("Failed to probe node {}: {}", node_id, e);
                    self.update_local_trust(key.clone(), node_id, false).await?;
                    results.push((node_id, ProbeResult::Error(e)));
                }
            }
        }
        
        Ok(results)
    }

    // Add convenience method to probe all pools for a hash
    pub async fn probe_hash(&self, hash: Hash) -> Result<Vec<(PoolKey, Vec<(NodeId, ProbeResult)>)>> {
        let mut results = Vec::new();
        let pools = self.pools.read().await;
        
        for key in pools.iter() {
            if key.hash == hash {
                match self.probe_pool(key.clone()).await {
                    Ok(pool_results) => results.push((key.clone(), pool_results)),
                    Err(e) => tracing::warn!("Failed to probe pool {}: {}", key.address, e),
                }
            }
        }
        
        Ok(results)
    }

    pub async fn get_pool_creator(&self, key: &PoolKey) -> Option<NodeId> {
        self.pool_creators.read().await.get(key).cloned()
    }

    /// Start background jobs for pool maintenance
    pub async fn start_background_jobs(&self) {
        let tracker = self.clone();
        
        // Try initial bootstrap with retries
        for i in 0..3 {
            match tracker.bootstrap().await {
                Ok(_) => {
                    tracing::info!("Successfully bootstrapped tracker");
                    break;
                }
                Err(e) => {
                    tracing::warn!("Bootstrap attempt {} failed: {}", i + 1, e);
                    if i < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }

        let mut shutdown_rx = self.shutdown_rx.clone();
        
        // Spawn background task
        tokio::spawn(async move {
            let update_interval = tokio::time::Duration::from_secs(10); // Increased from 10s
            let mut interval = tokio::time::interval(update_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        tracing::debug!("Starting periodic pool update");
                        if let Err(e) = tracker.update_all_pools().await {
                            tracing::warn!("Failed to update pools: {}", e);
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("Shutting down pool maintenance jobs");
                        break;
                    }
                }
            }
        });
    }

    /// Update all pools - fetch new peers and recalculate trust scores
    async fn update_all_pools(&self) -> Result<()> {
        let pools = self.pools.read().await.clone();
        let mut updated_count = 0;
        
        for pool_key in pools {
            match self.probe_pool(pool_key.clone()).await {
                Ok(results) => {
                    let mut responsive_peers = 0;
                    let total_peers = results.len();

                    for (node_id, result) in &results {
                        match result {
                            ProbeResult::Success(_) => {
                                responsive_peers += 1;
                            }
                            ProbeResult::Timeout(_) | ProbeResult::Error(_) => {
                                // Apply extra penalty for unresponsive peers
                                self.update_local_trust(pool_key.clone(), *node_id, false).await?;
                            }
                        }
                    }

                    tracing::info!(
                        "Updated pool {} with {}/{} responsive peers", 
                        pool_key.address,
                        responsive_peers,
                        total_peers
                    );
                    updated_count += 1;

                    // Check if we're already in the pool on-chain before trying to join
                    match get_historical_peers(pool_key.address, &self.eth_ws_url).await {
                        Ok(chain_peers) => {
                            if !chain_peers.contains(&self.current_node_id) {
                                match self.enter_pool(pool_key.clone(), self.current_node_id).await {
                                    Ok(_) => tracing::info!("Successfully joined pool {}", pool_key.address),
                                    Err(e) => tracing::warn!("Failed to join pool {}: {}", pool_key.address, e)
                                }
                            } else {
                                tracing::debug!("Already in pool {}, skipping join", pool_key.address);
                            }
                        }
                        Err(e) => tracing::warn!("Failed to check pool membership for {}: {}", pool_key.address, e)
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to update pool {}: {}", 
                        pool_key.address, 
                        e
                    );
                }
            }

            // Fetch historical peers from chain
            match get_historical_peers(pool_key.address, &self.eth_ws_url).await {
                Ok(historical_peers) => {
                    for peer in historical_peers {
                        self.add_pool_peer(pool_key.clone(), peer).await;
                    }
                }
                Err(e) => tracing::warn!("Failed to get historical peers for {}: {}", pool_key.address, e)
            }
        }

        tracing::info!("Updated {} pools", updated_count);
        Ok(())
    }

    /// Bootstrap by discovering and joining available pools
    pub async fn bootstrap(&self) -> Result<()> {
        tracing::info!("Starting bootstrap process...");
        
        // Get factory contract
        let factory = self.factory_contract.read().await;
        let factory = factory.as_ref().ok_or_else(|| anyhow::anyhow!("Factory not initialized"))?;

        // Get all pools
        let pools = factory.get_all_pools().await?;
        tracing::info!("Found {} pools during bootstrap", pools.len());
        
        let mut joined_count = 0;
        for pool_address in pools {
            // Create pool contract to get metadata
            let pool_contract = match PoolContract::new(
                pool_address,
                &self.eth_ws_url,
                &self.eth_private_key,
                self,
            ).await {
                Ok(contract) => contract,
                Err(e) => {
                    tracing::warn!("Failed to create contract for pool {}: {}", pool_address, e);
                    continue;
                }
            };

            // Get pool metadata
            let (hash, originator) = match pool_contract.get_metadata().await {
                Ok(metadata) => metadata,
                Err(e) => {
                    tracing::warn!("Failed to get metadata for pool {}: {}", pool_address, e);
                    continue;
                }
            };
            
            let key = PoolKey {
                hash,
                address: pool_address,
            };

            // Get peers from pool
            let peers = pool_contract.get_peers().await?;
            tracing::debug!("Pool {} has {} peers", pool_address, peers.len());

            // Try to download from originator or peers
            let mut providers = vec![originator];
            providers.extend(peers);

            for provider in providers {
                let ticket = BlobTicket::new(
                    provider.into(),
                    hash,
                    iroh_blobs::BlobFormat::Raw
                ).expect("valid ticket");

                if let Ok(_) = self.blobs_service.download_blob(&ticket).await {
                    // Add pool and enter
                    if let Ok(_) = self.add_pool(key.clone(), originator).await {
                        joined_count += 1;
                        tracing::info!("Successfully joined pool {} during bootstrap", pool_address);
                    }
                    break;
                }
            }
        }

        tracing::info!("Bootstrap complete - joined {} pools", joined_count);
        Ok(())
    }
}
