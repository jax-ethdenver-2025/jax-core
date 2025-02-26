use anyhow::Result;
use iroh::Endpoint;
use iroh_blobs::{
    Hash, 
    net_protocol::Blobs,
    store::fs::Store,
};
use std::path::Path;
use std::sync::Arc;

/// Service that handles blob operations
#[derive(Clone, Debug)]
pub struct BlobService {
    blobs: Arc<Blobs<Store>>,
}

impl BlobService {
    /// Create a new blob service
    pub async fn new(data_dir: &Path, endpoint: Endpoint) -> Result<Self> {
        let blobs_path = data_dir.join("blobs");
        let store = Store::load(blobs_path).await?;
        let blobs = Blobs::builder(store).build(&endpoint);
        
        Ok(Self {
            blobs: Arc::new(blobs),
        })
    }
    
    /// Store a blob with the given format
    pub async fn store_blob(&self, data: Vec<u8>) -> Result<Hash> {
        let hash = self.blobs.client().add_bytes(data)
            .await?
            .hash;
            
        Ok(hash)
    }

    /// Get the underlying Blobs instance
    pub fn get_inner_blobs(&self) -> &Arc<Blobs<Store>> {
        &self.blobs
    }
} 