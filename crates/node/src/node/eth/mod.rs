pub mod contracts;

use alloy::providers::Provider;
use anyhow::Result;
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::providers::WsConnect;
use url::Url;
use alloy::primitives::U256;

pub async fn get_address_balance(
    address: Address,
    ws_url: &Url,
) -> Result<U256> {
    let provider = ProviderBuilder::new()
    .with_chain(alloy_chains::NamedChain::AnvilHardhat)
    .on_ws(WsConnect::new(ws_url.as_str()))
    .await?;
    let balance = provider.get_balance(address).await?;
    Ok(balance)
}
