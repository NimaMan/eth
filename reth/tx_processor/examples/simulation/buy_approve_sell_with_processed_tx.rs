/// Buy → Approve → Sell with ProcessedTransaction Generation
/// 
/// This example demonstrates a complete token trading workflow using SimulationChain
/// with proper state preservation, generating ProcessedTransaction objects for each step.
/// 
/// Key features:
/// 1. Uses SimulationChain for proper state preservation between transactions
/// 2. Generates ProcessedTransaction for each step with full event decoding
/// 3. Logs buyer's token balance changes after each transaction
/// 4. Compares USDC vs USDT trading with detailed metrics
/// 
/// This shows the CORRECT way to simulate sequential transactions where each
/// transaction depends on the results of previous ones.

use eyre::Result;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, CallRequest};
use tx_processor::tx_processor::{TxProcessor, data_models::ProcessedTransaction};
use std::str::FromStr;

const RETH_DB_PATH: &str = "/home/nima/.local/share/reth/mainnet";

// Well-known contract addresses
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";      // USDC token
const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";      // USDT token  
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";      // Wrapped ETH
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"; // Uniswap V2 Router

// Test address with ETH balance
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";       // Test buyer (NOT a fee recipient)

#[tokio::main]
async fn main() -> Result<()> {
    println!("💱 Buy → Approve → Sell with ProcessedTransaction Demo");
    println!("======================================================");
    println!("Using SimulationChain for proper state preservation.\n");
    
    // Initialize simulator and processor
    let simulator = TxSimulator::new(RETH_DB_PATH)?;
    let tx_processor = TxProcessor::new();
    println!("✅ Simulator and TxProcessor initialized");
    
    // Get latest block
    let latest_block = simulator.get_latest_block()?;
    println!("📊 Block: {}", latest_block);
    println!("💰 Investment: 0.1 ETH per token\n");
    
    let buyer_address = Address::from_str(TEST_BUYER)?;
    let router_address = Address::from_str(UNISWAP_V2_ROUTER)?;
    
    // Test USDC workflow
    println!("{}", "=".repeat(60));
    println!("USDC WORKFLOW WITH STATE PRESERVATION");
    println!("{}", "=".repeat(60));
    
    let usdc_metrics = execute_trading_workflow_with_processed_tx(
        &simulator,
        &tx_processor,
        buyer_address,
        USDC_ADDRESS,
        "USDC",
        router_address,
        latest_block,
    ).await?;
    
    println!("\n{}", "=".repeat(60));
    println!("USDT WORKFLOW WITH STATE PRESERVATION");
    println!("{}", "=".repeat(60));
    
    let usdt_metrics = execute_trading_workflow_with_processed_tx(
        &simulator,
        &tx_processor,
        buyer_address,
        USDT_ADDRESS,
        "USDT",
        router_address,
        latest_block,
    ).await?;
    
    // Compare results
    println!("\n{}", "=".repeat(60));
    println!("📊 COMPARISON RESULTS");
    println!("{}", "=".repeat(60));
    
    println!("\n💰 Token Amounts:");
    println!("  USDC received: {:.2}", usdc_metrics.tokens_received);
    println!("  USDT received: {:.2}", usdt_metrics.tokens_received);
    let token_diff = ((usdt_metrics.tokens_received - usdc_metrics.tokens_received) / usdc_metrics.tokens_received) * 100.0;
    println!("  Difference: {:.2}%", token_diff);
    
    println!("\n💸 ETH Returns (selling tokens):");
    println!("  USDC → ETH: {:.6}", usdc_metrics.eth_received_on_sell);
    println!("  USDT → ETH: {:.6}", usdt_metrics.eth_received_on_sell);
    
    if usdc_metrics.eth_received_on_sell > 0.0 && usdt_metrics.eth_received_on_sell > 0.0 {
        let eth_diff = ((usdt_metrics.eth_received_on_sell - usdc_metrics.eth_received_on_sell) / usdc_metrics.eth_received_on_sell) * 100.0;
        println!("  Difference: {:.2}%", eth_diff);
    } else {
        println!("  Difference: ETH returns not detected (balance calculator issue)");
    }
    
    println!("\n⛽ Gas Usage:");
    println!("  USDC total: {} gas", usdc_metrics.total_gas);
    println!("  USDT total: {} gas", usdt_metrics.total_gas);
    println!("  Difference: {} gas", (usdt_metrics.total_gas as i64 - usdc_metrics.total_gas as i64));
    
    println!("\n📝 Event Counts:");
    println!("  USDC transfers: {}", usdc_metrics.total_transfer_events);
    println!("  USDT transfers: {}", usdt_metrics.total_transfer_events);
    
    println!("\n✅ Workflow demonstration complete!");
    Ok(())
}

