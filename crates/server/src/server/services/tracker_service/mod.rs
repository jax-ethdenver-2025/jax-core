use std::collections::{HashMap, HashSet};
use std::sync::Arc;


use alloy::eips::BlockNumberOrTag;
use alloy::{
    providers::Provider,
    network::EthereumWallet,
    signers::local::PrivateKeySigner,
    primitives::Address, providers::ProviderBuilder, rpc::types::Filter, sol, sol_types::SolEvent
};
use alloy::primitives::Log as PrimitivesLog;
use anyhow::Result;
use futures_util::StreamExt;
use iroh::{Endpoint, NodeId};
use iroh_blobs::get::Stats;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobFormat, Hash, HashAndFormat};
use tokio::sync::{RwLock, Mutex};

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
#[derive(Clone)]
pub struct TrackerService {
    node_id: NodeId,
    address: Address,
    ws_url: String,
    http_url: String,
    private_key_signer: PrivateKeySigner,
    content_map: Arc<RwLock<HashMap<ContentKey, HashSet<NodeId>>>>,
    ws_provider: Arc<Mutex<Option<Arc<dyn Provider>>>>,
}

impl TrackerService {
    /// Create a new tracker service
    pub async fn new(
        address: Address,
        ws_url: String,
        http_url: String,
        private_key: String,
        endpoint: Endpoint) -> Result<Self> {
        // We're ignoring the data_dir for now - this is a pure in-memory implementation
        Ok(Self {
            node_id: endpoint.node_id(),
            address,
            ws_url,
            http_url,
            private_key_signer: private_key.parse::<PrivateKeySigner>().unwrap(),
            content_map: Arc::new(RwLock::new(HashMap::new())),
            ws_provider: Arc::new(Mutex::new(None)),
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
        let provider = Arc::new(
            ProviderBuilder::new()
                .with_chain(alloy_chains::NamedChain::AnvilHardhat)
                .on_ws(ws)
                .await?
        );
        
        let filter = Filter::new()
            .address(self.address)
            // TODO: maybe add filters
            .from_block(BlockNumberOrTag::Latest);
        
        let watch = provider.subscribe_logs(&filter).await?;
        let mut stream = watch.into_stream();

        // Clone the fields we need before spawning
        let content_map = self.content_map.clone();
        let self_node_id = self.node_id;
        let provider_clone = provider.clone();
        
        tokio::spawn(async move {
            while let Some(log) = stream.next().await {
                let primitive_log = PrimitivesLog::from(log);

                if let Ok(event) = TicketBroadcast::decode_log(&primitive_log, true) {
                    let ticket = match event.ticket.parse::<BlobTicket>() {
                        Ok(ticket) => ticket,
                        Err(e) => {
                            tracing::error!("tracker_service::start_listening: failed to parse ticket: {}", e);
                            continue;
                        }
                    };
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
                // TODO: add removed event handling
                // else if let Ok(event) = TicketRemoved::decode_log(&primitive_log, true) {
                //     tracing::info!("Received value event: {}", event.ticket);
                //     let ticket = event.ticket.parse::<BlobTicket>().unwrap();
                //     let node_id = ticket.node_addr().node_id;
                //     if node_id == self_node_id {
                //         continue;
                //     }
                //     let key = ContentKey::from(ticket);
                //     let mut map = content_map.write().await;
                //     map.entry(key).and_modify(|nodes| {
                //         nodes.remove(&node_id);
                //     });
                // }
            }
            // Keep the provider alive
            let _provider = provider_clone;
        });

        // Store the provider in the service to keep it alive
        let mut guard = self.ws_provider.lock().await;
        *guard = Some(provider);
        
        Ok(())
    }

    /// Broadcast a ticket to the Ethereum network
    pub async fn broadcast_ticket(&self, ticket: BlobTicket ) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key_signer.clone()))
            .on_builtin(self.http_url.as_str())
            .await?;
        let contract = Contract::new(self.address, provider);
        
        // Convert ticket to string representation
        let ticket_str = ticket.to_string();
        
        // Call the broadcast function on the smart contract
        let tx = contract.broadcastTicket(ticket_str.clone()).send().await?;
        tx.watch().await?;
        
        tracing::info!("Broadcasted ticket to Ethereum: {}", ticket_str);
        Ok(())
    }

    /// Get all known locations for a given hash
    pub async fn get_locations(&self, hash: Hash) -> Result<Vec<NodeId>> {
        let map = self.content_map.read().await;
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