use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use alloy::primitives::Address;
use anyhow::Result;
use iroh::{Endpoint, NodeId};
use iroh_blobs::get::Stats;
use iroh_blobs::Hash;
use tokio::sync::{Mutex, RwLock};

/// Key type for the content tracker
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    hash: Hash,
    address: Address,
}

/// Simple in-memory store for network state
#[derive(Clone)]
pub struct Tracker {
    pools: Arc<RwLock<HashMap<PoolKey, HashSet<NodeId>>>>,
}

impl Tracker {
    /// Create a new tracker service
    pub fn new() -> Result<Self> {
        Ok(Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get global known peers for a given hash, regardless of pool
    pub async fn get_peers(&self, hash: Hash) -> Result<Vec<NodeId>> {
        let map = self.pools.read().await;
        let mut nodes = Vec::new();

        // Look for this hash with any format
        for (key, node_set) in map.iter() {
            if key.hash == hash {
                nodes.extend(node_set.iter().cloned());
            }
        }

        Ok(nodes)
    }
}
