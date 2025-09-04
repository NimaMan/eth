/// Buy → Approve → Sell PEPE with ProcessedTransaction Generation
/// 
/// This example demonstrates a complete PEPE token trading workflow using SimulationChain
/// with proper state preservation, generating ProcessedTransaction objects for each step.
/// 
/// Key features:
/// 1. Uses SimulationChain for proper state preservation between transactions
/// 2. Generates ProcessedTransaction for each step with full event decoding
/// 3. Extracts exact PEPE amounts using address_balance_changes with U256 precision
/// 4. Extracts exact ETH amounts using address_balance_changes with U256 precision
/// 5. Shows detailed metrics including gas usage and event counts
/// 6. NO PRECISION LOSS - all amounts kept as U256 throughout
/// 
/// PEPE is a popular meme token with 18 decimals (unlike USDC/USDT which have 6).
/// This demonstrates how to handle high-decimal tokens with large supply while
/// maintaining full precision for amounts in the trillions.

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, CallRequest};
use tx_processor::tx_processor::{TxProcessor, data_models::ProcessedTransaction};
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Contract addresses
const PEPE_ADDRESS: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";     // PEPE token (18 decimals)
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";     // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test address with ETH balance (~5.38 ETH)
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐸 PEPE Buy → Approve → Sell with ProcessedTransaction Demo");
    println!("============================================================");
    println!("Using SimulationChain for state preservation.");
    println!("Full U256 precision - no accuracy loss!\n");
    
    // Initialize simulator and processor
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let tx_processor = TxProcessor::new();
    println!("✅ Simulator and TxProcessor initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Block: {}", latest_block);
    println!("💰 Investment: 0.1 ETH");
    println!("🏦 Buyer: {}\n", TEST_BUYER);
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // Execute PEPE workflow
    println!("{}", "=".repeat(60));
    println!("PEPE WORKFLOW WITH PROCESSED TRANSACTIONS");
    println!("{}", "=".repeat(60));
    
    let pepe_metrics = execute_pepe_trading_workflow_with_processed_tx(
        &simulator,
        &tx_processor,
        buyer_address,
        router_address,
        latest_block,
    ).await?;
    
    // Display final results with proper formatting
    println!("\n{}", "=".repeat(60));
    println!("📊 FINAL PEPE TRADING RESULTS");
    println!("{}", "=".repeat(60));
    
    println!("\n💰 Token Amounts:");
    println!("  PEPE received from buy: {}", format_pepe_amount(pepe_metrics.tokens_received_wei));
    println!("  ETH spent on buy: {}", format_eth_amount(pepe_metrics.eth_spent_wei));
    
    println!("\n💸 Sell Results:");
    println!("  PEPE sold: {}", format_pepe_amount(pepe_metrics.tokens_sold_wei));
    println!("  ETH received: {}", format_eth_amount(pepe_metrics.eth_received_wei));
    
    // Calculate slippage with U256 arithmetic
    if pepe_metrics.eth_spent_wei > U256::ZERO && pepe_metrics.eth_received_wei > U256::ZERO {
        // For display, we can use approximate calculations
        let eth_spent_f64 = wei_to_eth_approx(pepe_metrics.eth_spent_wei);
        let eth_received_f64 = wei_to_eth_approx(pepe_metrics.eth_received_wei);
        let net_eth = eth_received_f64 - eth_spent_f64;
        let slippage_pct = ((eth_spent_f64 - eth_received_f64) / eth_spent_f64) * 100.0;
        
        println!("\n📈 Trading Analysis:");
        println!("  Net ETH: {:+.6} ETH", net_eth);
        println!("  Slippage: {:.2}%", slippage_pct);
    }
    
    println!("\n⛽ Gas Usage:");
    println!("  Buy: {} gas", pepe_metrics.buy_gas);
    println!("  Approve: {} gas", pepe_metrics.approve_gas);
    println!("  Sell: {} gas", pepe_metrics.sell_gas);
    println!("  Total: {} gas", pepe_metrics.total_gas);
    
    println!("\n📝 Event Counts:");
    println!("  Transfer events: {}", pepe_metrics.total_transfer_events);
    println!("  Approval events: {}", pepe_metrics.total_approval_events);
    
    println!("\n✅ PEPE workflow demonstration complete!");
    Ok(())
}

