use std::collections::HashSet;
use std::sync::Arc;

use alloy::primitives::{FixedBytes, U256};
use alloy::{
    eips::BlockNumberOrTag,
    network::EthereumWallet,
    primitives::{Address, Log},
    providers::ProviderBuilder,
    providers::{Provider, WsConnect},
    rpc::types::Filter,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use ed25519::Signature as Ed25519Signature;
use futures_util::StreamExt;
use iroh::NodeId;
use iroh_blobs::Hash;
use tokio::sync::{watch, Mutex};
use url::Url;

use crate::node::tracker::Tracker;

// Define the PeerAdded event
sol!(
    event PeerAdded(string nodeId);
    event Deposit(uint256 amount, bytes32 hash);
);

// TODO: make this use the sol file
// Define the RewardPool contract interface
sol! {
    #[sol(rpc)]
    contract RewardPool {
        function enterPool(string memory nodeId, bytes32 k, bytes32 r, bytes32 s, address memory m) external;
        function getHash() external view returns (bytes32);
        function getPeers() external view returns (string[] memory);
        function getBalance() external view returns (uint256);
        function deposit() external payable;
    }
}

/// Simple in-memory service that tracks content announcements
#[derive(Clone)]
pub struct PoolContract {
    ws_url: Url,
    private_key: PrivateKeySigner,
    address: Address,
    #[allow(dead_code)]
    provider: Arc<Mutex<Arc<dyn Provider>>>,
    tracker: Tracker,
    iroh_signature: Ed25519Signature,
}

#[allow(dead_code)]
// NOTE (amiller68): not even used, see not below on event listener
// Define event for internal communication
#[derive(Debug, Clone)]
pub enum PoolEvent {
    PeerAdded {
        pool_address: Address,
        hash: Hash,
        node_id: String,
    },
    Deposit {
        pool_address: Address,
        hash: Hash,
        amount: U256,
    },
}

impl PoolContract {
    /// Create a new tracker service
    pub async fn new(
        address: Address,
        ws_url: &Url,
        private_key: &PrivateKeySigner,
        tracker: &Tracker,
    ) -> Result<Self> {
        let ws = WsConnect::new(ws_url.as_str());
        let provider = Arc::new(
            ProviderBuilder::new()
                .with_chain(alloy_chains::NamedChain::AnvilHardhat)
                .wallet(EthereumWallet::from(private_key.clone()))
                .on_ws(ws)
                .await?,
        );
        // We're ignoring the data_dir for now - this is a pure in-memory implementation
        Ok(Self {
            address,
            ws_url: ws_url.clone(),
            private_key: private_key.clone(),
            provider: Arc::new(Mutex::new(provider)),
            tracker: tracker.clone(),
            iroh_signature: tracker.iroh_signature,
        })
    }

    // TODO: create a pool

    // TODO: get this hooked up to handlers
    #[allow(dead_code)]
    pub async fn listen_events(&self, hash: Hash, shutdown_rx: watch::Receiver<()>) -> Result<()> {
        let filter = Filter::new()
            .address(self.address)
            .from_block(BlockNumberOrTag::Latest);

        let provider = self.provider.lock().await;
        let watch = provider.subscribe_logs(&filter).await?;
        let mut stream = watch.into_stream();

        let provider_clone = provider.clone();
        let tracker = self.tracker.clone();
        let pool_address = self.address;
        let pool_hash = hash;
        let mut shutdown = shutdown_rx;

        // TODO: i think the original design for this assumed i was sending pool messages
        //  over an event channel but it doesn't look like cursor actually implemented that pattern ...
        //  we should fix this and get on event channels
        // Yeah i really gotta unmess this up because its kinda trash and makes no sense
        //  like the ideas are so screwed up -- we should be emitting events and letting the tracker
        //  manage its own state. For now this will write directly to the tracker
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(log) = stream.next() => {
                        let primitive_log = Log::from(log);
                        if let Ok(event) = PeerAdded::decode_log(&primitive_log, true) {
                            let node_id = event.nodeId.clone();
                            if let Ok(node_id) = node_id.parse::<NodeId>() {
                                // Create a pool key and add the peer
                                let key = crate::node::tracker::PoolKey {
                                    hash: pool_hash,
                                    address: pool_address,
                                };
                                tracker.add_pool_peer(key, node_id).await;
                                tracing::info!("Added peer {} to pool {}", node_id, pool_address);
                            }
                        } else if let Ok(event) = Deposit::decode_log(&primitive_log, true) {
                            // NOTE (amiller68): this is actually just screwing with the frontend and local state
                            //  just trace it for now
                            let amount = event.amount;
                            // let key = PoolKey {
                            //     hash: pool_hash,
                            //     address: pool_address,
                            // };
                            // tracker.add_pool_deposit(key, amount).await;
                            tracing::info!("Added deposit {} to pool {}", amount, pool_address);
                        }
                    }
                    _ = shutdown.changed() => {
                        tracing::info!("Shutting down pool contract listener for {}", pool_address);
                        break;
                    }
                }
            }
            let _provider = provider_clone;
        });

        Ok(())
    }

    pub async fn enter_pool(&self) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let iroh_signature = self.iroh_signature;
        let node_id = self.tracker.current_node_id;
        let k_bytes = FixedBytes::<32>::try_from(self.tracker.current_node_id.as_bytes()).expect("Failed to convert node_id to FixedBytes");
        let r_bytes = FixedBytes::<32>::from(iroh_signature.r_bytes());
        let s_bytes = FixedBytes::<32>::from(iroh_signature.s_bytes());
        let m = self.private_key.address();
        let tx = contract
            .enterPool(node_id.to_string(), k_bytes, r_bytes, s_bytes, m)
            .send()
            .await?;
        let _receipt = tx.watch().await?;
        Ok(())
    }

    pub async fn get_balance(&self) -> Result<U256> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let balance = contract.getBalance().call().await?._0;
        Ok(balance)
    }

    pub async fn deposit(&self, amount: U256) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let tx = contract.deposit().value(amount).send().await?;
        let _receipt = tx.watch().await?;
        Ok(())
    }

    pub async fn get_hash(&self) -> Result<iroh_blobs::Hash> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_ws(WsConnect::new(self.ws_url.as_str()))
            .await?;
        let contract = RewardPool::new(self.address, provider);
        let hash_fixed_bytes = contract.getHash().call().await?._0;
        let hash_vec = hash_fixed_bytes.as_slice().to_vec();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(hash_vec.as_slice());
        let hash = Hash::from_bytes(hash_bytes);
        Ok(hash)
    }
}

