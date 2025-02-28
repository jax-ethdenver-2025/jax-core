use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use alloy::primitives::{Address, U256};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use async_trait::async_trait;
use ed25519::Signature;
use iroh::NodeId;
use iroh_blobs::get::Stats;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{Hash, HashAndFormat};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use url::Url;

use crate::node::eth::contracts::{
    get_peers, FactoryContract, FactoryEvent, PoolContract, PoolEvent,
};
use crate::node::iroh::BlobsService;

use super::create_ephemeral_endpoint;
use super::eth::get_address_balance;
use super::iroh::probe_complete;

use jax_eigen_trust::{EigenTrust, TrustFetcher};

// NOTE (amiller68): prolly makes no sense to hash on both the
//  address and hash, but im not sure what else to do here
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolKey {
    pub hash: Hash,
    pub address: Address,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolInfo {
    pub key: PoolKey,
    pub balance: U256,
}

impl PoolInfo {
    pub fn key(&self) -> &PoolKey {
        &self.key
    }
}

/// Simple in-memory store for network state
#[derive(Clone)]
pub struct Tracker {
    pub iroh_signature: Signature,
    // Track all known pools
    pools: Arc<RwLock<HashMap<PoolKey, U256>>>,
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
    factory_contract: Arc<RwLock<FactoryContract>>,
    // Channels for contract events
    factory_event_rx: Arc<Mutex<Option<mpsc::Receiver<FactoryEvent>>>>,
    pool_event_rx: Arc<Mutex<Option<mpsc::Receiver<PoolEvent>>>>,
    #[allow(dead_code)]
    pool_event_tx: mpsc::Sender<PoolEvent>,
    blobs_service: Arc<BlobsService>,
    pub current_node_id: NodeId,
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

    async fn get_peers(&self) -> Result<HashSet<NodeId>> {
        get_peers(self.pool_key.address, &self.eth_ws_url).await
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

        if let Ok(peers) = self.get_peers().await {
            let mut all_peers = current_peers;
            all_peers.extend(peers);
            return Ok(all_peers);
        }

        Ok(current_peers)
    }
}

// Add this enum to track probe outcomes
#[derive(Debug, Clone)]
pub enum ProbeResult {
    Success(Stats),
    Timeout(std::time::Duration),
    Error,
}

impl Tracker {
    /// Create a new tracker service
    pub async fn new(
        shutdown_rx: watch::Receiver<()>,
        eth_ws_url: Url,
        factory_address: &Address,
        iroh_node_id: NodeId,
        eth_private_key: PrivateKeySigner,
        blobs_service: BlobsService,
        iroh_signature: Signature,
    ) -> Result<Self> {
        let (factory_event_tx, factory_event_rx) = mpsc::channel(100);
        let (pool_event_tx, pool_event_rx) = mpsc::channel(100);

        let factory_contract = FactoryContract::new(
            factory_address,
            &eth_ws_url,
            &eth_private_key,
            factory_event_tx.clone(),
        )
        .await?;

        let tracker = Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            pool_trust: Arc::new(RwLock::new(HashMap::new())),
            shutdown_rx: shutdown_rx.clone(),
            eth_ws_url: Arc::new(eth_ws_url),
            eth_private_key: Arc::new(eth_private_key),
            pool_listeners: Arc::new(RwLock::new(HashSet::new())),
            factory_contract: Arc::new(RwLock::new(factory_contract)),
            factory_event_rx: Arc::new(Mutex::new(Some(factory_event_rx))),
            pool_event_rx: Arc::new(Mutex::new(Some(pool_event_rx))),
            pool_event_tx,
            blobs_service: Arc::new(blobs_service),
            current_node_id: iroh_node_id,
            iroh_signature,
        };

        let tracker_clone = tracker.clone();
        tokio::spawn(async move {
            tracker_clone.start_background_jobs().await;
        });

        Ok(tracker)
    }

    /// Start listening for events from contracts
    pub async fn start_event_listeners(&self) -> Result<()> {
        // Start factory listener
        let factory = self.factory_contract.read().await;
        factory.listen_events(self.shutdown_rx.clone()).await?;

        // Start event processor
        let mut factory_rx = self
            .factory_event_rx
            .lock()
            .await
            .take()
            .expect("Factory event receiver should be available");
        let mut pool_rx = self
            .pool_event_rx
            .lock()
            .await
            .take()
            .expect("Pool event receiver should be available");
        let tracker = self.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();

        // passively listen for new pools and peers
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = factory_rx.recv() => {
                        match event {
                            FactoryEvent::PoolCreated { pool_address, hash, balance } => {
                                    let key = PoolKey { hash, address: pool_address };
                                    // TODO (amiller68): handle errors here
                                    tracker.add_pool(key, balance).await.expect("failed to add pool");
                            }
                        }
                    }
                    // NOTE (amiller68): see not in eth::contracts::pool.rs, i don't think
                    //  we're even emitting these events
                    Some(event) = pool_rx.recv() => {
                        match event {
                            PoolEvent::PeerAdded { pool_address, hash, node_id } => {
                                if let Ok(node_id) = iroh::NodeId::from_str(&node_id) {
                                    let key = PoolKey { hash, address: pool_address };
                                    tracker.add_pool_peer(key, node_id).await;
                                }
                            },
                            // TODO: handle deposits i guess for now we'll update via polling
                            _ => {}
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

    pub async fn create_pool(&self, hash: Hash, value: Option<U256>) -> Result<()> {
        tracing::info!("Creating pool {}", hash);
        let factory = self.factory_contract.read().await;
        // TODO (amiller68): for some reason this seemed to be returning the
        //  wrong address -- we should be updating our local knowledge of pools here
        let _pool_address = factory.create_pool(hash, value).await?;
        // tracing::info!("Pool created at {}", pool_address);
        // self.add_pool(PoolKey { hash, address: pool_address }).await?;
        Ok(())
    }

    pub async fn set_pool_balance(&self, key: PoolKey, amount: U256) -> () {
        let mut pools = self.pools.write().await;
        pools.insert(key, amount);
        ()
    }

    pub async fn add_pool_deposit(&self, key: PoolKey, amount: U256) -> () {
        let mut pools = self.pools.write().await;
        let value = pools.get(&key).copied();
        if let Some(value) = value {
            pools.insert(key, amount + value);
        } else {
            tracing::warn!("node::tracker::: attempted to up deposit state of non-extant pool");
        }
        ()
    }

    /// NOTE (amiller68): this is a janky place to put this, but it's convenient
    ///  since the tracker has the ws url
    pub async fn get_address_balance(&self, address: Address) -> Result<U256> {
        let balance = get_address_balance(address, &self.eth_ws_url).await?;
        Ok(balance)
    }

    /// NOTE (amiller68): yet another way to get the balance of a pool (from anvil)
    pub async fn get_pool_balance_live(&self, key: PoolKey) -> Result<U256> {
        let balance = get_address_balance(key.address, &self.eth_ws_url).await?;
        Ok(balance)
    }

    pub async fn deposit_into_pool(&self, key: PoolKey, amount: U256) -> Result<()> {
        let pool_contract =
            PoolContract::new(key.address, &self.eth_ws_url, &self.eth_private_key, self).await?;
        pool_contract.deposit(amount).await?;
        let address = key.address;
        let balance = get_address_balance(address, &self.eth_ws_url).await?;
        // update local state -- this can just be incrementing the balance
        self.add_pool_deposit(key, balance).await;
        Ok(())
    }

    pub async fn add_pool(&self, key: PoolKey, balance: U256) -> Result<()> {
        // check if the pool already exists
        let mut pools = self.pools.write().await;
        let existing = pools.get(&key);
        if existing.is_some() {
            tracing::warn!("Pool already exists: {}", key.address);
            return Ok(());
        }
        pools.insert(key.clone(), balance);

        // Create new EigenTrust instance for this pool
        let network_fetcher = NetworkTrustFetcher::new(key.clone(), self.eth_ws_url.clone());
        let pool_eigen = EigenTrust::new(network_fetcher);
        self.pool_trust
            .write()
            .await
            .insert(key.clone(), pool_eigen);

        // Look up and add historical peers
        if let Ok(peers) = get_peers(key.address, &self.eth_ws_url).await {
            for peer in peers.clone() {
                self.add_pool_peer(key.clone(), peer).await;
            }
            tracing::info!(
                "Added {} historical peers for pool {}",
                peers.len(),
                key.address
            );
        }

        // Check if we already have a listener for this pool
        let mut listeners = self.pool_listeners.write().await;
        if !listeners.contains(&key.address) {
            // Create a new pool contract and start listening
            if let Ok(pool_contract) =
                PoolContract::new(key.address, &self.eth_ws_url, &self.eth_private_key, self).await
            {
                if let Ok(_) = pool_contract
                    .listen_events(key.hash, self.shutdown_rx.clone())
                    .await
                {
                    listeners.insert(key.address);
                    tracing::info!(
                        "Started listener for pool {} with hash {}",
                        key.address,
                        key.hash
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn add_pool_peer(&self, key: PoolKey, node_id: NodeId) {
        tracing::info!("Adding peer {} to pool {}", node_id, key.address);
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            // Add peer to the pool's trust network with zero initial trust
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher.add_peer(node_id).await;
                // Record initial interaction with zero trust
                fetcher
                    .record_interaction(self.current_node_id, node_id, false)
                    .await;
            }
            // Set initial local trust to 0
            eigen.update_local_trust(node_id, 0.0, 1.0);
        }
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

    pub async fn get_pool_balance(&self, key: PoolKey) -> Result<U256> {
        let pools = self.pools.read().await;
        let balance = pools.get(&key);
        if let Some(balance) = balance {
            Ok(*balance)
        } else {
            Err(anyhow::anyhow!("Pool does not exist: {}", key.address))
        }
    }

    pub async fn enter_pool(&self, key: PoolKey) -> Result<()> {
        // Check if the pool exists
        let pools = self.pools.read().await;
        if !pools.contains_key(&key) {
            return Err(anyhow::anyhow!("Pool does not exist: {}", key.address));
        }

        // Create a pool contract and enter the pool
        let pool_contract =
            PoolContract::new(key.address, &self.eth_ws_url, &self.eth_private_key, self).await?;

        // sanity check -- we should make sure we have the hash
        //  prior to entering the pool
        // see if you have the hash
        let stat = self.blobs_service.get_blob_stat(key.hash).await?;
        if !stat {
            return Err(anyhow::anyhow!("You don't have the hash: {}", key.hash));
        }

        // Enter the pool
        pool_contract.enter_pool().await?;

        // mark yourself as a peer
        self.add_pool_peer(key, self.current_node_id).await;

        Ok(())
    }

    pub async fn probe_node(ticket: BlobTicket) -> Result<Stats> {
        let ephemeral_endpoint = create_ephemeral_endpoint().await;
        let hash_and_format = HashAndFormat {
            hash: ticket.hash(),
            format: ticket.format(),
        };

        // Add a timeout of 10 seconds
        let probe_future = probe_complete(
            &ephemeral_endpoint,
            &ticket.node_addr().node_id,
            &hash_and_format,
        );
        match tokio::time::timeout(std::time::Duration::from_secs(2), probe_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Probe timed out after 2 seconds")),
        }
    }

    pub async fn get_pool_trust(&self, key: &PoolKey) -> Result<Option<HashMap<NodeId, f64>>> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(key) {
            Ok(Some(eigen.compute_global_trust().await?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_hash_trust(&self, hash: &Hash) -> Result<Option<HashMap<NodeId, f64>>> {
        // find the pool with this hash -- there should only be one
        let pools = self.pools.read().await;
        let pool = pools.iter().find(|(p, _)| p.hash == *hash);
        if let Some((pool, _)) = pool {
            match self.get_pool_trust(pool).await {
                Ok(Some(trust_scores)) => Ok(Some(trust_scores)),
                Ok(None) => Ok(None),
                Err(e) => {
                    tracing::warn!(
                        "Failed to get trust scores for pool {}: {}",
                        pool.address,
                        e
                    );
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn list_pools_with_trust(&self) -> Result<BTreeMap<PoolInfo, HashMap<NodeId, f64>>> {
        let mut result = BTreeMap::new();
        let pools = self.pools.read().await;
        let mut pool_trust = self.pool_trust.write().await;

        for (key, eigen) in pool_trust.iter_mut() {
            // NOTE (amiller68): we should be gauranteed that the pool exists
            let balance = pools.get(key).unwrap_or(&U256::ZERO);
            let pool_info = PoolInfo {
                key: key.clone(),
                balance: *balance,
            };
            result.insert(
                pool_info,
                eigen.compute_global_trust().await.unwrap_or_default(),
            );
        }

        Ok(result)
    }

    pub async fn update_local_trust(
        &self,
        key: PoolKey,
        node_id: NodeId,
        success: bool,
    ) -> Result<()> {
        if let Some(eigen) = self.pool_trust.write().await.get_mut(&key) {
            // Update trust fetcher interactions
            if let Some(fetcher) = eigen.get_fetcher_mut() {
                fetcher
                    .record_interaction(self.current_node_id, node_id, success)
                    .await;
            }
            // Make trust changes more dramatic
            // Success = high trust (1.0)
            // Failure = very low trust (0.0)
            // Increase weight to 0.5 for faster trust changes
            let trust_value = if success { 1.0 } else { 0.0 };
            eigen.update_local_trust(node_id, trust_value, 0.25);
        }
        Ok(())
    }

    pub async fn probe_and_update_trust(
        &self,
        key: PoolKey,
        node_id: NodeId,
    ) -> Result<ProbeResult> {
        let ticket = BlobTicket::new(node_id.into(), key.hash, iroh_blobs::BlobFormat::Raw)
            .expect("valid ticket");

        // Reduce timeout threshold to 2 seconds
        let probe_result = match Self::probe_node(ticket).await {
            Ok(stats) => {
                // More aggressive timeout threshold - now 2 seconds instead of 10
                if stats.elapsed.as_millis() < 2000 {
                    ProbeResult::Success(stats)
                } else {
                    ProbeResult::Timeout(stats.elapsed)
                }
            }
            Err(_) => ProbeResult::Error,
        };

        // Update trust based on probe result
        let success = matches!(probe_result, ProbeResult::Success(_));

        self.update_local_trust(key.clone(), node_id, success)
            .await?;

        Ok(probe_result)
    }

    // Add this method to probe all nodes in a pool
    pub async fn probe_pool(&self, key: PoolKey) -> Result<()> {
        let peers = self.get_pool_peers(key.clone()).await?;
        for node_id in peers {
            if let Err(e) = self.probe_and_update_trust(key.clone(), node_id).await {
                tracing::warn!(
                    "tracker::probe_pool: failed to probe node {}: {}",
                    node_id,
                    e
                );
            }
        }
        Ok(())
    }

    /// Start background jobs for pool maintenance
    pub async fn start_background_jobs(&self) {
        let tracker = self.clone();

        // Try initial bootstrap with retries
        for i in 0..3 {
            match tracker.update_all_pools().await {
                Ok(_) => {
                    tracing::info!(
                        "tracker::start_background_jobs: successfully bootstrapped tracker"
                    );
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "tracker::start_background_jobs: bootstrap attempt {} failed: {}",
                        i + 1,
                        e
                    );
                    if i < 2 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        }

        let mut shutdown_rx = self.shutdown_rx.clone();

        // Spawn background task
        tokio::spawn(async move {
            let update_interval = tokio::time::Duration::from_secs(5);
            let mut interval = tokio::time::interval(update_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = tracker.update_all_pools().await {
                            tracing::warn!("tracker::start_background_jobs: failed to update pools: {}", e);
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        tracing::info!("tracker::start_background_jobs: shutting down pool maintenance jobs");
                        break;
                    }
                }
            }
        });
    }

    /// Update all pools - fetch new peers and recalculate trust scores
    async fn update_all_pools(&self) -> Result<()> {
        // read the factory contract
        let factory = self.factory_contract.read().await;

        // get all pools from the factory
        let pools = factory.get_all_pools().await?;

        // add the new pools to the pool set and update the pool info
        let mut pool_info = Vec::new();
        for pool in pools {
            let pool_contract =
                PoolContract::new(pool, &self.eth_ws_url, &self.eth_private_key, self).await?;
            let hash = match pool_contract.get_hash().await {
                Ok(hash) => hash,
                Err(e) => {
                    tracing::warn!(
                        "tracker::update_all_pools: failed to get hash for pool {}: {}",
                        pool,
                        e
                    );
                    continue;
                }
            };
            let balance = match pool_contract.get_balance().await {
                Ok(balance) => balance,
                Err(e) => {
                    tracing::warn!(
                        "tracker::update_all_pools: failed to get balance for pool {}: {}",
                        pool,
                        e
                    );
                    continue;
                }
            };
            let pi = PoolInfo {
                key: PoolKey {
                    hash,
                    address: pool,
                },
                balance,
            };
            pool_info.push(pi.clone());
            let pk = pi.key();
            if !self.pools.read().await.contains_key(&pk) {
                self.add_pool(pk.clone(), balance).await?;
            } else {
                // update the pool balance
                self.pools.write().await.insert(pk.clone(), balance);
            }
        }

        // update the pool peers and trust scores
        for pi in pool_info {
            let pool_key = pi.key();
            // get the current pool peers
            let peers = self.get_pool_peers(pool_key.clone()).await?;
            let peers_set: HashSet<_> = peers.clone().into_iter().collect();
            // get the historical peers
            let peers = get_peers(pi.key().address, &self.eth_ws_url).await?;
            let set: HashSet<_> = peers.clone().into_iter().collect();
            // get the new peer/
            let new_peers = set.difference(&peers_set);
            // add the new peers
            for peer in new_peers {
                self.add_pool_peer(pool_key.clone(), peer.clone()).await;
            }

            if !peers_set.contains(&self.current_node_id) {
                // check if you have the hash
                let stat = self.blobs_service.get_blob_stat(pool_key.hash).await?;
                let mut proceed = stat;
                if !stat {
                    // iterate through the peers and attempt to download the hash
                    for peer in peers.clone() {
                        let ticket = BlobTicket::new(
                            peer.into(),
                            pool_key.hash,
                            iroh_blobs::BlobFormat::Raw,
                        )
                        .expect("valid ticket");
                        if let Ok(_) = self.blobs_service.download_blob(&ticket).await {
                            proceed = true;
                            break;
                        }
                    }
                }
                if proceed {
                    match self.enter_pool(pool_key.clone()).await {
                        Ok(_) => {
                            tracing::info!(
                                "tracker::update_all_pools: successfully joined pool {}",
                                pool_key.address
                            )
                        }
                        Err(e) => {
                            tracing::warn!(
                                "tracker::update_all_pools: failed to join pool {}: {}",
                                pool_key.address,
                                e
                            )
                        }
                    }
                } else {
                    tracing::warn!(
                        "tracker::update_all_pools: failed to download hash for pool {}, skipping join",
                        pool_key.address
                    );
                }
            } else {
                tracing::debug!(
                    "tracker::update_all_pools: already in pool {}, skipping join",
                    pool_key.address
                );
            }
            // this both probes and updates trust
            let _probes = self.probe_pool(pool_key.clone()).await?;
        }

        Ok(())
    }

    /// Find first available peer with positive trust score
    pub async fn find_peer(&self, hash: Hash) -> Option<NodeId> {
        let pools = self.pools.read().await;

        for (pool_key, _) in pools.iter() {
            if pool_key.hash == hash {
                if let Ok(Some(trust_scores)) = self.get_pool_trust(pool_key).await {
                    // Return first peer with positive trust score
                    return trust_scores
                        .into_iter()
                        .find(|(peer, trust)| *trust > 0.0 && *peer != self.current_node_id)
                        .map(|(peer, _)| peer);
                }
            }
        }
        None
    }

    /// Pull a blob from the network
    pub async fn pull_blob(&self, hash: Hash) -> Result<()> {
        // Check if we already have the blob
        let stat = self.blobs_service.get_blob_stat(hash).await?;
        if !stat {
            let peer = self
                .find_peer(hash)
                .await
                .ok_or_else(|| anyhow::anyhow!("No peers available for hash {}", hash))?;

            let ticket = BlobTicket::new(peer.into(), hash, iroh_blobs::BlobFormat::Raw)
                .expect("valid ticket");

            let _stats = self.blobs_service.download_blob(&ticket).await?;
        }
        Ok(())
    }
}