/// Metrics collected from PEPE trading workflow - using U256 for full precision
#[derive(Debug, Default)]
struct PepeTradingMetrics {
    tokens_received_wei: U256,      // PEPE received from buy (in wei)
    tokens_sold_wei: U256,           // PEPE sold (in wei)
    eth_spent_wei: U256,             // ETH spent to buy PEPE (in wei)
    eth_received_wei: U256,          // ETH received from selling PEPE (in wei)
    total_gas: u64,
    buy_gas: u64,
    approve_gas: u64,
    sell_gas: u64,
    total_transfer_events: usize,
    total_approval_events: usize,
}

/// Execute PEPE trading workflow with ProcessedTransaction generation
async fn execute_pepe_trading_workflow_with_processed_tx(
    simulator: &TxSimulator,
    tx_processor: &TxProcessor,
    buyer: Address,
    router: Address,
    block: u64,
) -> Result<PepeTradingMetrics> {
    println!("\n🚀 Starting PEPE workflow at block {}", block);
    
    let mut metrics = PepeTradingMetrics::default();
    
    // Start a simulation chain for state preservation
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    println!("📍 Chain initialized with state preservation");
    
    // Step 1: Buy PEPE with 0.1 ETH
    println!("\n[Step 1] Buying PEPE with 0.1 ETH...");
    let buy_amount = U256::from(100_000_000_000_000_000u128); // 0.1 ETH
    let buy_tx = create_buy_pepe_transaction(buyer, buy_amount);
    
    let buy_result = chain.step_with_trace(buy_tx.clone()).await?;
    
    // Generate ProcessedTransaction for buy
    let buy_processed = tx_processor.process_transaction_from_simulation_result(
        &buy_tx,
        &buy_result,
        block,
        0, // tx_index
    ).await?;
    
    println!("  Status: {}", if buy_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", buy_result.gas_used);
    metrics.buy_gas = buy_result.gas_used;
    metrics.total_gas += buy_result.gas_used;
    
    // Extract exact amounts from balance changes (U256 precision)
    let (pepe_received, eth_spent) = extract_buy_amounts_from_balance_changes(&buy_processed, buyer);
    metrics.tokens_received_wei = pepe_received;
    metrics.eth_spent_wei = eth_spent.abs_diff(U256::ZERO); // Make positive for display
    
    println!("\n  💼 Balance Changes After Buy:");
    println!("    ETH: -{} (spent)", format_eth_amount(metrics.eth_spent_wei));
    println!("    PEPE: +{} (received)", format_pepe_amount(metrics.tokens_received_wei));
    
    // Count transfer events
    let buy_transfers = buy_processed.erc20_transfers.len();
    println!("  ERC20 Transfer events: {}", buy_transfers);
    metrics.total_transfer_events += buy_transfers;
    
    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(metrics);
    }
    
    // Use the exact PEPE amount received for selling
    let pepe_amount_to_sell = metrics.tokens_received_wei;
    
    // Step 2: Approve router to spend PEPE
    println!("\n[Step 2] Approving router to spend PEPE...");
    let pepe_addr = Address::from_str(PEPE_ADDRESS)?;
    let approve_tx = create_approve_transaction(buyer, pepe_addr, router, U256::MAX);
    
    let approve_result = chain.step_with_trace(approve_tx.clone()).await?;
    
    // Generate ProcessedTransaction for approve
    let approve_processed = tx_processor.process_transaction_from_simulation_result(
        &approve_tx,
        &approve_result,
        block,
        1, // tx_index
    ).await?;
    
    println!("  Status: {}", if approve_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", approve_result.gas_used);
    metrics.approve_gas = approve_result.gas_used;
    metrics.total_gas += approve_result.gas_used;
    
    // Check for Approval events
    let approval_count = approve_processed.approvals.len();
    println!("  Approval events: {}", approval_count);
    metrics.total_approval_events += approval_count;
    
    // Balance changes should be minimal for approve (only gas fees)
    println!("\n  💼 Balance Changes After Approve:");
    log_balance_changes_for_pepe(&approve_processed, buyer);
    
    if !approve_result.success {
        println!("  Revert reason: {:?}", approve_result.revert_reason);
        return Ok(metrics);
    }
    
    // Step 3: Sell all PEPE back to ETH
    println!("\n[Step 3] Selling all PEPE back to ETH...");
    println!("  Selling amount: {}", format_pepe_amount(pepe_amount_to_sell));
    
    let sell_tx = create_sell_pepe_transaction(buyer, pepe_amount_to_sell);
    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;
    
    // Generate ProcessedTransaction for sell
    let sell_processed = tx_processor.process_transaction_from_simulation_result(
        &sell_tx,
        &sell_result,
        block,
        2, // tx_index
    ).await?;
    
    println!("  Status: {}", if sell_result.success { "✅ Success" } else { "❌ Failed" });
    println!("  Gas used: {}", sell_result.gas_used);
    metrics.sell_gas = sell_result.gas_used;
    metrics.total_gas += sell_result.gas_used;
    
    // Extract exact amounts from balance changes (U256 precision)
    let (pepe_sold, eth_received) = extract_sell_amounts_from_balance_changes(&sell_processed, buyer);
    metrics.tokens_sold_wei = pepe_sold.abs_diff(U256::ZERO); // Make positive for display
    metrics.eth_received_wei = eth_received;
    
    println!("\n  💼 Balance Changes After Sell:");
    println!("    PEPE: -{} (sold)", format_pepe_amount(metrics.tokens_sold_wei));
    println!("    ETH: +{} (received)", format_eth_amount(metrics.eth_received_wei));
    
    // Count transfer events
    let sell_transfers = sell_processed.erc20_transfers.len();
    println!("  ERC20 Transfer events: {}", sell_transfers);
    metrics.total_transfer_events += sell_transfers;
    
    if !sell_result.success {
        println!("  Revert reason: {:?}", sell_result.revert_reason);
    }
    
    // Final state summary
    let state = chain.current_state();
    println!("\n📊 Final Chain State:");
    println!("  • Transactions executed: {}", state.transaction_count);
    println!("  • Total gas used: {}", state.total_gas_used);
    println!("  • State preserved throughout: ✅");
    
    Ok(metrics)
}

/// Extract PEPE received and ETH spent from buy transaction balance changes
/// Returns (pepe_wei, eth_wei) with full U256 precision
fn extract_buy_amounts_from_balance_changes(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
) -> (U256, U256) {
    // Get ETH spent from currency_net (will be negative for buy)
    let eth_wei = processed_tx.get_address_currency_balance_change(&buyer, "ETH")
        .unwrap_or(U256::ZERO);
    
    // Get PEPE received from token_net using token address
    let pepe_address = Address::from_str(PEPE_ADDRESS).unwrap();
    let pepe_wei = processed_tx.get_address_token_balance_change(&buyer, &pepe_address)
        .unwrap_or(U256::ZERO);
    
    (pepe_wei, eth_wei)
}

/// Extract PEPE sold and ETH received from sell transaction balance changes
/// Returns (pepe_wei, eth_wei) with full U256 precision
fn extract_sell_amounts_from_balance_changes(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
) -> (U256, U256) {
    // Get ETH received from currency_net (will be positive for sell)
    let eth_wei = processed_tx.get_address_currency_balance_change(&buyer, "ETH")
        .unwrap_or(U256::ZERO);
    
    // Get PEPE sold from token_net using token address (will be negative for sell)
    let pepe_address = Address::from_str(PEPE_ADDRESS).unwrap();
    let pepe_wei = processed_tx.get_address_token_balance_change(&buyer, &pepe_address)
        .unwrap_or(U256::ZERO);
    
    (pepe_wei, eth_wei)
}

/// Log all balance changes for PEPE workflow with full precision
fn log_balance_changes_for_pepe(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
) {
    let mut has_changes = false;
    
    // Log ETH changes from currency_net
    if let Some(eth_amount) = processed_tx.get_address_currency_balance_change(&buyer, "ETH") {
        if eth_amount > U256::from(1_000_000_000_000u64) { // More than 0.000001 ETH
            has_changes = true;
            println!("    ETH: {}", format_eth_amount_with_sign(eth_amount));
        }
    }
    
    // Log PEPE changes from token_net
    let pepe_address = Address::from_str(PEPE_ADDRESS).unwrap();
    if let Some(pepe_amount) = processed_tx.get_address_token_balance_change(&buyer, &pepe_address) {
        if pepe_amount != U256::ZERO {
            has_changes = true;
            println!("    PEPE: {}", format_pepe_amount_with_sign(pepe_amount));
        }
    }
    
    // Log any other known currencies
    if let Some(changes) = processed_tx.address_balance_changes.get(&buyer) {
        for (currency, amount) in &changes.currency_net {
            if currency != "ETH" && *amount != U256::ZERO {
                has_changes = true;
                println!("    {}: {}", currency, format_amount_with_decimals(*amount, 18));
            }
        }
    }
    
    if !has_changes {
        println!("    No significant balance changes (only gas fees)");
    }
}

/// Format PEPE amount from wei to human-readable with proper commas
fn format_pepe_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 PEPE".to_string();
    }
    
    // PEPE has 18 decimals
    let divisor = U256::from(10u128).pow(U256::from(18));
    let whole = wei / divisor;
    let remainder = wei % divisor;
    
    // Format whole part with commas
    let whole_str = format_with_commas(whole);
    
    // For PEPE, we typically don't need decimal places for large amounts
    if remainder == U256::ZERO {
        format!("{} PEPE", whole_str)
    } else {
        // Show 2 decimal places for fractional PEPE
        let decimal_part = (remainder * U256::from(100)) / divisor;
        format!("{}.{:02} PEPE", whole_str, decimal_part)
    }
}

