use std::sync::Arc;

use alloy::{
    eips::BlockNumberOrTag,
    network::EthereumWallet,
    primitives::{Address, FixedBytes, Log, U256},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use futures_util::StreamExt;
use iroh_blobs::Hash;
use tokio::sync::{mpsc, watch, Mutex};
use url::Url;

// Define event for internal communication
#[derive(Debug, Clone)]
pub enum FactoryEvent {
    PoolCreated {
        pool_address: Address,
        hash: Hash,
        balance: U256,
    },
}

sol!(
    event PoolCreated(address indexed poolAddress, bytes32 hash, uint256 balance);
);

sol!(
    #[sol(rpc)]
    "../../contracts/src/Factory.sol"
);

/// Factory contract wrapper
#[derive(Clone)]
pub struct FactoryContract {
    ws_url: Url,
    private_key: PrivateKeySigner,
    address: Address,
    provider: Arc<Mutex<Arc<dyn Provider>>>,
    event_sender: mpsc::Sender<FactoryEvent>,
}

impl FactoryContract {
    /// Create a new factory contract
    pub async fn new(
        address: &Address,
        ws_url: &Url,
        private_key: &PrivateKeySigner,
        event_sender: mpsc::Sender<FactoryEvent>,
    ) -> Result<Self> {
        let ws = WsConnect::new(ws_url.as_str());
        let provider = Arc::new(
            ProviderBuilder::new()
                .with_chain(alloy_chains::NamedChain::AnvilHardhat)
                .wallet(EthereumWallet::from(private_key.clone()))
                .on_ws(ws)
                .await?,
        );

        Ok(Self {
            address: address.clone(),
            ws_url: ws_url.clone(),
            private_key: private_key.clone(),
            provider: Arc::new(Mutex::new(provider)),
            event_sender,
        })
    }

    pub async fn listen_events(&self, shutdown_rx: watch::Receiver<()>) -> Result<()> {
        let filter = Filter::new()
            .address(self.address)
            .from_block(BlockNumberOrTag::Latest);

        let provider = self.provider.lock().await;
        let watch = provider.subscribe_logs(&filter).await?;
        let mut stream = watch.into_stream();

        let provider_clone = provider.clone();
        let event_sender = self.event_sender.clone();
        let mut shutdown = shutdown_rx;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                            Some(log) = stream.next() => {
                                let primitive_log = Log::from(log);
                                if let Ok(event) = PoolCreated::decode_log(&primitive_log, true) {
                                    let pool_address = event.poolAddress;
                                    let hash_fixed_bytes = event.hash.clone();
                                    let hash_vec = hash_fixed_bytes.as_slice().to_vec();
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(hash_vec.as_slice());
                let hash = iroh_blobs::Hash::from_bytes(hash_bytes);
                let balance = event.balance;

                                    // Send event to tracker
                                    let _ = event_sender.send(FactoryEvent::PoolCreated {
                                        pool_address,
                                        hash,
                                        balance,
                                    }).await;
                                }
                            }
                            _ = shutdown.changed() => {
                                tracing::info!("Shutting down factory contract listener");
                                break;
                            }
                        }
            }
            let _provider = provider_clone;
        });

        Ok(())
    }

    /// Get all deployed pools
    pub async fn get_all_pools(&self) -> Result<Vec<Address>> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_builtin(self.ws_url.as_str())
            .await?;

        let factory = Factory::new(self.address, provider);
        let pools = factory.getAllPools().call().await?._0;
        Ok(pools)
    }

    pub async fn create_pool(&self, hash: Hash, value: Option<U256>) -> Result<()> {
        let provider = ProviderBuilder::new()
            .with_chain(alloy_chains::NamedChain::AnvilHardhat)
            .wallet(EthereumWallet::from(self.private_key.clone()))
            .on_builtin(self.ws_url.as_str())
            .await?;

        let address: Address = self.private_key.address();
        let factory = Factory::new(self.address, provider);
        let u256_value = value.unwrap_or(U256::ZERO);
        let hash_bytes = hash.as_bytes();
        let hash_fixed_bytes = FixedBytes::from_slice(hash_bytes);
        let tx = factory
            .createPool(hash_fixed_bytes)
            .from(address)
            .value(u256_value)
            .send()
            .await?;
        let _receipt = tx.watch().await?;
        Ok(())
    }
}
