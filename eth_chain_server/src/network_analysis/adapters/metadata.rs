use alloy_primitives::Address;
use eth_token::erc20::ERC20TokenMetadata;
use eth_token::network::model::normalize_network_address;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;

pub async fn load_token_metadata(
    provider: &RethQueryProvider,
    token: Address,
    block_number: u64,
) -> Result<ERC20TokenMetadata> {
    let metadata = provider
        .get_token_metadata(token, Some(block_number), None)
        .await?
        .ok_or_else(|| {
            eyre!(
                "token {:#x} did not return ERC20 metadata at block {block_number}",
                token
            )
        })?;

    Ok(ERC20TokenMetadata {
        address: normalize_network_address(format!("{:#x}", metadata.address)),
        name: metadata.name,
        symbol: metadata.symbol,
        decimals: metadata.decimals,
        total_supply: metadata.total_supply.to_string(),
    })
}