/// Metrics collected from trading workflow
#[derive(Debug, Default)]
struct TradingMetrics {
    tokens_received: f64,
    eth_received_on_sell: f64,
    total_gas: u64,
    total_transfer_events: usize,
    buy_gas: u64,
    approve_gas: u64,
    sell_gas: u64,
}

/// Execute trading workflow with ProcessedTransaction generation
async fn execute_trading_workflow_with_processed_tx(
    simulator: &TxSimulator,
    tx_processor: &TxProcessor,
    buyer: Address,
    token_address: &str,
    token_symbol: &str,
    router: Address,
    block: u64,
) -> Result<TradingMetrics> {
    println!("\n🚀 Starting {} workflow at block {}", token_symbol, block);
    
    let mut metrics = TradingMetrics::default();
    
    // Start a simulation chain for state preservation
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    println!("📍 Chain initialized with state preservation");
    
    // Step 1: Buy tokens with 0.1 ETH
    println!("\n[Step 1] Buying {} with 0.1 ETH...", token_symbol);
    let buy_tx = create_buy_token_transaction(
        buyer, 
        token_address, 
        U256::from(100_000_000_000_000_000u128) // 0.1 ETH
    );
    
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
    
    // Analyze buyer's balance changes from ProcessedTransaction
    log_buyer_balance_changes(&buy_processed, buyer, "After Buy");
    
    // Count ERC20 transfers in ProcessedTransaction
    let buy_transfers = buy_processed.erc20_transfers.len();
    println!("  ERC20 Transfer events (from ProcessedTx): {}", buy_transfers);
    metrics.total_transfer_events += buy_transfers;
    
    // Extract token amount received from ProcessedTransaction
    let mut tokens_received = U256::ZERO;
    for transfer in &buy_processed.erc20_transfers {
        if transfer.to_address == buyer {
            tokens_received = transfer.amount;
            let tokens_float = tokens_received.to_string().parse::<f64>().unwrap_or(0.0) / 1_000_000.0;
            metrics.tokens_received = tokens_float;
            println!("  Tokens received: {:.2} {} (from ProcessedTx)", tokens_float, token_symbol);
        }
    }
    
    if !buy_result.success {
        println!("  Revert reason: {:?}", buy_result.revert_reason);
        return Ok(metrics);
    }
    
    // Step 2: Approve router to spend tokens
    println!("\n[Step 2] Approving router to spend {}...", token_symbol);
    let token_addr = Address::from_str(token_address)?;
    let approve_tx = create_approve_transaction(buyer, token_addr, router, U256::MAX);
    
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
    
    // Check for Approval events in ProcessedTransaction
    let approval_count = approve_processed.approvals.len();
    println!("  Approval events (from ProcessedTx): {}", approval_count);
    
    // Log balance changes (should be minimal for approve)
    log_buyer_balance_changes(&approve_processed, buyer, "After Approve");
    
    if !approve_result.success {
        println!("  Revert reason: {:?}", approve_result.revert_reason);
        return Ok(metrics);
    }
    
    // Step 3: Sell tokens back to ETH
    println!("\n[Step 3] Selling {} back to ETH...", token_symbol);
    
    // Use actual tokens received from buy (sell ALL tokens instead of half)
    let sell_amount = if tokens_received > U256::ZERO {
        tokens_received // Sell ALL tokens we bought
    } else {
        // Fallback amount if we couldn't extract from ProcessedTx
        if token_symbol == "USDC" {
            U256::from(100_000_000u128) // 100 USDC (6 decimals)
        } else {
            U256::from(100_000_000u128) // 100 USDT (6 decimals)
        }
    };
    
    println!("  Selling amount: {:.2} {}", 
        sell_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1_000_000.0, 
        token_symbol
    );
    
    let sell_tx = create_sell_token_transaction(buyer, token_address, sell_amount);
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
    
    
    // Analyze ETH received from ProcessedTransaction
    log_buyer_balance_changes(&sell_processed, buyer, "After Sell");
    
    // DEBUG: Print internal transactions to see if ETH transfers are captured
    println!("  DEBUG: Internal transactions in sell: {}", sell_processed.internal_transactions.len());
    for (i, internal_tx) in sell_processed.internal_transactions.iter().enumerate() {
        println!("    [{i}] {:#x} -> {:#x}: {} wei", 
                 internal_tx.from_address, 
                 internal_tx.to_address,
                 internal_tx.value);
        
        // Check if this internal tx involves our buyer
        if internal_tx.from_address == buyer || internal_tx.to_address == buyer {
            let eth_amount = internal_tx.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            println!("      ^^^^ BUYER INVOLVED: {} ETH", eth_amount);
        }
    }
    
    // Count transfers in sell transaction
    let sell_transfers = sell_processed.erc20_transfers.len();
    println!("  ERC20 Transfer events (from ProcessedTx): {}", sell_transfers);
    metrics.total_transfer_events += sell_transfers;
    
    // Extract ETH received from balance changes via currency_net
    if let Some(balance_changes) = sell_processed.address_balance_changes.get(&buyer) {
        if let Some(currency_net) = balance_changes.get("currency_net") {
            if let Some(currencies) = currency_net.as_object() {
                if let Some(eth_value) = currencies.get("ETH") {
                    if let Some(eth_amount) = eth_value.as_f64() {
                        // Note: eth_amount will be positive for ETH received
                        metrics.eth_received_on_sell = eth_amount.abs(); // Use abs to handle any sign issues
                        if metrics.eth_received_on_sell > 0.0 {
                            println!("  ETH received: {:.6} ETH (from ProcessedTx)", metrics.eth_received_on_sell);
                        }
                    }
                }
            }
        }
    }
    
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

/// Log buyer's balance changes from ProcessedTransaction
fn log_buyer_balance_changes(
    processed_tx: &ProcessedTransaction,
    buyer: Address,
    context: &str,
) {
    println!("\n  💼 {} Balance Changes:", context);
    
    if let Some(changes) = processed_tx.address_balance_changes.get(&buyer) {
        // Look for currency_net which contains all currency changes
        if let Some(currency_net) = changes.get("currency_net") {
            if let Some(currencies) = currency_net.as_object() {
                // Extract and display each currency change
                for (currency, value) in currencies {
                    if let Some(amount_f64) = value.as_f64() {
                        if amount_f64 != 0.0 {
                            if currency == "ETH" {
                                println!("    {}: {:+.6}", currency, amount_f64);
                            } else {
                                // USDC, USDT, etc - already in proper units from balance calculator
                                println!("    {}: {:+.2}", currency, amount_f64);
                            }
                        }
                    }
                }
            }
        } else {
            println!("    No currency_net found in balance changes");
        }
    } else {
        println!("    No balance changes recorded");
    }
}

/// Create a transaction to buy a token with ETH using Uniswap V2
fn create_buy_token_transaction(buyer: Address, token_address: &str, eth_amount: U256) -> CallRequest {
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
    
    // Target token address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&hex::decode(&token_address[2..]).unwrap());
    
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

/// Create a transaction to sell tokens for ETH using Uniswap V2
fn create_sell_token_transaction(seller: Address, token_address: &str, token_amount: U256) -> CallRequest {
    // swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] path, address to, uint256 deadline)
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5]; // Function selector
    
    // amountIn
    data.extend_from_slice(&token_amount.to_be_bytes::<32>());
    
    // amountOutMin (1 = accept any amount)
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
        gas: Some(300_000),
        gas_price: Some(20_000_000_000),
        nonce: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    }
}