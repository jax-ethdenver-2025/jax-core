use anyhow::Result;
use iroh::{Endpoint, NodeId};
use iroh_blobs::{BlobFormat, Hash};
use jax_protocol::prelude::*;
use std::path::Path;
use std::sync::Arc;

/// Service that handles content tracking and discovery
#[derive(Clone, Debug)]
pub struct TrackerService {
    tracker: Arc<JaxTracker>,
}

impl TrackerService {
    /// Create a new tracker service
    pub async fn new(data_dir: &Path, endpoint: Endpoint) -> Result<Self> {
        let db_path = data_dir.join("tracker.redb");
        let tracker = JaxTracker::new(db_path, endpoint).await?;
        
        Ok(Self {
            tracker: Arc::new(tracker),
        })
    }
    
    /// Announce content that we have available
    pub async fn announce_content(&self, hash: Hash, format: BlobFormat) -> Result<()> {
        self.tracker.announce_content(hash, format).await
    }
    
    /// Find nodes that have announced the specified content
    pub async fn discover_content(&self, hash: Hash, format: BlobFormat) -> Result<Vec<NodeId>> {
        self.tracker.discover_content(hash, format).await
    }
    
    /// Probe a node to verify content availability
    pub async fn probe_node(&self, node_id: NodeId, hash: Hash, format: BlobFormat) 
        -> Result<iroh_blobs::get::Stats> 
    {
        self.tracker.probe_node(node_id, hash, format).await
    }
} 