/// Buy → Approve → Sell FLOKI with ProcessedTransaction Generation
/// 
/// This example demonstrates a complete FLOKI token trading workflow using SimulationChain
/// with proper state preservation, generating ProcessedTransaction objects for each step.
/// 
/// This example demonstrates buying FLOKI with 1 ETH and selling the full amount received.
/// Testing with larger amounts to match successful on-chain transactions.
/// 
/// Key features:
/// 1. Uses SimulationChain for proper state preservation between transactions
/// 2. Generates ProcessedTransaction for each step with full event decoding
/// 3. Extracts exact FLOKI amounts using address_balance_changes with U256 precision
/// 4. Handles 9 decimal tokens (FLOKI has 9 decimals, not 18 like most tokens)
/// 5. Demonstrates handling of failed sell transactions
/// 6. NO PRECISION LOSS - all amounts kept as U256 throughout
/// 
/// FLOKI is a meme token with 9 decimals and potential trading restrictions.
/// This example shows how to properly handle tokens that may have sell limitations.

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTransaction;
use tx_processor::tx_processor::tax_calculator::{
    calculate_buy_tax_from_processed_transaction, TaxCalculationResult
};
use std::str::FromStr;
use hex;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Contract addresses
const FLOKI_ADDRESS: &str = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E";     // FLOKI token (9 decimals!)
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";      // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router
const FLOKI_WETH_POOL: &str = "0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0";     // FLOKI-WETH Uniswap V2 Pool
const FLOKI_DECIMALS: u8 = 9;  // FLOKI has 9 decimals, not 18!

// Test address with ETH balance (~5.38 ETH)
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐕 FLOKI Buy → Approve → Sell with ProcessedTransaction Demo");
    println!("=============================================================");
    println!("Using SimulationChain for state preservation.");
    println!("Full U256 precision - no accuracy loss!");
    println!("⚠️  Note: FLOKI may have sell restrictions!\n");
    
    // Initialize simulator and processor
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let tx_processor = TxProcessor::new();
    println!("✅ Simulator and TxProcessor initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Block: {}", latest_block);
    println!("💰 Investment: 1 ETH");
    println!("🏦 Buyer: {}\n", TEST_BUYER);
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // Execute FLOKI workflow
    println!("{}", "=".repeat(60));
    println!("FLOKI WORKFLOW WITH PROCESSED TRANSACTIONS");
    println!("{}", "=".repeat(60));
    
    let floki_metrics = execute_floki_trading_workflow(
        &simulator,
        &tx_processor,
        buyer_address,
        router_address,
        latest_block,
    ).await?;
    
    // Display final results with proper formatting
    println!("\n{}", "=".repeat(60));
    println!("📊 FINAL FLOKI TRADING RESULTS");
    println!("{}", "=".repeat(60));
    
    println!("\n💰 Token Amounts:");
    println!("  FLOKI received from buy: {}", format_floki_amount(floki_metrics.tokens_received_wei));
    println!("  ETH spent on buy: {}", format_eth_amount(floki_metrics.eth_spent_wei));
    
    println!("\n💸 Sell Results:");
    if floki_metrics.sell_succeeded {
        println!("  FLOKI sold: {}", format_floki_amount(floki_metrics.tokens_sold_wei));
        println!("  ETH received: {}", format_eth_amount(floki_metrics.eth_received_wei));
    } else {
        println!("  ❌ Sell transaction FAILED!");
        println!("  Attempted to sell: {}", format_floki_amount(floki_metrics.tokens_attempted_to_sell));
        println!("  ETH received: 0 ETH (transaction reverted)");
        if let Some(reason) = &floki_metrics.sell_failure_reason {
            println!("  Failure reason: {}", reason);
        }
    }
    
    // Calculate slippage only if sell succeeded
    if floki_metrics.sell_succeeded && floki_metrics.eth_spent_wei > U256::ZERO && floki_metrics.eth_received_wei > U256::ZERO {
        let eth_spent_f64 = wei_to_eth_approx(floki_metrics.eth_spent_wei);
        let eth_received_f64 = wei_to_eth_approx(floki_metrics.eth_received_wei);
        let net_eth = eth_received_f64 - eth_spent_f64;
        let slippage_pct = ((eth_spent_f64 - eth_received_f64) / eth_spent_f64) * 100.0;
        
        println!("\n📈 Trading Analysis:");
        println!("  Net ETH: {:+.6} ETH", net_eth);
        println!("  Slippage: {:.2}%", slippage_pct);
    } else if !floki_metrics.sell_succeeded {
        println!("\n⚠️  Trading Analysis:");
        println!("  Cannot calculate slippage - sell transaction failed");
        println!("  This may indicate:");
        println!("    • Token has sell restrictions or cooldown periods");
        println!("    • Token has high sell tax that causes reversion");
        println!("    • Liquidity issues in the pool");
        println!("    • Contract-level restrictions on selling");
    }
    
    println!("\n⛽ Gas Usage:");
    println!("  Buy: {} gas", floki_metrics.buy_gas);
    println!("  Approve: {} gas", floki_metrics.approve_gas);
    println!("  Sell: {} gas {}", 
            floki_metrics.sell_gas,
            if !floki_metrics.sell_succeeded { "(failed - gas still consumed)" } else { "" });
    println!("  Total: {} gas", floki_metrics.total_gas);
    
    println!("\n📝 Event Counts:");
    println!("  Transfer events: {}", floki_metrics.total_transfer_events);
    println!("  Approval events: {}", floki_metrics.total_approval_events);
    
    if !floki_metrics.sell_succeeded {
        println!("\n🔍 Debugging Tips for Failed Sell:");
        println!("  1. Check if token has cooldown period between buy and sell");
        println!("  2. Try smaller sell amounts (e.g., 50% of tokens)");
        println!("  3. Check if token has maximum sell amount per transaction");
        println!("  4. Review token contract for trading enable/disable functions");
        println!("  5. Check if selling requires special conditions or whitelist");
    }
    
    println!("\n✅ FLOKI workflow demonstration complete!");
    Ok(())
}

