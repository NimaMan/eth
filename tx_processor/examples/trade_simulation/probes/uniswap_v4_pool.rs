use alloy_primitives::{Address, Bytes, B256, U256};
use clap::Parser;
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_processor::trade_simulation::types::UniswapV4PoolConfig;
use tx_processor::trade_simulation::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

const DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    token: Address,
    #[arg(long)]
    pool_manager: Address,
    #[arg(long)]
    pool_id: B256,
    #[arg(long)]
    currency0: Address,
    #[arg(long)]
    currency1: Address,
    #[arg(long)]
    denom: Address,
    #[arg(long, default_value_t = Address::ZERO)]
    hooks: Address,
    #[arg(long)]
    fee: u32,
    #[arg(long)]
    tick_spacing: i32,
    #[arg(long)]
    block: u64,
    #[arg(long, value_delimiter = ',', default_value = "10000000000000000")]
    amount_wei: Vec<String>,
    #[arg(long)]
    reth_datadir: Option<String>,
    #[arg(long)]
    buyer: Option<Address>,
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
    let token_decimals = erc20_decimals(&simulator, args.token, args.block)
        .await
        .unwrap_or(18);
    let denom_decimals = erc20_decimals(&simulator, args.denom, args.block)
        .await
        .unwrap_or(18);

    for block in [args.block.saturating_sub(1), args.block] {
        for amount in &args.amount_wei {
            let amount = U256::from_str_radix(amount, 10)
                .map_err(|error| eyre!("invalid --amount-wei value {amount}: {error}"))?;
            let params =
                PoolBuySellParameters::new(args.token, args.pool_manager, PoolType::UniswapV4)
                    .with_test_amount(amount)
                    .with_block(block)
                    .with_denom_address(args.denom)
                    .with_denom_decimals(denom_decimals)
                    .with_token_decimals(token_decimals)
                    .with_uniswap_v4_config(UniswapV4PoolConfig {
                        pool_manager: args.pool_manager,
                        pool_id: args.pool_id,
                        currency0: args.currency0,
                        currency1: args.currency1,
                        fee: args.fee,
                        tick_spacing: args.tick_spacing,
                        hooks: args.hooks,
                        hook_data: Vec::new(),
                    });
            let params = if let Some(buyer) = args.buyer {
                params.with_buyer(buyer)
            } else {
                params
            };

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
                    if args.verbose {
                        for (idx, tx) in result.prior_transactions.iter().enumerate() {
                            println!(
                                "  prior[{idx}] status={} type={} from={:#x} to={:?} actions={:?} erc20_transfers={:?} permit2_events={:?}",
                                tx.status,
                                tx.tx_type,
                                tx.from_address,
                                tx.to_address,
                                tx.actions,
                                tx.erc20_transfers,
                                tx.permit2_events
                            );
                        }
                        println!(
                            "  buy status={} type={} actions={:?} erc20_transfers={:?} permit2_events={:?}",
                            result.buy_transaction.status,
                            result.buy_transaction.tx_type,
                            result.buy_transaction.actions,
                            result.buy_transaction.erc20_transfers,
                            result.buy_transaction.permit2_events
                        );
                        println!(
                            "  approve status={} type={} actions={:?} erc20_transfers={:?} permit2_events={:?}",
                            result.approve_transaction.status,
                            result.approve_transaction.tx_type,
                            result.approve_transaction.actions,
                            result.approve_transaction.erc20_transfers,
                            result.approve_transaction.permit2_events
                        );
                        println!(
                            "  sell status={} type={} actions={:?} erc20_transfers={:?} permit2_events={:?}",
                            result.sell_transaction.status,
                            result.sell_transaction.tx_type,
                            result.sell_transaction.actions,
                            result.sell_transaction.erc20_transfers,
                            result.sell_transaction.permit2_events
                        );
                    }
                }
                Err(error) => {
                    println!("block={block} amount={amount} error={error:?}");
                }
            }
        }
    }

    Ok(())
}

async fn erc20_decimals(simulator: &Arc<TxSimulator>, token: Address, block: u64) -> Result<u8> {
    let result = simulator
        .simulate_view_function(token, Bytes::from_static(&DECIMALS_SELECTOR), Some(block))
        .await?;
    if !result.success || result.output.len() < 32 {
        return Err(eyre!("decimals() failed for {token:#x} at block {block}"));
    }
    Ok(result.decode_uint8())
}
