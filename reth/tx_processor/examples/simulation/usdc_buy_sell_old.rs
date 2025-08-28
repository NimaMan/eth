/// USDC vs USDT Comparison Example
/// 
/// This example compares buying and selling USDC vs USDT:
/// 1. Buy USDC with 1 ETH
/// 2. Buy USDT with 1 ETH  
/// 3. Sell 10% of each token back to ETH
/// 4. Compare the differences in execution

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_processor::{RethTxSimulator, CallRequest, SequentialSimulationOptions, TxProcessor};
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Well-known contract addresses
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";      // USDC token
const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";      // USDT token
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";      // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test addresses
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";       // Has ETH for testing

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 USDC vs USDT Comparison");
    println!("===========================");
    
    // Initialize simulator
    let simulator = RethTxSimulator::new(RETH_DB_PATH)?;
    println!("✅ Simulator initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Latest block: {}", latest_block);
    println!("ETH Amount: 1 ETH per token\n");
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let usdc_address = Address::from_str(USDC_ADDRESS)?;
    let usdt_address = Address::from_str(USDT_ADDRESS)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // Simulate USDC buy/sell
    println!("📊 USDC SIMULATION");
    println!("==================");
    
    let usdc_buy_tx = create_buy_token_transaction(buyer_address, USDC_ADDRESS, U256::from(1_000_000_000_000_000_000u128));
    let usdc_buy_result = simulator.simulate_unsigned_transaction_with_full_trace_at_block(usdc_buy_tx.clone(), latest_block).await?;
    
    let mut usdc_received = U256::ZERO;
    let mut usdc_buy_gas = 0u64;
    let mut usdc_received_float = 0.0;
    
    if usdc_buy_result.success {
        usdc_buy_gas = usdc_buy_result.gas_used;
        if let Some(buyer_changes) = usdc_buy_result.address_balance_changes.get(&buyer_address) {
            if let Some(usdc_amount) = buyer_changes.token_net.get("USDC") {
                // Convert I256 to f64
                let usdc_str = usdc_amount.to_string();
                let usdc_amount_i128: i128 = usdc_str.parse().unwrap_or(0);
                usdc_received_float = usdc_amount_i128 as f64 / 1_000_000.0;
                usdc_received = U256::from(usdc_amount_i128.unsigned_abs());
                println!("Buy: Received {:.2} USDC for 1 ETH", usdc_received_float);
                println!("     Gas used: {}", usdc_buy_gas);
            }
        }
    }
    
    // Simulate USDC sell (10%)
    let usdc_sell_amount = usdc_received / U256::from(10);
    let usdc_sequence = vec![
        usdc_buy_tx,
        create_approve_transaction(buyer_address, usdc_address, router_address, U256::MAX),
        create_sell_token_transaction(buyer_address, USDC_ADDRESS, usdc_sell_amount),
    ];
    
    let options = SequentialSimulationOptions {
        at_block: Some(latest_block),
        stop_on_failure: false,
        auto_increment_nonces: true,
        gas_limit_per_tx: Some(300000),
    };
    
    let usdc_seq_result = simulator.simulate_transaction_sequence(usdc_sequence, options.clone()).await?;
    
    let mut usdc_eth_received = 0.0;
    let mut usdc_sell_gas = 0u64;
    
    if usdc_seq_result.results.len() >= 3 && usdc_seq_result.results[2].success {
        usdc_sell_gas = usdc_seq_result.results[2].gas_used;
        if let Some(buyer_changes) = usdc_seq_result.results[2].address_balance_changes.get(&buyer_address) {
            let eth_str = buyer_changes.eth_net.to_string();
            let eth_change: i128 = eth_str.parse().unwrap_or(0);
            usdc_eth_received = eth_change as f64 / 1e18;
            println!("Sell: Sold {:.2} USDC for {:.6} ETH", usdc_sell_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1_000_000.0, usdc_eth_received);
            println!("      Gas used: {}", usdc_sell_gas);
        }
    }
    
    println!();
    
    // Simulate USDT buy/sell
    println!("📊 USDT SIMULATION");
    println!("==================");
    
    let usdt_buy_tx = create_buy_token_transaction(buyer_address, USDT_ADDRESS, U256::from(1_000_000_000_000_000_000u128));
    let usdt_buy_result = simulator.simulate_unsigned_transaction_with_full_trace_at_block(usdt_buy_tx.clone(), latest_block).await?;
    
    let mut usdt_received = U256::ZERO;
    let mut usdt_buy_gas = 0u64;
    let mut usdt_received_float = 0.0;
    
    if usdt_buy_result.success {
        usdt_buy_gas = usdt_buy_result.gas_used;
        if let Some(buyer_changes) = usdt_buy_result.address_balance_changes.get(&buyer_address) {
            if let Some(usdt_amount) = buyer_changes.token_net.get("USDT") {
                // Convert I256 to f64
                let usdt_str = usdt_amount.to_string();
                let usdt_amount_i128: i128 = usdt_str.parse().unwrap_or(0);
                usdt_received_float = usdt_amount_i128 as f64 / 1_000_000.0;
                usdt_received = U256::from(usdt_amount_i128.unsigned_abs());
                println!("Buy: Received {:.2} USDT for 1 ETH", usdt_received_float);
                println!("     Gas used: {}", usdt_buy_gas);
            }
        }
    }
    
    // Simulate USDT sell (10%)
    let usdt_sell_amount = usdt_received / U256::from(10);
    let usdt_sequence = vec![
        usdt_buy_tx,
        create_approve_transaction(buyer_address, usdt_address, router_address, U256::MAX),
        create_sell_token_transaction(buyer_address, USDT_ADDRESS, usdt_sell_amount),
    ];
    
    let usdt_seq_result = simulator.simulate_transaction_sequence(usdt_sequence, options).await?;
    
    let mut usdt_eth_received = 0.0;
    let mut usdt_sell_gas = 0u64;
    
    if usdt_seq_result.results.len() >= 3 && usdt_seq_result.results[2].success {
        usdt_sell_gas = usdt_seq_result.results[2].gas_used;
        if let Some(buyer_changes) = usdt_seq_result.results[2].address_balance_changes.get(&buyer_address) {
            let eth_str = buyer_changes.eth_net.to_string();
            let eth_change: i128 = eth_str.parse().unwrap_or(0);
            usdt_eth_received = eth_change as f64 / 1e18;
            println!("Sell: Sold {:.2} USDT for {:.6} ETH", usdt_sell_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1_000_000.0, usdt_eth_received);
            println!("      Gas used: {}", usdt_sell_gas);
        }
    }
    
    println!("\n📈 COMPARISON");
    println!("=============");
    println!("Token Amounts:");
    println!("  USDC received: {:.6} | USDT received: {:.6}", usdc_received_float, usdt_received_float);
    println!("  Difference: {:.6} ({:.2}%)", 
        usdt_received_float - usdc_received_float,
        ((usdt_received_float - usdc_received_float) / usdc_received_float) * 100.0
    );
    
    println!("\nETH Returns (selling 10%):");
    println!("  USDC → ETH: {:.6} | USDT → ETH: {:.6}", usdc_eth_received, usdt_eth_received);
    println!("  Difference: {:.6} ({:.2}%)", 
        usdt_eth_received - usdc_eth_received,
        ((usdt_eth_received - usdc_eth_received) / usdc_eth_received) * 100.0
    );
    
    println!("\nGas Usage:");
    println!("  USDC buy: {} | USDT buy: {} (diff: {})", usdc_buy_gas, usdt_buy_gas, (usdt_buy_gas as i64 - usdc_buy_gas as i64));
    println!("  USDC sell: {} | USDT sell: {} (diff: {})", usdc_sell_gas, usdt_sell_gas, (usdt_sell_gas as i64 - usdc_sell_gas as i64));
    
    println!("\n💡 Key Insights:");
    if usdt_received_float > usdc_received_float {
        println!("  - USDT has better liquidity (more tokens per ETH)");
    } else {
        println!("  - USDC has better liquidity (more tokens per ETH)");
    }
    
    if usdt_eth_received > usdc_eth_received {
        println!("  - USDT has better sell rates");
    } else {
        println!("  - USDC has better sell rates");
    }
    
    Ok(())
}