// TODO: jank as hell to have this here
pub async fn get_peers(address: Address, ws_url: &Url) -> Result<HashSet<NodeId>> {
    let provider = ProviderBuilder::new()
        .with_chain(alloy_chains::NamedChain::AnvilHardhat)
        .on_ws(WsConnect::new(ws_url.as_str()))
        .await?;
    let contract = RewardPool::new(address, provider);
    let peers = contract.getPeers().call().await?._0;
    let mut peer_set = HashSet::new();
    for peer in peers {
        if let Ok(node_id) = peer.parse::<NodeId>() {
            peer_set.insert(node_id);
        }
    }
    Ok(peer_set)
}

mod test {
    #[allow(unused)]
    use super::*;

    #[tokio::test]
    async fn test_enter_pool() -> Result<()> {
        // Setup test parameters
        let zeros = [0u8; 32];
        let secret_key = iroh::SecretKey::from_bytes(&zeros);
        let node_id = secret_key.public();
        let private_key: PrivateKeySigner = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse()?;
        let message = private_key.address().into_array();
        let signature = secret_key.sign(message.as_ref());
        let k_bytes = FixedBytes::<32>::try_from(node_id.as_bytes()).expect("Failed to convert node_id to FixedBytes");
        let r_bytes = FixedBytes::<32>::from(signature.r_bytes());
        let s_bytes = FixedBytes::<32>::from(signature.s_bytes());
        let m = private_key.address();
        // Remember to replace this line every time we redeploy factory
        let pool_address = "0x41CD982c4C291B50B2C2b3113ca4Cc7EE3e33c63".parse::<Address>()?;
        let ws_url = Url::parse("ws://localhost:8545")?;

        // Create a provider
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(private_key.clone()))
            .on_ws(WsConnect::new(ws_url.as_str()))
            .await?;

        println!("message: {:?}", message);
        println!("k_bytes: {:?}", k_bytes);
        println!("r_bytes: {:?}", r_bytes);
        println!("s_bytes: {:?}", s_bytes);
        println!("m: {:?}", m);

        // Create the contract instance
        let contract = RewardPool::new(pool_address, provider);

        // Call the enterPool function
        let tx = contract.enterPool(
            node_id.to_string(),
            k_bytes,
            r_bytes,
            s_bytes,
            m
        ).send().await?;
        
        let _receipt = tx.watch().await?;

        // Verify the peer was added
        let peers = contract.getPeers().call().await?._0;
        assert!(peers.contains(&node_id.to_string()));

        Ok(())
    }
}