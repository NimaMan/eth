use alloy_primitives::{Address, U256};
use clap::Parser;
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_processor::trade_simulation::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    token: Address,
    #[arg(long)]
    pool: Address,
    #[arg(long, default_value = "uniswap-v2")]
    protocol: String,
    #[arg(long)]
    block: u64,
    #[arg(long, value_delimiter = ',', default_value = "10000000000000000")]
    amount_wei: Vec<String>,
    #[arg(long, default_value_t = 18)]
    token_decimals: u8,
    #[arg(long, default_value = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2")]
    denom: Address,
    #[arg(long, default_value_t = 18)]
    denom_decimals: u8,
    #[arg(long)]
    fee_tier: Option<u32>,
    #[arg(long)]
    reth_datadir: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let reth_datadir = args
        .reth_datadir
        .or_else(|| std::env::var("RETH_DATADIR").ok())
        .unwrap_or_else(|| "/home/nima/.local/share/reth/mainnet".to_string());

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    let pool_type = parse_pool_type(&args.protocol, args.fee_tier)?;

    for block in [args.block.saturating_sub(1), args.block] {
        for amount in &args.amount_wei {
            let amount = U256::from_str_radix(amount, 10)
                .map_err(|error| eyre!("invalid --amount-wei value {amount}: {error}"))?;
            let params = PoolBuySellParameters::new(args.token, args.pool, pool_type)
                .with_test_amount(amount)
                .with_block(block)
                .with_denom_address(args.denom)
                .with_denom_decimals(args.denom_decimals)
                .with_token_decimals(args.token_decimals);

            match check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), params).await {
                Ok(result) => {
                    println!(
                        "block={block} amount={amount} tradeable={} buy={} approve={} sell={} tokens_received={} denom_received={} failure={}",
                        result.is_tradeable,
                        result.can_buy,
                        result.can_approve,
                        result.can_sell,
                        result.tokens_received,
                        result.denom_received,
                        result.failure_reason.as_deref().unwrap_or("-")
                    );
                }
                Err(error) => {
                    println!("block={block} amount={amount} error={error:?}");
                }
            }
        }
    }

    Ok(())
}

fn parse_pool_type(protocol: &str, fee_tier: Option<u32>) -> Result<PoolType> {
    match protocol.trim().to_ascii_lowercase().as_str() {
        "uniswap-v2" | "uniswapv2" | "v2" => Ok(PoolType::UniswapV2),
        "sushiswap-v2" | "sushi-v2" | "sushi" => Ok(PoolType::SushiSwap),
        "pancakeswap-v2" | "pancake-v2" | "pancake" => Ok(PoolType::PancakeSwapV2),
        "shibaswap-v2" | "shiba-v2" | "shiba" => Ok(PoolType::ShibaSwapV2),
        "fraxswap-v2" | "frax-v2" | "frax" => Ok(PoolType::FraxswapV2),
        "uniswap-v3" | "uniswapv3" | "v3" => Ok(PoolType::UniswapV3 {
            fee_tier: fee_tier.ok_or_else(|| eyre!("--fee-tier is required for Uniswap V3"))?,
        }),
        "sushiswap-v3" | "sushi-v3" => Ok(PoolType::SushiSwapV3 {
            fee_tier: fee_tier.ok_or_else(|| eyre!("--fee-tier is required for SushiSwap V3"))?,
        }),
        "pancakeswap-v3" | "pancake-v3" => Ok(PoolType::PancakeSwapV3 {
            fee_tier: fee_tier.ok_or_else(|| eyre!("--fee-tier is required for PancakeSwap V3"))?,
        }),
        other => Err(eyre!("unsupported protocol {other}")),
    }
}