/// Create a transaction to buy a token with ETH
fn create_buy_token_transaction(buyer: Address, token_address: &str, eth_amount: U256) -> CallRequest {
    // swapExactETHForTokens(uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let _function_selector = "7ff36ab5";
    let amount_out_min = U256::ZERO; // Accept any amount
    let deadline = U256::from(9999999999u64);
    
    // Encode the calldata
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5]; // Function selector
    
    // amountOutMin
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    
    // path offset
    data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
    
    // to address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(buyer.as_slice());
    
    // deadline
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length
    
    // WETH
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    // Target token
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&token_address[2..]).unwrap());
    
    CallRequest {
        from: Some(buyer),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(eth_amount),
        data: Some(Bytes::from(data)),
        gas: Some(300000),
        gas_price: Some(20_000_000_000), // 20 gwei
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create approve transaction
fn create_approve_transaction(from: Address, token: Address, spender: Address, amount: U256) -> CallRequest {
    // approve(address spender, uint256 amount)
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // approve selector
    
    // spender address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
    
    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    
    CallRequest {
        from: Some(from),
        to: Some(token),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(100000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create a transaction to sell a token for ETH
fn create_sell_token_transaction(seller: Address, token_address: &str, token_amount: U256) -> CallRequest {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let _function_selector = "18cbafe5";
    let amount_out_min = U256::ZERO; // Accept any amount
    let deadline = U256::from(9999999999u64);
    
    // Encode the calldata
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector
    
    // amountIn
    data.extend_from_slice(&token_amount.to_be_bytes::<32>());
    
    // amountOutMin
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    
    // path offset
    data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
    
    // to address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(seller.as_slice());
    
    // deadline
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length
    
    // Source token
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&token_address[2..]).unwrap());
    
    // WETH
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    CallRequest {
        from: Some(seller),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(300000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}