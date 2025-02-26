use std::collections::{HashMap, HashSet};
use std::sync::Arc;


use alloy::eips::BlockNumberOrTag;
use alloy::{
    providers::Provider,
    primitives::Address, providers::ProviderBuilder, rpc::types::Filter, sol, sol_types::SolEvent
};
use alloy::primitives::Log as PrimitivesLog;
use anyhow::Result;
use futures_util::StreamExt;
use iroh::{Endpoint, NodeId};
use iroh_blobs::get::Stats;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobFormat, Hash, HashAndFormat};
use tokio::sync::RwLock;

use crate::server::ephemeral_endpoint; 

mod probe;

use probe::probe_complete;

sol!(
    event TicketBroadcast(string ticket, address sender);
    event TicketRemoved(string ticket, address sender);
);

sol!(
    #[sol(rpc)]
    "../../contracts/broadcast/src/Contract.sol"
);



/// Key type for the content tracker
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ContentKey {
    hash: Hash,
    format: BlobFormat,
}

impl From<HashAndFormat> for ContentKey {
    fn from(hf: HashAndFormat) -> Self {
        Self {
            hash: hf.hash,
            format: hf.format,
        }
    }
}

impl From<BlobTicket> for ContentKey {
    fn from(ticket: BlobTicket) -> Self {
        Self {
            hash: ticket.hash(),
            format: ticket.format(),
        }
    }
}
impl From<ContentKey> for HashAndFormat {
    fn from(key: ContentKey) -> Self {
        Self {
            hash: key.hash,
            format: key.format,
        }
    }
}

impl From<(Hash, BlobFormat)> for ContentKey {
    fn from((hash, format): (Hash, BlobFormat)) -> Self {
        Self { hash, format }
    }
}

/// Simple in-memory service that tracks content announcements
#[derive(Clone, Debug)]
pub struct TrackerService {
    node_id: NodeId,
    address: Address,
    ws_url: String,
    content_map: Arc<RwLock<HashMap<ContentKey, HashSet<NodeId>>>>,
}

impl TrackerService {
    /// Create a new tracker service
    pub async fn new(
        address: Address,
        ws_url: String,
        endpoint: Endpoint) -> Result<Self> {
        // We're ignoring the data_dir for now - this is a pure in-memory implementation
        Ok(Self {
            node_id: endpoint.node_id(),
            address,
            ws_url,
            content_map: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn probe_node(&self,
       node_id: NodeId,
       key: HashAndFormat) -> Result<Stats> {
        let endpoint = ephemeral_endpoint().await;
        let stats = probe_complete(&endpoint, &node_id, &key).await?;
        Ok(stats)
    }

    pub async fn start_listening(&self) -> Result<()> {
        let ws = alloy::providers::WsConnect::new(self.ws_url.as_str());
        let provider = ProviderBuilder::new()
            .on_ws(ws)
            .await?;
        
        let filter = Filter::new()
            .address(self.address)
            .event("TicketBroadcast(string,address)")
            .event("TicketRemoved(string,address)")
            .from_block(BlockNumberOrTag::Latest);
        
        let watch = provider.subscribe_logs(&filter).await?;
        
        // Clone the fields we need before spawning
        let content_map = self.content_map.clone();
        let self_node_id = self.node_id;
        
        tokio::spawn(async move {
            let mut stream = watch.into_stream();
            
            while let Some(log) = stream.next().await {
                let primitive_log = PrimitivesLog::from(log);
                
                if let Ok(event) = TicketBroadcast::decode_log(&primitive_log, true) {
                    tracing::info!("Received message event: {}", event.ticket);
                    let ticket = event.ticket.parse::<BlobTicket>().unwrap();
                    let node_id = ticket.node_addr().node_id;
                    if node_id == self_node_id {
                        continue;
                    }
                    let key = ContentKey::from(ticket);
                    let mut map = content_map.write().await;
                    map.entry(key)
                        .or_insert_with(HashSet::new)
                        .insert(node_id);
                }
                else if let Ok(event) = TicketRemoved::decode_log(&primitive_log, true) {
                    tracing::info!("Received value event: {}", event.ticket);
                    let ticket = event.ticket.parse::<BlobTicket>().unwrap();
                    let node_id = ticket.node_addr().node_id;
                    if node_id == self_node_id {
                        continue;
                    }
                    let key = ContentKey::from(ticket);
                    let mut map = content_map.write().await;
                    map.entry(key).and_modify(|nodes| {
                        nodes.remove(&node_id);
                    });
                }
            }
        });
        
        Ok(())
    }

    /// Broadcast a ticket to the Ethereum network
    pub async fn broadcast_ticket(&self, ticket: BlobTicket ) -> Result<()> {
        let provider = ProviderBuilder::new()
            .on_anvil_with_wallet();
        let contract = Contract::new(self.address, provider);
        
        // Convert ticket to string representation
        let ticket_str = ticket.to_string();
        
        // Call the broadcast function on the smart contract
        let tx = contract.broadcastTicket(ticket_str.clone()).send().await?;
        tx.watch().await?;
        
        tracing::info!("Broadcasted ticket to Ethereum: {}", ticket_str);
        Ok(())
    }
} 