use alloy_primitives::{Address, B256};
/// Balancer Price Feed Example
///
/// Shows prices from Balancer weighted pools and meta stable pools
/// Uses the modular tx_simulator for view function calls
use eth_price::BalancerReader;
use hex::decode as hex_decode;
use reth_chain_query::provider_factory_from_datadir;
use reth_provider::BlockNumReader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚖️  Balancer Price Feed - Weighted Pool AMM");
    println!("{}", "=".repeat(50));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize Balancer reader with shared provider
    let reader = BalancerReader::from_provider(provider_factory.clone())?;

    println!("\n📊 ETH Price from Balancer Weighted Pool (generic, by poolId):");
    println!("{}", "-".repeat(50));

    // Balancer Vault and DAI/WETH 60/40 weighted poolId
    let vault =
        Address::from_slice(&hex_decode("BA12222222228d8Ba445958a75a0704d566BF2C8").unwrap());
    let dai_weth_pool_id = B256::from_slice(
        &hex_decode("0b09dea16768f0799065c475be02919503cb2a3500020000000000000000001a").unwrap(),
    );

    // token0=DAI(18), token1=WETH(18), weights 60% DAI / 40% WETH, base=WETH is token1
    match reader
        .get_weighted_two_token_price_latest_by_pool(
            vault,
            dai_weth_pool_id,
            18,
            18,
            600_000_000_000_000_000u128,
            400_000_000_000_000_000u128,
            true,
        )
        .await
    {
        Ok((dai_per_eth, block)) => {
            println!(
                "  💰 ETH/DAI (60/40): {:>10.6}  poolId=0x{:x}  block={}",
                dai_per_eth, dai_weth_pool_id, block
            );
        }
        Err(e) => println!("  ❌ ETH/DAI (60/40): {}", e),
    }

    if let Ok(latest_block) = provider_factory.last_block_number() {
        println!("\n📍 Latest block: {}", latest_block);
    }

    println!("\n✅ Balancer generic price feed ready");
    Ok(())
}