/// Metrics collected from FLOKI trading workflow - using U256 for full precision
#[derive(Debug, Default)]
struct FlokiTradingMetrics {
    tokens_received_wei: U256,          // FLOKI received from buy (in smallest units)
    tokens_sold_wei: U256,               // FLOKI sold (in smallest units)
    tokens_attempted_to_sell: U256,      // FLOKI we tried to sell
    eth_spent_wei: U256,                 // ETH spent to buy FLOKI (in wei)
    eth_received_wei: U256,              // ETH received from selling FLOKI (in wei)
    total_gas: u64,
    buy_gas: u64,
    approve_gas: u64,
    sell_gas: u64,
    sell_succeeded: bool,
    sell_failure_reason: Option<String>,
    total_transfer_events: usize,
    total_approval_events: usize,
}

/// Execute FLOKI trading workflow with ProcessedTransaction generation
async fn execute_floki_trading_workflow(
    simulator: &TxSimulator,
    tx_processor: &TxProcessor,
    buyer: Address,
    router: Address,
    block: u64,
) -> Result<FlokiTradingMetrics> {
    println!("\n🚀 Starting FLOKI workflow at block {}", block);
    
    let mut metrics = FlokiTradingMetrics::default();
    
    // Start a simulation chain for state preservation
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    println!("📍 Chain initialized with state preservation");
    
    // Step 1: Buy FLOKI with 1 ETH
    println!("\n[Step 1] Buying FLOKI with 1 ETH...");
    let buy_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
    let buy_tx = create_buy_floki_transaction(buyer, buy_amount);
    
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
    let (floki_received, eth_spent) = extract_buy_amounts_from_balance_changes(&buy_processed, buyer);
    
    // Extra instrumentation: verify token_net keys and values for FLOKI
    {
        use reth_chain_query::to_checksum_address;
        let floki_addr = Address::from_str(FLOKI_ADDRESS).unwrap();
        let checksum_key = to_checksum_address(&floki_addr);
        let debug_key = format!("{:?}", floki_addr);
        println!("\n  🔎 Instrumentation - token_net lookup variants:");
        if let Some(changes) = buy_processed.address_balance_changes.get(&buyer) {
            let token_net_len = changes.token_net.len();
            println!("    token_net entries: {}", token_net_len);
            if token_net_len > 0 {
                println!("    token_net keys sample (up to 5):");
                for (i, (k, v)) in changes.token_net.iter().take(5).enumerate() {
                    println!("      [{}] {} => {}", i, k, v);
                }
            }
            let val_checksum = changes.token_net.get(&checksum_key).copied().unwrap_or(U256::ZERO);
            let val_debug = changes.token_net.get(&debug_key).copied().unwrap_or(U256::ZERO);
            println!("    lookup by checksum  ({}): {}", checksum_key, val_checksum);
            println!("    lookup by {:?} (Debug): {}", floki_addr, val_debug);
        } else {
            println!("    No address_balance_changes found for buyer");
        }
    }
    metrics.tokens_received_wei = floki_received;
    metrics.eth_spent_wei = eth_spent.abs_diff(U256::ZERO); // Make positive for display
    
    // Debug: Show raw amounts
    println!("\n  🔍 Debug - Raw amounts:");
    println!("    FLOKI received (raw wei): {}", floki_received);
    println!("    Expected decimals: {}", FLOKI_DECIMALS);
    
    println!("\n  💼 Balance Changes After Buy:");
    println!("    ETH: -{} (spent)", format_eth_amount(metrics.eth_spent_wei));
    println!("    FLOKI: +{} (received)", format_floki_amount(metrics.tokens_received_wei));
    
    // Count transfer events
    let buy_transfers = buy_processed.erc20_transfers.len();
    println!("  ERC20 Transfer events: {}", buy_transfers);
    
    // Debug: Check ALL ERC20 transfers to understand the flow
    println!("\n  🔍 Debug - ALL ERC20 Transfers:");
    println!("    Total ERC20 transfers: {}", buy_processed.erc20_transfers.len());
    
    // Define key addresses for analysis
    let floki_address = Address::from_str(FLOKI_ADDRESS)?;
    let pool_address = Address::from_str(FLOKI_WETH_POOL)?;
    
    for (i, transfer) in buy_processed.erc20_transfers.iter().enumerate() {
        println!("\n    Transfer #{}: ", i + 1);
        println!("      Token: {}", transfer.token_address);
        println!("      From:  {}", transfer.from_address);
        println!("      To:    {}", transfer.to_address);
        println!("      Amount: {} (raw: {})", 
                 if transfer.token_address.to_string().to_lowercase() == FLOKI_ADDRESS.to_lowercase() {
                     format_floki_amount(transfer.amount)
                 } else {
                     format!("{} tokens", transfer.amount)
                 },
                 transfer.amount);
        
        // Identify key addresses
        if transfer.from_address == pool_address {
            println!("      ⭐ FROM: FLOKI-WETH Pool");
        } else if transfer.from_address == buyer {
            println!("      ⭐ FROM: Buyer");
        } else if transfer.from_address == Address::from_str(UNISWAP_V2_ROUTER)? {
            println!("      ⭐ FROM: Uniswap V2 Router");
        }
        
        if transfer.to_address == pool_address {
            println!("      ⭐ TO: FLOKI-WETH Pool");
        } else if transfer.to_address == buyer {
            println!("      ⭐ TO: Buyer");
        } else if transfer.to_address == Address::from_str(UNISWAP_V2_ROUTER)? {
            println!("      ⭐ TO: Uniswap V2 Router");
        }
        
        // Special analysis for FLOKI transfers
        if transfer.token_address.to_string().to_lowercase() == FLOKI_ADDRESS.to_lowercase() {
            println!("      🟡 This is a FLOKI transfer");
            
            if transfer.from_address == pool_address && transfer.to_address == buyer {
                println!("      💰 Direct pool → buyer transfer");
            } else if transfer.from_address == pool_address {
                println!("      📤 Pool sending FLOKI somewhere");
            } else if transfer.to_address == pool_address {
                println!("      📥 Pool receiving FLOKI from somewhere");
            }
        }
    }
    
    // Debug: Log ALL address balance changes to understand token flow
    println!("\n  🔍 Debug - ALL Address Balance Changes:");
    
    println!("    Key addresses:");
    println!("      Buyer: {}", buyer);
    println!("      Pool:  {}", pool_address);
    println!("      FLOKI: {}", floki_address);
    
    // Log every address that had balance changes
    for (address, changes) in &buy_processed.address_balance_changes {
        println!("\n    Address: {}", address);
        
        // ETH changes
        if !changes.currency_net.is_empty() {
            println!("      Currency changes:");
            for (currency, amount) in &changes.currency_net {
                let formatted_amount = if currency == "ETH" {
                    format_eth_amount(*amount)
                } else {
                    format!("{} {}", amount, currency)
                };
                println!("        {}: {} (raw: {})", currency, formatted_amount, amount);
                
                // Special check for negative values that might be displayed as positive
                println!("        🔍 Raw U256 analysis:");
                println!("          Hex: {:#x}", amount);
                println!("          Is > U256::MAX/2: {}", *amount > (U256::MAX / U256::from(2)));
                if *amount > (U256::MAX / U256::from(2)) {
                    let negative_amount = U256::MAX - *amount + U256::from(1);
                    println!("          Interpreted as negative: -{} ({})", 
                             if currency == "ETH" { format_eth_amount(negative_amount) } else { negative_amount.to_string() },
                             negative_amount);
                }
            }
        }
        
        // Token changes
        if !changes.token_net.is_empty() {
            println!("      Token changes:");
            for (token_key, amount) in &changes.token_net {
                let formatted_amount = if token_key.contains(&floki_address.to_string()[2..]) {
                    format_floki_amount(*amount)
                } else {
                    format!("{} tokens", amount)
                };
                println!("        {}: {} (raw: {})", token_key, formatted_amount, amount);
                
                // Special check for negative values that might be displayed as positive
                println!("        🔍 Raw U256 analysis:");
                println!("          Hex: {:#x}", amount);
                println!("          Is > U256::MAX/2: {}", *amount > (U256::MAX / U256::from(2)));
                if *amount > (U256::MAX / U256::from(2)) {
                    let negative_amount = U256::MAX - *amount + U256::from(1);
                    println!("          Interpreted as negative: -{} ({})", 
                             if token_key.contains(&floki_address.to_string()[2..]) {
                                 format_floki_amount(negative_amount)
                             } else {
                                 format!("{} tokens", negative_amount)
                             },
                             negative_amount);
                }
            }
        }
        
        // Special analysis for key addresses
        if *address == buyer {
            println!("      ⭐ This is the BUYER");
        } else if *address == pool_address {
            println!("      ⭐ This is the FLOKI-WETH POOL");
            
            // Deep analysis of pool balance changes
            if let Some(floki_change) = changes.token_net.get(&floki_address.to_string()) {
                println!("      🔍 POOL FLOKI ANALYSIS:");
                println!("        Raw value: {}", floki_change);
                println!("        Formatted: {}", format_floki_amount(*floki_change));
                println!("        Hex: {:#x}", floki_change);
                
                // Check if it's actually a negative value stored as positive
                if *floki_change > (U256::MAX / U256::from(2)) {
                    let actual_negative = U256::MAX - *floki_change + U256::from(1);
                    println!("        ✅ ACTUALLY NEGATIVE: -{} FLOKI", format_floki_amount(actual_negative));
                } else {
                    println!("        ❌ POSITIVE VALUE: Pool gained FLOKI (this is wrong!)");
                }
            }
        } else if *address == Address::from_str(UNISWAP_V2_ROUTER)? {
            println!("      ⭐ This is the Uniswap V2 Router");
        }
    }
    
    // Calculate buy tax using the tax calculator
    println!("\n  🔍 Debug - Buy Tax Analysis:");
    
    let buy_tax_result = calculate_buy_tax_from_processed_transaction(
        &buy_processed,
        pool_address,
        buyer,
        floki_address,
    );
    
    match buy_tax_result {
        TaxCalculationResult::Calculated { tax_basis_points } => {
            if let Some(tax_percentage) = buy_tax_result.as_percentage() {
                println!("    ✅ Buy tax calculated: {:.2}% ({} basis points)", tax_percentage, tax_basis_points);
                if tax_basis_points > 0 {
                    println!("    ⚠️  FLOKI has a {:.2}% buy tax!", tax_percentage);
                    println!("       This means you receive {:.2}% fewer tokens than expected", tax_percentage);
                } else {
                    println!("    ✅ No buy tax detected");
                }
            }
        }
        TaxCalculationResult::InvalidSimulation { reason } => {
            println!("    ❌ Could not calculate buy tax: {}", reason);
            
            // Manual analysis based on what we found
            let buyer_floki = buy_processed.get_address_token_balance_change(&buyer, &floki_address).unwrap_or(U256::ZERO);
            let pool_floki = buy_processed.get_address_token_balance_change(&pool_address, &floki_address).unwrap_or(U256::ZERO);
            
            println!("    📊 Manual analysis:");
            println!("      Buyer FLOKI change: {} ({})", buyer_floki, format_floki_amount(buyer_floki));
            println!("      Pool FLOKI change: {} ({})", pool_floki, format_floki_amount(pool_floki));
            
            if pool_floki > U256::ZERO {
                println!("      ⚠️  Pool GAINED FLOKI - this indicates a tax/reflection mechanism");
                println!("      💡 FLOKI likely redistributes tokens to holders during transactions");
            }
        }
    }
    
    metrics.total_transfer_events += buy_transfers;
    
    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(metrics);
    }
    
    // For FLOKI, we might need to try different sell amounts due to restrictions
    // Let's try different amounts to see what works
    
    // Test selling the full amount received from buy
    let test_amounts = vec![
        ("Full amount", metrics.tokens_received_wei),
    ];
    
    println!("\n  🔍 Debug - Testing different sell amounts:");
    println!("    Total received: {} ({} formatted)", metrics.tokens_received_wei, format_floki_amount(metrics.tokens_received_wei));
    
    // Debug: Verify FLOKI balance after buy by checking balanceOf
    println!("\n  🔍 Debug - Verifying FLOKI balance after buy:");
    println!("    Expected balance: {} raw", metrics.tokens_received_wei);
    
    // Call balanceOf to check actual balance
    let balance_check_data = {
        let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(buyer.as_slice());
        data
    };
    
    let balance_call = UnsignedTransaction {
        from: Some(buyer),
        to: Some(Address::from_str(FLOKI_ADDRESS)?),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(balance_check_data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let balance_result = chain.step_with_trace(balance_call).await?;
    if let Some(output) = balance_result.call_trace.output {
        if output.len() >= 32 {
            let actual_balance = U256::from_be_slice(&output[0..32]);
            println!("    Actual balance from balanceOf: {} raw", actual_balance);
            println!("    Formatted: {} FLOKI", format_floki_amount(actual_balance));
            
            if actual_balance != metrics.tokens_received_wei {
                println!("    ⚠️ WARNING: Balance mismatch!");
                println!("       Expected: {}", metrics.tokens_received_wei);
                println!("       Actual:   {}", actual_balance);
            } else {
                println!("    ✅ Balance matches expected!");
            }
        }
    }
    
    // Step 2: Approve router to spend FLOKI (do this once before testing all amounts)
    println!("\n[Step 2] Approving router to spend FLOKI...");
    let floki_addr = Address::from_str(FLOKI_ADDRESS)?;
    let approve_tx = create_approve_transaction(buyer, floki_addr, router, U256::MAX);
    
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
    log_balance_changes_for_floki(&approve_processed, buyer);
    
    if !approve_result.success {
        println!("  Revert reason: {:?}", approve_result.revert_reason);
        return Ok(metrics);
    }
    
    // Debug: Check allowance after approval
    println!("\n  🔍 Debug - Checking allowance after approval:");
    let allowance_check_data = {
        let mut data = vec![0xdd, 0x62, 0xed, 0x3e]; // allowance selector
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(buyer.as_slice());
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(router.as_slice());
        data
    };
    
    let allowance_call = UnsignedTransaction {
        from: Some(buyer),
        to: Some(floki_addr),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(allowance_check_data)),
        gas: Some(50_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let allowance_result = chain.step_with_trace(allowance_call).await?;
    if let Some(output) = allowance_result.call_trace.output {
        if output.len() >= 32 {
            let allowance = U256::from_be_slice(&output[0..32]);
            println!("    Allowance for router: {} raw", allowance);
            if allowance >= metrics.tokens_received_wei {
                println!("    ✅ Allowance is sufficient for sell!");
            } else {
                println!("    ❌ Allowance is NOT sufficient!");
                println!("       Need: {}", metrics.tokens_received_wei);
                println!("       Have: {}", allowance);
            }
        }
    }

    // Targeted check: simulate transferFrom(buyer -> pool) with controlled caller
    println!("\n  🎯 Targeted Check - transferFrom(buyer → pool) in isolation:");
    let pool_addr = Address::from_str(FLOKI_WETH_POOL)?;
    // Use an EOA as caller to avoid simulator restriction on contract senders
    let test_caller = Address::from_str("0x1000000000000000000000000000000000000001")?;
    let transferfrom_selector = [0x23, 0xb8, 0x72, 0xdd]; // transferFrom(address,address,uint256)

    let try_transfer_from_with_nonce = |amount: U256, nonce: Option<u64>| -> UnsignedTransaction {
        let mut data = Vec::with_capacity(4 + 3 * 32);
        data.extend_from_slice(&transferfrom_selector);
        // from (buyer)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(buyer.as_slice());
        // to (pool)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(pool_addr.as_slice());
        // amount
        data.extend_from_slice(&amount.to_be_bytes::<32>());

        UnsignedTransaction {
            from: Some(test_caller), // approved EOA caller simulating router role
            to: Some(floki_addr),
            value: Some(U256::ZERO),
            data: Some(Bytes::from(data)),
            gas: Some(200_000),
            gas_price: Some(20_000_000_000),
            nonce,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        }
    };

    let tf_small = metrics.tokens_received_wei / U256::from(100); // 1%
    let tf_full = metrics.tokens_received_wei; // 100%

    // Approve the test caller to spend on behalf of buyer for transferFrom
    println!("    Preparing approval for test caller (EOA) to perform transferFrom...");
    let approve_tester_tx = create_approve_transaction(buyer, floki_addr, test_caller, U256::MAX);
    let approve_tester_res = chain.step_with_trace(approve_tester_tx).await?;
    println!("    Approve test caller: success={}, gas_used={}", approve_tester_res.success, approve_tester_res.gas_used);
    if !approve_tester_res.success {
        println!("    Cannot proceed with targeted transferFrom test (approval failed): {:?}", approve_tester_res.revert_reason);
    }

    // Ensure test_caller has ETH for gas (fund from buyer)
    let fund_amount = U256::from(10_000_000_000_000_000u128); // 0.01 ETH
    println!("    Funding test caller with {} wei for gas", fund_amount);
    let fund_tx = UnsignedTransaction {
        from: Some(buyer),
        to: Some(test_caller),
        value: Some(fund_amount),
        data: Some(Bytes::from(vec![])),
        gas: Some(21_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    let fund_res = chain.step_with_trace(fund_tx).await?;
    println!("    Fund tx: success={}, gas_used={}", fund_res.success, fund_res.gas_used);

    println!(
        "    Attempting transferFrom(buyer → pool) SMALL: {} raw ({} FLOKI)",
        tf_small,
        format_floki_amount(tf_small)
    );
    let tf_small_tx = try_transfer_from_with_nonce(tf_small, Some(0));
    let tf_small_res = chain.step_with_trace(tf_small_tx).await?;
    println!("    Result: success={}, gas_used={}", tf_small_res.success, tf_small_res.gas_used);
    println!("    Revert reason: {:?}", tf_small_res.revert_reason);
    println!("    Trace error: {:?}", tf_small_res.call_trace.error);

    println!(
        "    Attempting transferFrom(buyer → pool) FULL: {} raw ({} FLOKI)",
        tf_full,
        format_floki_amount(tf_full)
    );
    let tf_full_tx = try_transfer_from_with_nonce(tf_full, Some(1));
    let tf_full_res = chain.step_with_trace(tf_full_tx).await?;
    println!("    Result: success={}, gas_used={}", tf_full_res.success, tf_full_res.gas_used);
    println!("    Revert reason: {:?}", tf_full_res.revert_reason);
    println!("    Trace error: {:?}", tf_full_res.call_trace.error);
    
    // Debug: Test a direct transfer to see if there's a tax
    println!("\n  🔍 Debug - Testing direct FLOKI transfer to check for tax:");
    let test_transfer_amount = metrics.tokens_received_wei / U256::from(100); // Transfer 1% to test
    println!("    Attempting to transfer: {} raw ({} FLOKI)", 
             test_transfer_amount, format_floki_amount(test_transfer_amount));
    
    let transfer_data = {
        let mut data = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer selector
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(router.as_slice()); // Transfer to router as test
        data.extend_from_slice(&test_transfer_amount.to_be_bytes::<32>());
        data
    };
    
    let transfer_call = UnsignedTransaction {
        from: Some(buyer),
        to: Some(floki_addr),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(transfer_data)),
        gas: Some(100_000),
        gas_price: None,
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };
    
    let transfer_result = chain.step_with_trace(transfer_call).await?;
    if transfer_result.success {
        println!("    ✅ Direct transfer succeeded!");
        
        // Check router's balance to see if full amount arrived
        let router_balance_data = {
            let mut data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
            data.extend_from_slice(&[0u8; 12]);
            data.extend_from_slice(router.as_slice());
            data
        };
        
        let router_balance_call = UnsignedTransaction {
            from: Some(buyer),
            to: Some(floki_addr),
            value: Some(U256::ZERO),
            data: Some(Bytes::from(router_balance_data)),
            gas: Some(50_000),
            gas_price: None,
            nonce: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        };
        
        let router_balance_result = chain.step_with_trace(router_balance_call).await?;
        if let Some(output) = router_balance_result.call_trace.output {
            if output.len() >= 32 {
                let router_balance = U256::from_be_slice(&output[0..32]);
                println!("    Router received: {} raw", router_balance);
                
                if router_balance < test_transfer_amount {
                    let tax_amount = test_transfer_amount - router_balance;
                    let tax_percent = (tax_amount * U256::from(100)) / test_transfer_amount;
                    println!("    ⚠️ TRANSFER TAX DETECTED!");
                    println!("       Sent: {}", test_transfer_amount);
                    println!("       Received: {}", router_balance);
                    println!("       Tax: {} raw (~{}%)", tax_amount, tax_percent);
                } else {
                    println!("    ✅ No transfer tax detected");
                }
            }
        }
    } else {
        println!("    ❌ Direct transfer failed!");
        println!("       This indicates transfer restrictions");
    }
    
    // Step 3: Attempt to sell 100% of FLOKI we received
    println!("\n[Step 3] Selling 100% of FLOKI received from buy...");
    
    // Sell all of the amount we received from the buy
    let floki_amount_to_sell = metrics.tokens_received_wei;
    
    println!("  🔄 Attempting to sell 100% of received amount");
    println!("    Total received: {} raw ({} formatted)", 
             metrics.tokens_received_wei, format_floki_amount(metrics.tokens_received_wei));
    println!("    Amount to sell: {} raw ({} formatted)", 
             floki_amount_to_sell, format_floki_amount(floki_amount_to_sell));
    
    metrics.tokens_attempted_to_sell = floki_amount_to_sell;
    
    println!("    Will pass to create_sell_floki_transaction: {} raw ({} FLOKI)", 
             floki_amount_to_sell, format_floki_amount(floki_amount_to_sell));
    let sell_tx = create_sell_floki_transaction(buyer, floki_amount_to_sell);
    
    // Debug: Print the sell transaction calldata
    println!("\n  🔍 Debug - Sell transaction details:");
    if let Some(data) = &sell_tx.data {
        println!("    Calldata: 0x{}", hex::encode(data.as_ref()));
        println!("    Calldata length: {} bytes", data.len());
        
        // Decode the parameters from calldata
        if data.len() >= 4 {
            let selector = &data[0..4];
            println!("    Function selector: 0x{}", hex::encode(selector));
            
            if data.len() >= 36 {
                let amount_in_bytes = &data[4..36];
                let amount_in = U256::from_be_slice(amount_in_bytes);
                println!("    Amount in (from calldata): {} raw ({} FLOKI)", 
                         amount_in, format_floki_amount(amount_in));
            }
        }
    }
    
    let sell_result = chain.step_with_trace(sell_tx.clone()).await?;
    
    // Generate ProcessedTransaction for sell (even if it fails)
    let sell_processed = tx_processor.process_transaction_from_simulation_result(
        &sell_tx,
        &sell_result,
        block,
        2, // tx_index
    ).await?;
    
    metrics.sell_succeeded = sell_result.success;
    println!("    Status: {}", if sell_result.success { "✅ Success" } else { "❌ Failed" });
    println!("    Gas used: {}", sell_result.gas_used);
    metrics.sell_gas = sell_result.gas_used;
    metrics.total_gas += sell_result.gas_used;
    
    if sell_result.success {
        // Extract exact amounts from balance changes (U256 precision)
        let (floki_sold, eth_received) = extract_sell_amounts_from_balance_changes(&sell_processed, buyer);
        metrics.tokens_sold_wei = floki_sold.abs_diff(U256::ZERO); // Make positive for display
        metrics.eth_received_wei = eth_received;
        
        println!("\n    💼 Balance Changes:");
        println!("      FLOKI: -{} (sold)", format_floki_amount(metrics.tokens_sold_wei));
        println!("      ETH: +{} (received)", format_eth_amount(metrics.eth_received_wei));
        
        metrics.total_transfer_events += sell_processed.erc20_transfers.len();
    } else {
        println!("    Revert reason: {:?}", sell_result.revert_reason);
        
        // Track failure for reporting
        metrics.sell_failure_reason = sell_result.revert_reason.clone()
            .or_else(|| Some("Transaction reverted".to_string()));
        
        println!("\n  ❌ Sell failed!");
        println!("  Transaction reverted at block {}", block);
        
        println!("\n  💡 Possible reasons for failure:");
        println!("    • Insufficient liquidity for large amount");
        println!("    • Price impact too high");
        println!("    • Token tax or fees");
        println!("    • Slippage protection triggered");
    }
    
    // Final state summary
    let state = chain.current_state();
    println!("\n📊 Final Chain State:");
    println!("  • Transactions executed: {}", state.transaction_count);
    println!("  • Total gas used: {}", state.total_gas_used);
    println!("  • State preserved throughout: ✅");
    
    Ok(metrics)
}

/// Extract FLOKI received and ETH spent from buy transaction balance changes
/// Returns (floki_wei, eth_wei) with full U256 precision
fn extract_buy_amounts_from_balance_changes(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
) -> (U256, U256) {
    // Get ETH spent from currency_net (will be negative for buy)
    let eth_wei = processed_tx.get_address_currency_balance_change(&buyer, "ETH")
        .unwrap_or(U256::ZERO);
    
    // Get FLOKI received from token_net using token address
    let floki_address = Address::from_str(FLOKI_ADDRESS).unwrap();
    let floki_wei = processed_tx.get_address_token_balance_change(&buyer, &floki_address)
        .unwrap_or(U256::ZERO);
    
    (floki_wei, eth_wei)
}

/// Extract FLOKI sold and ETH received from sell transaction balance changes
/// Returns (floki_wei, eth_wei) with full U256 precision
fn extract_sell_amounts_from_balance_changes(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
) -> (U256, U256) {
    // Get ETH received from currency_net (will be positive for sell)
    let eth_wei = processed_tx.get_address_currency_balance_change(&buyer, "ETH")
        .unwrap_or(U256::ZERO);
    
    // Get FLOKI sold from token_net using token address (will be negative for sell)
    let floki_address = Address::from_str(FLOKI_ADDRESS).unwrap();
    let floki_wei = processed_tx.get_address_token_balance_change(&buyer, &floki_address)
        .unwrap_or(U256::ZERO);
    
    (floki_wei, eth_wei)
}

/// Log all balance changes for FLOKI workflow with full precision
fn log_balance_changes_for_floki(
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
    
    // Log FLOKI changes from token_net
    let floki_address = Address::from_str(FLOKI_ADDRESS).unwrap();
    if let Some(floki_amount) = processed_tx.get_address_token_balance_change(&buyer, &floki_address) {
        if floki_amount != U256::ZERO {
            has_changes = true;
            println!("    FLOKI: {}", format_floki_amount_with_sign(floki_amount));
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

/// Format FLOKI amount from smallest units to human-readable with proper commas
/// FLOKI has 9 decimals!
fn format_floki_amount(wei: U256) -> String {
    if wei == U256::ZERO {
        return "0 FLOKI".to_string();
    }
    
    // FLOKI has 9 decimals
    let divisor = U256::from(10u128).pow(U256::from(FLOKI_DECIMALS));
    let whole = wei / divisor;
    let remainder = wei % divisor;
    
    // Format whole part with commas
    let whole_str = format_with_commas(whole);
    
    // For FLOKI, we typically don't need decimal places for large amounts
    if remainder == U256::ZERO {
        format!("{} FLOKI", whole_str)
    } else {
        // Show 2 decimal places for fractional FLOKI
        let decimal_part = (remainder * U256::from(100)) / divisor;
        format!("{}.{:02} FLOKI", whole_str, decimal_part)
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
fn format_floki_amount_with_sign(wei: U256) -> String {
    // Note: U256 can't be negative, so negative values are represented as very large numbers
    // For display purposes, we'll assume positive values
    format!("+{}", format_floki_amount(wei))
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

/// Create a transaction to buy FLOKI with ETH using Uniswap V2
fn create_buy_floki_transaction(buyer: Address, eth_amount: U256) -> UnsignedTransaction {
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
    
    // FLOKI address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_ADDRESS[2..]).unwrap());
    
    UnsignedTransaction {
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
fn create_approve_transaction(from: Address, token: Address, spender: Address, amount: U256) -> UnsignedTransaction {
    // approve(address spender, uint256 amount)
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // approve selector
    
    // spender address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
    
    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    
    UnsignedTransaction {
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

/// Create a transaction to sell FLOKI for ETH using Uniswap V2
fn create_sell_floki_transaction(seller: Address, floki_amount: U256) -> UnsignedTransaction {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector
    
    // amountIn (amount of FLOKI to sell)
    data.extend_from_slice(&floki_amount.to_be_bytes::<32>());
    
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
    
    // FLOKI address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&FLOKI_ADDRESS[2..]).unwrap());
    
    // WETH address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&WETH_ADDRESS[2..]).unwrap());
    
    UnsignedTransaction {
        from: Some(seller),
        to: Some(Address::from_str(UNISWAP_V2_ROUTER).unwrap()),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        gas: Some(500_000), // Higher gas limit since FLOKI might have complex logic
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}
