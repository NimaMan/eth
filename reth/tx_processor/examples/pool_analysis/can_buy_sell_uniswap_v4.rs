use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::simulator::types::UniswapV4PoolConfig;
use tx_processor::simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Uniswap V4 USDC/WETH buy → approve → sell simulation\n");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("Simulating at block {latest_block}");

    let pool_manager = Address::from_str("0x000000000004444C5DC75cB358380d2E3de08a90")?;
    let pool_id =
        B256::from_str("0x6d4bc5556c4b1b0d13d58f710e6de12b1d7a0711ef2b95dbf8507e96932162fa")?;

    let usdc = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    let weth = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let hooks = Address::from_str("0x36FABF0DaCD49E94dDb3A21999F199068a9Fe8a8")?;

    let v4_config = UniswapV4PoolConfig {
        pool_manager,
        pool_id,
        currency0: usdc,
        currency1: weth,
        fee: 1_000,
        tick_spacing: 1,
        hooks,
        hook_data: Vec::new(),
    };

    let config = PoolBuySellParameters {
        token_address: usdc,
        pool_address: pool_manager,
        pool_type: PoolType::UniswapV4,
        test_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH
        buyer_address: Address::from([
            0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D,
            0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89,
        ]),
        prior_txs: Vec::new(),
        block_number: Some(latest_block),
        slippage_tolerance: 5.0,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        buy_gas_limit: 800_000,
        approve_gas_limit: 250_000,
        sell_gas_limit: 800_000,
        weth_address: weth,
        denom_address: Some(weth),
        block_delay: 0,
        token_decimals: 6,
        block_header: None,
        uniswap_v4_config: Some(v4_config),
    };

    match check_can_buy_sell_pool(simulator, processor, config).await {
        Ok(res) => {
            println!("can_buy: {}", res.can_buy);
            println!("can_approve: {}", res.can_approve);
            println!("can_sell: {}", res.can_sell);
            println!("buy_tax: {:.4}%", res.buy_tax_percent);
            println!("sell_tax: {:.4}%", res.sell_tax_percent);
            println!("tokens received: {}", res.tokens_received);
            println!("eth received: {}", res.denom_received);
            if let Some(reason) = res.failure_reason {
                println!("failure_reason: {reason}");
            }
            if res.is_tradeable
                && res.tokens_received > U256::ZERO
                && res.denom_received > U256::ZERO
            {
                println!("✅ Baygus router swap path succeeded end-to-end.");
            } else {
                eprintln!(
                    "❌ Baygus router pipeline incomplete (tradeable={}, tokens={}, eth={}).",
                    res.is_tradeable, res.tokens_received, res.denom_received
                );
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("Simulation failed: {err:?}");
            std::process::exit(1);
        }
    }

    Ok(())
}