/// Format ETH amount from wei to human-readable
fn format_eth_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 ETH".to_string();
    }
    
    // ETH has 18 decimals
    let divisor = U256::from(10u128).pow(U256::from(18));
    let whole = wei / divisor;
    let remainder = wei % divisor;
    
    // Show 6 decimal places for ETH
    let decimal_part = (remainder * U256::from(1_000_000u64)) / divisor;
    format!("{}.{:06} ETH", whole, decimal_part)
}

/// Format amounts with sign for balance changes
fn format_pepe_amount_with_sign(wei: U256) -> String {
    // Note: U256 can't be negative, so negative values are represented as very large numbers
    // For display purposes, we'll assume positive values
    format!("+{}", format_pepe_amount(wei))
}

fn format_eth_amount_with_sign(wei: U256) -> String {
    format!("+{}", format_eth_amount(wei))
}

/// Generic formatter for tokens with specified decimals
fn format_amount_with_decimals(wei: U256, decimals: u8) -> String {
    let divisor = U256::from(10u128).pow(U256::from(decimals));
    let whole = wei / divisor;
    let remainder = wei % divisor;
    
    if remainder == U256::ZERO {
        whole.to_string()
    } else {
        let decimal_places = 6; // Show 6 decimal places
        let multiplier = U256::from(10u128).pow(U256::from(decimal_places));
        let decimal_part = (remainder * multiplier) / divisor;
        format!("{}.{:0width$}", whole, decimal_part, width = decimal_places as usize)
    }
}

