//! Protocol implementation for Jax content tracking and discovery

mod alpn;
mod announce;
mod discovery;
mod storage;

pub use announce::{Announcement, AnnounceKind, AnnounceManager};
pub use discovery::{ContentDiscovery, NodeInfo};
pub use storage::{StorageManager, ContentEntry};
pub use alpn::*;  // Export all ALPN identifiers

use iroh::NodeId;
use iroh_blobs::{BlobFormat, Hash, HashAndFormat};

/// The main protocol interface for content tracking and discovery
#[derive(Debug)]
pub struct JaxTracker {
    storage: storage::StorageManager,
    announcer: announce::AnnounceManager,
    discovery: discovery::ContentDiscovery,
}

impl JaxTracker {
    /// Create a new JaxTracker instance
    pub async fn new(
        db_path: std::path::PathBuf,
        endpoint: iroh::Endpoint,
    ) -> anyhow::Result<Self> {
        let storage = storage::StorageManager::new(db_path)?;
        let announcer = announce::AnnounceManager::new(endpoint.clone());
        let discovery = discovery::ContentDiscovery::new(endpoint);
        
        Ok(Self {
            storage,
            announcer,
            discovery,
        })
    }
    
    /// Announce content availability
    pub async fn announce_content(
        &self, 
        hash: Hash, 
        format: BlobFormat
    ) -> anyhow::Result<()> {
        let content = HashAndFormat { hash, format };
        self.announcer.announce(content).await?;
        let node_id = self.discovery.node_id();
        self.storage.store_announcement(content, node_id).await?;
        Ok(())
    }
    
    /// Discover nodes that have the specified content
    pub async fn discover_content(
        &self, 
        hash: Hash, 
        format: BlobFormat
    ) -> anyhow::Result<Vec<NodeId>> {
        let content = HashAndFormat { hash, format };
        self.discovery.find_nodes(content).await
    }
    
    /// Probe a node to verify content availability
    pub async fn probe_node(
        &self, 
        node_id: NodeId, 
        hash: Hash, 
        format: BlobFormat
    ) -> anyhow::Result<iroh_blobs::get::Stats> {
        let content = HashAndFormat { hash, format };
        self.discovery.probe_node(node_id, content).await
    }
} 