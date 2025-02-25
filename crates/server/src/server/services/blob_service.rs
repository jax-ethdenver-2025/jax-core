use anyhow::Result;
use iroh::Endpoint;
use iroh_blobs::{
    BlobFormat, 
    Hash, 
    net_protocol::Blobs,
    store::{fs::Store, ExportFormat, ExportMode},
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
    
    /// Get the underlying blob store
    pub fn store(&self) -> &Store {
        self.blobs.store()
    }
    
    /// Get a blob by hash and format
    pub async fn get_blob(&self, hash: Hash, format: BlobFormat) -> Result<Vec<u8>> {
        let temp_path = std::env::temp_dir().join(hash.to_string());
        
        self.blobs.client().export(hash, temp_path.clone(), ExportFormat::Blob, ExportMode::Copy)
            .await?
            .finish()
            .await?;
            
        let data = tokio::fs::read(temp_path).await?;
        Ok(data)
    }
    
    /// Store a blob with the given format
    pub async fn store_blob(&self, data: Vec<u8>, format: BlobFormat) -> Result<Hash> {
        // let temp_path = std::env::temp_dir().join(format!("temp-{}", uuid::Uuid::new_v4()));
        
        // Write data to temp file
        // tokio::fs::write(&temp_path, &data).await?;
        
        
        // Import the blob directly using blobs.client()
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