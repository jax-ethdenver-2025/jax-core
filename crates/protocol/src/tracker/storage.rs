//! Storage implementation for tracking content announcements and discovery

use std::path::PathBuf;
use std::sync::Arc;
use iroh::NodeId;
use iroh_blobs::HashAndFormat;
use redb::{Database, ReadableTable, TableDefinition, Key as RedbKey, Value as RedbValue, TypeName};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

// Create serializable versions of our key components
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct HashAndFormatKey(HashAndFormat);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeIdKey(NodeId);

// Key type for the announces table
pub type AnnounceKey = (HashAndFormatKey, NodeIdKey);
pub type AnnounceValue = u64; // Timestamp

// Implement RedbKey for HashAndFormatKey
impl RedbKey for HashAndFormatKey {
    fn compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
        let key_a: Self = bincode::deserialize(a).unwrap();
        let key_b: Self = bincode::deserialize(b).unwrap();
        key_a.0.cmp(&key_b.0)
    }
}

// Implement RedbKey for NodeIdKey
impl RedbKey for NodeIdKey {
    fn compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
        let key_a: Self = bincode::deserialize(a).unwrap();
        let key_b: Self = bincode::deserialize(b).unwrap();
        key_a.0.cmp(&key_b.0)
    }
}

// Add necessary From/Into implementations
impl From<HashAndFormat> for HashAndFormatKey {
    fn from(hf: HashAndFormat) -> Self {
        Self(hf)
    }
}

impl From<HashAndFormatKey> for HashAndFormat {
    fn from(key: HashAndFormatKey) -> Self {
        key.0
    }
}

impl From<NodeId> for NodeIdKey {
    fn from(id: NodeId) -> Self {
        Self(id)
    }
}

impl From<NodeIdKey> for NodeId {
    fn from(key: NodeIdKey) -> Self {
        key.0
    }
}

// Implement RedbValue for HashAndFormatKey
impl RedbValue for HashAndFormatKey {
    type SelfType<'a> = Self;
    type AsBytes<'a> = std::borrow::Cow<'a, [u8]>;

    fn type_name() -> TypeName {
        TypeName::new("hash_and_format")
    }

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(bytes: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(bytes).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a + 'b,
    {
        let bytes = bincode::serialize(value).unwrap();
        std::borrow::Cow::Owned(bytes)
    }
}

// Implement RedbValue for NodeIdKey
impl RedbValue for NodeIdKey {
    type SelfType<'a> = Self;
    type AsBytes<'a> = std::borrow::Cow<'a, [u8]>;

    fn type_name() -> TypeName {
        TypeName::new("node_id")
    }

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(bytes: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(bytes).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a + 'b,
    {
        let bytes = bincode::serialize(value).unwrap();
        std::borrow::Cow::Owned(bytes)
    }
}

// Define the table with our wrapper types
const ANNOUNCES_TABLE: TableDefinition<AnnounceKey, AnnounceValue> = 
    TableDefinition::new("announces");

#[derive(Debug)]
pub struct ContentEntry {
    pub hash_and_format: HashAndFormat,
    pub nodes: Vec<NodeId>,
    pub last_seen: std::time::SystemTime,
}

#[derive(Debug)]
pub struct StorageManager {
    db: Arc<RwLock<Database>>,
}

impl StorageManager {
    /// Create a new storage manager with the given database path
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let db = Database::create(path)?;
        let db = Arc::new(RwLock::new(db));
        
        Ok(Self { db })
    }
    
    /// Store an announcement for content from our node
    pub async fn store_announcement(&self, content: HashAndFormat, node_id: NodeId) -> anyhow::Result<()> {
        let db = self.db.write().await;
        let tx = db.begin_write()?;
        {
            let mut table = tx.open_table(ANNOUNCES_TABLE)?;
            // Store current time as timestamp
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            
            // Use the provided node_id
            table.insert((content.into(), node_id.into()), now)?;
        }
        tx.commit()?;
        
        Ok(())
    }
    
    /// Get all known content entries
    pub async fn get_all_content(&self) -> anyhow::Result<Vec<ContentEntry>> {
        let db = self.db.read().await;
        let tx = db.begin_read()?;
        let table = tx.open_table(ANNOUNCES_TABLE)?;
        
        let mut content_map = std::collections::HashMap::new();
        
        let iter = table.iter()?;
        for item in iter {
            let (key_guard, value_guard) = item?;
            let key_tuple = key_guard.value();
            let hash_and_format: HashAndFormat = key_tuple.0.clone().into();
            let node_id: NodeId = key_tuple.1.clone().into();
            let timestamp = value_guard.value();
            
            let entry = content_map
                .entry(hash_and_format)
                .or_insert_with(|| ContentEntry {
                    hash_and_format,
                    nodes: Vec::new(),
                    last_seen: std::time::UNIX_EPOCH,
                });
            
            entry.nodes.push(node_id);
            
            // Update last_seen if this timestamp is newer
            let timestamp_duration = std::time::Duration::from_secs(timestamp);
            let timestamp_time = std::time::UNIX_EPOCH + timestamp_duration;
            if timestamp_time > entry.last_seen {
                entry.last_seen = timestamp_time;
            }
        }
        
        Ok(content_map.into_values().collect())
    }
    
    /// Find nodes that have announced a specific content
    pub async fn find_nodes_for_content(&self, content: HashAndFormat) -> anyhow::Result<Vec<NodeId>> {
        let db = self.db.read().await;
        let tx = db.begin_read()?;
        let table = tx.open_table(ANNOUNCES_TABLE)?;
        
        let mut nodes = Vec::new();
        // let content_key = HashAndFormatKey::from(content);
        
        let range = table.iter()?;
        for item in range {
            let (key_guard, _) = item?;
            let key_tuple = key_guard.value();
            let node_id: NodeId = key_tuple.1.clone().into();
            nodes.push(node_id);
        }
        
        Ok(nodes)
    }
} 