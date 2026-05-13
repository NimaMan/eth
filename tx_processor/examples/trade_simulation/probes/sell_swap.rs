use alloy_primitives::{Address, U256};
use clap::Parser;
use eyre::{eyre, Result};
use reth_chain_query::to_checksum_address;
use std::sync::Arc;
use tx_processor::trade_simulation::{
    simulate_sell_swap_with_params, PoolBuySellParameters, PoolType,
};
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
    #[arg(long)]
    amount_raw: String,
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
    #[arg(long)]
    seller: Option<Address>,
    #[arg(long)]
    verbose: bool,
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
    let amount = parse_u256(&args.amount_raw)?;
    let pool_type = parse_pool_type(&args.protocol, args.fee_tier)?;

    let base_params = PoolBuySellParameters::new(args.token, args.pool, pool_type)
        .with_block(args.block)
        .with_denom_address(args.denom)
        .with_denom_decimals(args.denom_decimals)
        .with_token_decimals(args.token_decimals);
    let base_params = if let Some(seller) = args.seller {
        base_params.with_buyer(seller)
    } else {
        base_params
    };

    for (label, sell_amount) in probe_amounts(amount) {
        if sell_amount.is_zero() {
            continue;
        }
        let result = simulate_sell_swap_with_params(
            simulator.clone(),
            tx_processor.clone(),
            base_params.clone(),
            sell_amount,
        )
        .await?;

        println!(
            "amount_label={label} amount_raw={sell_amount} success={} denom_received={} gas_used={} failure={}",
            result.success,
            result.denom_received,
            result.sell_transaction.fees.gas_used,
            result.failure_reason.as_deref().unwrap_or("-")
        );
        if args.verbose {
            print_transaction_details(&result.sell_transaction, args.pool, args.denom);
        }
    }

    Ok(())
}

fn print_transaction_details(
    tx: &tx_processor::tx_processor::data_models::ProcessedTransaction,
    pool: Address,
    denom: Address,
) {
    println!("  erc20_transfers:");
    for transfer in &tx.erc20_transfers {
        if transfer.from_address == pool
            || transfer.to_address == pool
            || transfer.token_address == denom
        {
            println!(
                "    token={} from={} to={} amount={} log_index={}",
                to_checksum_address(&transfer.token_address),
                to_checksum_address(&transfer.from_address),
                to_checksum_address(&transfer.to_address),
                transfer.amount,
                transfer.log_index
            );
        }
    }
    println!("  v2_swaps:");
    for swap in &tx.uniswap_v2_swaps {
        println!(
            "    pair={} amount0_in={} amount1_in={} amount0_out={} amount1_out={} to={} log_index={}",
            to_checksum_address(&swap.pair_address),
            swap.amount0_in,
            swap.amount1_in,
            swap.amount0_out,
            swap.amount1_out,
            to_checksum_address(&swap.to),
            swap.log_index
        );
    }
    println!("  balance_changes:");
    for (address, changes) in &tx.address_balance_changes {
        if *address == pool
            || changes
                .currency_net
                .iter()
                .any(|(symbol, amount)| symbol == "ETH" && !amount.is_zero())
        {
            println!(
                "    address={} currency_net={:?} token_net={:?}",
                to_checksum_address(address),
                changes.currency_net,
                changes.token_net
            );
        }
    }
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
        other => Err(eyre!("unsupported protocol {other}")),
    }
}

fn parse_u256(value: &str) -> Result<U256> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|err| eyre!("invalid hex amount {value}: {err}"))
    } else {
        U256::from_str_radix(value, 10)
            .map_err(|err| eyre!("invalid decimal amount {value}: {err}"))
    }
}

fn probe_amounts(amount: U256) -> Vec<(&'static str, U256)> {
    [
        ("100%", 1u64, 1u64),
        ("100%-1", 0, 0),
        ("99.9%", 999, 1000),
        ("99%", 99, 100),
        ("50%", 1, 2),
        ("25%", 1, 4),
        ("20%", 1, 5),
        ("10%", 1, 10),
        ("5%", 1, 20),
        ("2%", 1, 50),
        ("1%", 1, 100),
    ]
    .into_iter()
    .map(|(label, numerator, denominator)| {
        if label == "100%-1" {
            (label, amount.saturating_sub(U256::from(1)))
        } else {
            (
                label,
                amount.saturating_mul(U256::from(numerator)) / U256::from(denominator),
            )
        }
    })
    .collect()
}