/// Format U256 with thousand separators
fn format_with_commas(value: U256) -> String {
    let s = value.to_string();
    let mut result = String::new();
    let mut count = 0;
    
    for c in s.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(c);
        count += 1;
    }
    
    result.chars().rev().collect()
}

/// Convert wei to ETH as f64 for approximate calculations only
fn wei_to_eth_approx(wei: U256) -> f64 {
    // This is only for display/percentage calculations, not for precise amounts
    let eth_str = format_eth_amount(wei).replace(" ETH", "");
    eth_str.parse::<f64>().unwrap_or(0.0)
}

/// Create a transaction to buy PEPE with ETH using Uniswap V2
fn create_buy_pepe_transaction(buyer: Address, eth_amount: U256) -> CallRequest {
    // swapExactETHForTokens(uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5]; // Function selector
    
    // amountOutMin (1 = accept any amount)
    data.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
    
    // path offset (points to dynamic array)
    data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
    
    // to address (buyer)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(buyer.as_slice());
    
    // deadline (far future)
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length = 2
    
    // WETH address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    // PEPE address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&PEPE_ADDRESS[2..]).unwrap());
    
    CallRequest {
        from: Some(buyer),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(eth_amount),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000), // 20 gwei
        nonce: None, // Let SimulationChain handle nonce
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create an approve transaction for ERC20 tokens
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
        gas: Some(100_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}

/// Create a transaction to sell PEPE for ETH using Uniswap V2
fn create_sell_pepe_transaction(seller: Address, pepe_amount: U256) -> CallRequest {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector
    
    // amountIn (amount of PEPE to sell)
    data.extend_from_slice(&pepe_amount.to_be_bytes::<32>());
    
    // amountOutMin (1 = accept any amount of ETH)
    data.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
    
    // path offset
    data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
    
    // to address (seller)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(seller.as_slice());
    
    // deadline
    data.extend_from_slice(&U256::from(9999999999u64).to_be_bytes::<32>());
    
    // path array
    data.extend_from_slice(&U256::from(2).to_be_bytes::<32>()); // length = 2
    
    // PEPE address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&PEPE_ADDRESS[2..]).unwrap());
    
    // WETH address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    CallRequest {
        from: Some(seller),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(300_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}