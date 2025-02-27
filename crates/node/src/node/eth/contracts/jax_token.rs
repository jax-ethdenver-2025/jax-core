use std::sync::Arc;

use alloy::{
    eips::BlockNumberOrTag,
    network::EthereumWallet,
    primitives::{Address, Log},
    providers::ProviderBuilder,
    providers::{Provider, WsConnect},
    rpc::types::Filter,
    signers::local::PrivateKeySigner,
    sol,
};
use anyhow::Result;
use futures_util::StreamExt;
use tokio::sync::Mutex;

sol!(
    #[sol(rpc)]
    "../../contracts/src/JAXToken.sol"
);

/// Simple in-memory service that tracks content announcements
#[derive(Clone)]
pub struct JAXTokenContract {
    address: Address,
    provider: Arc<Mutex<Arc<dyn Provider>>>,
}

impl JAXTokenContract {
    /// Create a new tracker service
    pub async fn new(
        address: Address,
        ws_url: String,
        private_key: PrivateKeySigner,
    ) -> Result<Self> {
        let ws = WsConnect::new(ws_url.as_str());
        let provider = Arc::new(
            ProviderBuilder::new()
                .with_chain(alloy_chains::NamedChain::AnvilHardhat)
                .wallet(EthereumWallet::from(private_key))
                .on_ws(ws)
                .await?,
        );
        // We're ignoring the data_dir for now - this is a pure in-memory implementation
        Ok(Self {
            address,
            provider: Arc::new(Mutex::new(provider)),
        })
    }

    // TODO: create a pool

    // TODO: get this hooked up to handlers
    pub async fn listen_events(&self) -> Result<()> {
        let filter = Filter::new()
            .address(self.address)
            .from_block(BlockNumberOrTag::Latest);

        let provider = self.provider.lock().await;
        let watch = provider.subscribe_logs(&filter).await?;
        let mut stream = watch.into_stream();

        // Clone the fields we need before spawning
        let provider_clone = provider.clone();

        tokio::spawn(async move {
            while let Some(log) = stream.next().await {
                let primitive_log = Log::from(log);
                tracing::info!(
                    "node::eth::contracts::factory::listen_events: received log: {:?}",
                    primitive_log
                );
            }
            // Keep the provider alive
            let _provider = provider_clone;
        });

        Ok(())
    }
}
