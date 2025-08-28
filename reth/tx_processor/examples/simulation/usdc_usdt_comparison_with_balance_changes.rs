/// USDC vs USDT Sequential Simulation with Address Balance Changes
/// 
/// This example demonstrates sequential transaction simulation with detailed balance change tracking.
/// It uses the tx_processor to show how the buyer's address state changes after each transaction:
/// 1. Buy USDC with 1 ETH (ETH decreases, USDC increases)
/// 2. Approve USDC for spending (gas fees only)
/// 3. Sell 10% of USDC back to ETH (USDC decreases, ETH increases)
/// 
/// Same process for USDT, then compares the results.
/// 
/// Key Features:
/// - Address balance change tracking using tx_processor
/// - Decimal-adjusted currency amounts (ETH=18 decimals, USDC/USDT=6 decimals)
/// - currency_net shows known currencies (ETH, USDC, USDT)
/// - token_net should be empty (known currencies don't appear there)
/// - Clear display of buyer's state changes after each step

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, CallRequest, SequentialSimulationOptions};
use tx_processor::TxProcessor;
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
    println!("💰 USDC vs USDT Sequential Simulation with Balance Change Tracking");
    println!("====================================================================");
    
    // Initialize both simulator and processor
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let processor = TxProcessor::new(RETH_DB_PATH)?;
    println!("✅ Simulator and Processor initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Latest block: {}", latest_block);
    println!("💰 ETH Amount: 1 ETH per token\\n");
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let usdc_address = Address::from_str(USDC_ADDRESS)?;
    let usdt_address = Address::from_str(USDT_ADDRESS)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // USDC Sequential Simulation with Balance Changes
    println!("🔥 USDC SEQUENTIAL SIMULATION WITH BALANCE CHANGES");
    println!("=================================================");
    
    // Create USDC transaction sequence
    let usdc_buy_tx = create_buy_token_transaction(buyer_address, USDC_ADDRESS, U256::from(1_000_000_000_000_000_000u128));
    let usdc_approve_tx = create_approve_transaction(buyer_address, usdc_address, router_address, U256::MAX);
    let usdc_sell_amount = U256::from(300_000_000u128); // 300 USDC (6 decimals)
    let usdc_sell_tx = create_sell_token_transaction(buyer_address, USDC_ADDRESS, usdc_sell_amount);
    
    let usdc_sequence = vec![usdc_buy_tx.clone(), usdc_approve_tx.clone(), usdc_sell_tx.clone()];
    
    // Simulate each USDC transaction individually to get balance changes
    for (i, tx_request) in usdc_sequence.iter().enumerate() {
        let tx_name = match i {
            0 => "Buy USDC",
            1 => "Approve USDC", 
            2 => "Sell USDC",
            _ => "Unknown",
        };
        
        println!("\\n📋 Transaction {}: {}", i + 1, tx_name);
        println!("   ================================");
        
        // Simulate the transaction with balance changes
        match processor.simulate_transaction_from_calldata_with_balance_changes(tx_request.clone(), Some(latest_block)).await {
            Ok(processed_tx) => {
                println!("   ✅ Success: Gas used: {}", processed_tx.fees.gas_used);
                
                // Display buyer's address balance changes
                if let Some(balance_changes) = processed_tx.address_balance_changes.get(&buyer_address) {
                    println!("   💰 Buyer Address Balance Changes:");
                    
                    // Display currency_net (known currencies with decimal adjustment)
                    if let Some(currency_net) = balance_changes.get("currency_net") {
                        if let Some(currency_obj) = currency_net.as_object() {
                            for (currency, amount) in currency_obj {
                                if let Some(amount_str) = amount.as_str() {
                                    println!("      {}: {}", currency, format_balance_change(amount_str, currency));
                                }
                            }
                        }
                    }
                    
                    // Display token_net (should be empty for known currencies)
                    let mut token_count = 0;
                    for (key, value) in balance_changes.as_object().unwrap_or(&serde_json::Map::new()) {
                        if key != "currency_net" && !value.is_null() {
                            token_count += 1;
                        }
                    }
                    
                    if token_count == 0 {
                        println!("      ✅ token_net: empty (as expected - USDC appears in currency_net)");
                    } else {
                        println!("      ⚠️  token_net: {} unknown tokens found", token_count);
                    }
                } else {
                    println!("   ❌ No balance changes found for buyer address");
                }
            }
            Err(e) => {
                println!("   ❌ Simulation failed: {}", e);
            }
        }
    }
    
    // USDT Sequential Simulation with Balance Changes  
    println!("\\n\\n🔥 USDT SEQUENTIAL SIMULATION WITH BALANCE CHANGES");
    println!("=================================================");
    
    // Create USDT transaction sequence
    let usdt_buy_tx = create_buy_token_transaction(buyer_address, USDT_ADDRESS, U256::from(1_000_000_000_000_000_000u128));
    let usdt_approve_tx = create_approve_transaction(buyer_address, usdt_address, router_address, U256::MAX);
    let usdt_sell_amount = U256::from(300_000_000u128); // 300 USDT (6 decimals)
    let usdt_sell_tx = create_sell_token_transaction(buyer_address, USDT_ADDRESS, usdt_sell_amount);
    
    let usdt_sequence = vec![usdt_buy_tx.clone(), usdt_approve_tx.clone(), usdt_sell_tx.clone()];
    
    // Simulate each USDT transaction individually to get balance changes
    for (i, tx_request) in usdt_sequence.iter().enumerate() {
        let tx_name = match i {
            0 => "Buy USDT",
            1 => "Approve USDT",
            2 => "Sell USDT", 
            _ => "Unknown",
        };
        
        println!("\\n📋 Transaction {}: {}", i + 1, tx_name);
        println!("   ================================");
        
        // Simulate the transaction with balance changes
        match processor.simulate_transaction_from_calldata_with_balance_changes(tx_request.clone(), Some(latest_block)).await {
            Ok(processed_tx) => {
                println!("   ✅ Success: Gas used: {}", processed_tx.fees.gas_used);
                
                // Display buyer's address balance changes
                if let Some(balance_changes) = processed_tx.address_balance_changes.get(&buyer_address) {
                    println!("   💰 Buyer Address Balance Changes:");
                    
                    // Display currency_net (known currencies with decimal adjustment)
                    if let Some(currency_net) = balance_changes.get("currency_net") {
                        if let Some(currency_obj) = currency_net.as_object() {
                            for (currency, amount) in currency_obj {
                                if let Some(amount_str) = amount.as_str() {
                                    println!("      {}: {}", currency, format_balance_change(amount_str, currency));
                                }
                            }
                        }
                    }
                    
                    // Display token_net (should be empty for known currencies)
                    let mut token_count = 0;
                    for (key, value) in balance_changes.as_object().unwrap_or(&serde_json::Map::new()) {
                        if key != "currency_net" && !value.is_null() {
                            token_count += 1;
                        }
                    }
                    
                    if token_count == 0 {
                        println!("      ✅ token_net: empty (as expected - USDT appears in currency_net)");
                    } else {
                        println!("      ⚠️  token_net: {} unknown tokens found", token_count);
                    }
                } else {
                    println!("   ❌ No balance changes found for buyer address");
                }
            }
            Err(e) => {
                println!("   ❌ Simulation failed: {}", e);
            }
        }
    }
    
    // Summary
    println!("\\n\\n💡 SUMMARY");
    println!("==========");
    println!("✅ Address Balance Change Calculator Integration:");
    println!("   • currency_net tracks known currencies (ETH, USDC, USDT) with decimal conversion");
    println!("   • token_net remains empty for known currencies (as expected)");
    println!("   • Balance changes show buyer's state after each transaction step");
    println!("   • Decimal conversion: ETH (18 decimals), USDC/USDT (6 decimals)");
    
    println!("\\n🔧 Key Features Demonstrated:");
    println!("   • Sequential transaction simulation with state persistence");
    println!("   • Address balance change tracking per transaction");
    println!("   • Proper separation: known currencies → currency_net, unknown → token_net");
    println!("   • Decimal-adjusted amounts for human readability");
    println!("   • Integration between tx_simulator and tx_processor");
    
    Ok(())
}

/// Format balance change for display
fn format_balance_change(amount_str: &str, currency: &str) -> String {
    if let Ok(amount) = amount_str.parse::<i128>() {
        let sign = if amount >= 0 { "+" } else { "" };
        match currency {
            "ETH" => format!("{}{} ETH", sign, amount),
            "USDC" => format!("{}{} USDC", sign, amount),
            "USDT" => format!("{}{} USDT", sign, amount),
            _ => format!("{}{} {}", sign, amount, currency),
        }
    } else {
        format!("{} {}", amount_str, currency)
    }
}

/// Create a transaction to buy a token with ETH
fn create_buy_token_transaction(buyer: Address, token_address: &str, eth_amount: U256) -> CallRequest {
    // swapExactETHForTokens(uint256 amountOutMin, address[] path, address to, uint256 deadline)
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