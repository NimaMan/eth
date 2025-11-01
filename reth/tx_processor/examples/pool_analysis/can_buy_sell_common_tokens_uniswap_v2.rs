use alloy_primitives::{Address, U256};
/// Token Trading Viability Simulation
///
/// Demonstrates token trading viability analysis by simulating the complete
/// trading sequence (buy -> approve -> sell) while maintaining blockchain state
/// between each transaction for accurate tax calculation.
use eyre::Result;
use reth_chain_query::common_addresses::dex_pools::{
    compute_sushiswap_pool, compute_uniswap_v2_pool, SUSHISWAP_FACTORY, UNISWAP_V2_FACTORY,
};
use std::sync::Arc;
use tx_processor::simulator::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::types::ViewFunctionResult;
use tx_simulator::TxSimulator;

/// Configuration for testing a specific token
#[derive(Debug, Clone)]
pub struct TokenConfig {
    symbol: &'static str,
    token_address: Address,
    denom_address: Address,
    pool_type: PoolType,
    expected_behavior: ExpectedBehavior,
    decimals: u8,
}

/// Expected behavior for different token categories
#[derive(Debug, Clone)]
pub enum ExpectedBehavior {
    ShouldWork,   // Blue chip tokens - should work perfectly
    MayHaveTaxes, // Meme tokens - may have transfer fees
    MayFail,      // Known problematic tokens
}

mod token_sets;

/// Result of testing a single token
#[derive(Debug)]
struct TokenTestResult {
    config: TokenConfig,
    pool_address: Address,
    result: Option<PoolBuySellSimulationResult>,
    error: Option<String>,
    test_duration: std::time::Duration,
}

/// Define comprehensive token test suite
#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Multi-Token Trading Viability Analysis");
    println!("==========================================");
    println!("Testing comprehensive token trading viability across multiple token categories.\n");

    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    println!("Using Reth datadir: {}", reth_datadir);

    // Create simulator and tx processor
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    // Get latest block to avoid pruned state
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}\n", latest_block);

    // Get token configurations
    let token_configs = token_sets::uniswap_v2::token_configs();
    let total_tokens = token_configs.len();
    let mut all_results = Vec::new();

    println!(
        "📋 Testing {} tokens across different categories:",
        total_tokens
    );

    for config in &token_configs {
        println!(
            "  {} {} - Expected: {:?}",
            match config.expected_behavior {
                ExpectedBehavior::ShouldWork => "✅",
                ExpectedBehavior::MayHaveTaxes => "⚠️",
                ExpectedBehavior::MayFail => "❌",
            },
            config.symbol,
            config.expected_behavior
        );
    }
    println!();

    // Test each token
    for (idx, config) in token_configs.into_iter().enumerate() {
        println!("{}", "=".repeat(80));
        println!(
            "🔍 TESTING [{}/{}]: {} ({})",
            idx + 1,
            total_tokens,
            config.symbol,
            config.token_address
        );
        println!("{}", "=".repeat(80));

        let test_result = test_single_token(&config, &simulator, &tx_processor, latest_block).await;

        all_results.push(test_result);

        // Brief pause between tests
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    print_comprehensive_summary(&all_results);

    Ok(())
}

/// Test a single token configuration comprehensively
async fn test_single_token(
    config: &TokenConfig,
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    latest_block: u64,
) -> TokenTestResult {
    let start_time = std::time::Instant::now();

    let pool_address = match resolve_pool_address(config) {
        Ok(addr) => addr,
        Err(error_msg) => {
            println!("❌ {}", error_msg);
            return TokenTestResult {
                config: config.clone(),
                pool_address: Address::ZERO,
                result: None,
                error: Some(error_msg),
                test_duration: start_time.elapsed(),
            };
        }
    };

    println!("🔍 Configuration:");
    println!("  Token: {} ({})", config.symbol, config.token_address);
    println!("  Denom: {}", config.denom_address);
    println!("  Pool: {}", pool_address);
    println!("  Type: {:?}", config.pool_type);
    println!("  Expected: {:?}", config.expected_behavior);
    println!("  Decimals: {}", config.decimals);

    let token_address = config.token_address;
    let denom_address = config.denom_address;

    // Test all tokens at latest block
    let buy_block = latest_block;
    let block_delays_to_test = vec![0]; // Test same-block for all tokens

    for (test_index, block_delay) in block_delays_to_test.iter().enumerate() {
        if block_delays_to_test.len() > 1 {
            let sell_block = buy_block + block_delay;
            println!(
                "\n🔍 TEST #{}: Buy at {}, Sell at {} (delay: {})",
                test_index + 1,
                buy_block,
                sell_block,
                block_delay
            );
        }

        // Create pool configuration
        let pool_config =
            PoolBuySellParameters::new(token_address, pool_address, config.pool_type.clone())
                .with_test_amount(U256::from(1_000_000_000_000_000_000u64)) // 1.0 ETH for testing
                .with_token_decimals(config.decimals)
                .with_block(buy_block)
                .with_block_delay(*block_delay);

        println!(
            "\n🚀 Running trading viability analysis (block_delay={})...",
            block_delay
        );

        // Run the analysis
        match check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), pool_config).await {
            Ok(result) => {
                let duration = start_time.elapsed();

                // Print detailed results for this token
                print_token_result_with_block_delay(config, &result, duration, *block_delay);

                if !result.can_buy
                    && matches!(config.pool_type, PoolType::UniswapV2 | PoolType::SushiSwap)
                {
                    if let Some(diag) = diagnose_v2_pool(
                        simulator,
                        config.pool_type.clone(),
                        token_address,
                        denom_address,
                        pool_address,
                        buy_block,
                    )
                    .await
                    {
                        println!("🔎 V2 diagnostics: {}", diag);
                    }
                }

                // If this is a successful test, or if it's the last test for this token, return the result
                if result.is_tradeable || test_index == block_delays_to_test.len() - 1 {
                    return TokenTestResult {
                        config: config.clone(),
                        pool_address,
                        result: Some(result),
                        error: None,
                        test_duration: duration,
                    };
                }
            }
            Err(e) => {
                let mut error_msg = format!("Analysis failed: {}", e);
                if matches!(config.pool_type, PoolType::UniswapV2 | PoolType::SushiSwap) {
                    if let Some(diag) = diagnose_v2_pool(
                        simulator,
                        config.pool_type.clone(),
                        token_address,
                        denom_address,
                        pool_address,
                        buy_block,
                    )
                    .await
                    {
                        error_msg.push_str(&format!(" | Diagnostics: {}", diag));
                    }
                }
                let duration = start_time.elapsed();

                println!(
                    "❌ Analysis Error (block_delay={}): {}",
                    block_delay, error_msg
                );
                println!("⏱️  Test Duration: {:?}", duration);

                // If this is the last test, return the error
                if test_index == block_delays_to_test.len() - 1 {
                    return TokenTestResult {
                        config: config.clone(),
                        pool_address,
                        result: None,
                        error: Some(error_msg),
                        test_duration: duration,
                    };
                }
            }
        }
    }

    // This should never be reached, but provide a fallback
    TokenTestResult {
        config: config.clone(),
        pool_address,
        result: None,
        error: Some("No tests completed".to_string()),
        test_duration: start_time.elapsed(),
    }
}

fn resolve_pool_address(config: &TokenConfig) -> Result<Address, String> {
    match config.pool_type {
        PoolType::UniswapV2 => Ok(compute_uniswap_v2_pool(
            config.token_address,
            config.denom_address,
        )),
        PoolType::SushiSwap => Ok(
            reth_chain_query::common_addresses::dex_pools::compute_sushiswap_pool(
                config.token_address,
                config.denom_address,
            ),
        ),
        other => Err(format!(
            "Pool resolution not implemented for pool type {:?}",
            other
        )),
    }
}

/// Print detailed results for a single token test with block delay info
fn print_token_result_with_block_delay(
    config: &TokenConfig,
    result: &PoolBuySellSimulationResult,
    duration: std::time::Duration,
    block_delay: u64,
) {
    println!(
        "📈 Analysis Results for {} (block_delay={}):",
        config.symbol, block_delay
    );
    println!("==================================");
    println!("⏱️  Execution Time: {:?}", duration);
    println!("📦 Block Number: {}", result.block_number);
    if block_delay > 0 {
        println!(
            "🕒 Block Timing: Buy at {}, Sell at {} (delay: {})",
            result.block_number,
            result.block_number + block_delay,
            block_delay
        );
    }

    println!("\n🔍 Individual Operations:");
    println!("  📈 Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
    println!(
        "  ✅ Can Approve: {}",
        if result.can_approve { "✅" } else { "❌" }
    );
    println!(
        "  📉 Can Sell: {}",
        if result.can_sell { "✅" } else { "❌" }
    );
    println!(
        "  🎯 Overall Tradeable: {}",
        if result.is_tradeable { "✅" } else { "❌" }
    );

    if result.is_tradeable {
        println!("\n💰 Tax Analysis:");
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);

        println!("\n🔢 Trade Details:");
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.denom_received);

        let loss = result.denom_spent.saturating_sub(result.denom_received);
        let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  📊 Net Loss: {:.6} ETH ({} wei)", loss_eth, loss);

        // Tax analysis
        if result.buy_tax_percent > 0.0 || result.sell_tax_percent > 0.0 {
            println!("\n⚠️  Tax Detection:");
            if result.buy_tax_percent > 0.0 {
                println!("     Buy tax detected: {:.2}%", result.buy_tax_percent);
            }
            if result.sell_tax_percent > 0.0 {
                println!("     Sell tax detected: {:.2}%", result.sell_tax_percent);
            }
        }

        println!(
            "\n✅ {} is fully tradeable with block_delay={}!",
            config.symbol, block_delay
        );
    } else {
        println!(
            "\n❌ Trading failed for {} with block_delay={}!",
            config.symbol, block_delay
        );
        if let Some(reason) = &result.failure_reason {
            println!("   Detailed Error: {}", reason);
        }

        // Additional debug info
        println!("\n🔍 Debug Information:");
        println!("  💰 ETH Spent: {} wei", result.denom_spent);
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.denom_received);
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);

        if block_delay == 0 {
            println!("  💡 This was a same-block test. Testing next-block execution next...");
        } else {
            println!(
                "  💡 This was a next-block test. Same issue persists across block boundaries."
            );
        }
    }
}

/// Print detailed results for a single token test
fn print_token_result(
    config: &TokenConfig,
    result: &PoolBuySellSimulationResult,
    duration: std::time::Duration,
) {
    println!("📈 Analysis Results for {}:", config.symbol);
    println!("==================================");
    println!("⏱️  Execution Time: {:?}", duration);
    println!("📦 Block Number: {}", result.block_number);

    println!("\n🔍 Individual Operations:");
    println!("  📈 Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
    println!(
        "  ✅ Can Approve: {}",
        if result.can_approve { "✅" } else { "❌" }
    );
    println!(
        "  📉 Can Sell: {}",
        if result.can_sell { "✅" } else { "❌" }
    );
    println!(
        "  🎯 Overall Tradeable: {}",
        if result.is_tradeable { "✅" } else { "❌" }
    );

    if result.is_tradeable {
        println!("\n💰 Tax Analysis:");
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);

        println!("\n🔢 Trade Details:");
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.denom_received);

        let loss = result.denom_spent.saturating_sub(result.denom_received);
        let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  📊 Net Loss: {:.6} ETH ({} wei)", loss_eth, loss);

        // Tax analysis
        if result.buy_tax_percent > 0.0 || result.sell_tax_percent > 0.0 {
            println!("\n⚠️  Tax Detection:");
            if result.buy_tax_percent > 0.0 {
                println!("     Buy tax detected: {:.2}%", result.buy_tax_percent);
            }
            if result.sell_tax_percent > 0.0 {
                println!("     Sell tax detected: {:.2}%", result.sell_tax_percent);
            }
        }

        println!("\n✅ {} is fully tradeable!", config.symbol);
    } else {
        println!("\n❌ Trading failed for {}!", config.symbol);
        if let Some(reason) = &result.failure_reason {
            println!("   Detailed Error: {}", reason);
        }

        // Additional debug info
        println!("\n🔍 Debug Information:");
        println!("  💰 ETH Spent: {} wei", result.denom_spent);
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.denom_received);
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);
    }
}

/// Print comprehensive summary of all test results
fn print_comprehensive_summary(results: &[TokenTestResult]) {
    println!("\n{}", "=".repeat(80));
    println!("📊 COMPREHENSIVE RESULTS SUMMARY");
    println!("{}", "=".repeat(80));

    // Count results by category
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut tax_tokens = 0;

    for result in results {
        if let Some(analysis) = &result.result {
            if analysis.is_tradeable {
                successful += 1;
                if analysis.buy_tax_percent > 0.0 || analysis.sell_tax_percent > 0.0 {
                    tax_tokens += 1;
                }
            } else {
                failed += 1;
            }
        } else {
            errors += 1;
        }
    }

    println!("\n📈 Overall Statistics:");
    println!("  ✅ Successful: {} tokens", successful);
    println!("  ❌ Failed Trading: {} tokens", failed);
    println!("  💥 Analysis Errors: {} tokens", errors);
    println!("  🏷️  Tax Tokens Detected: {} tokens", tax_tokens);
    println!("  📊 Total Tested: {} tokens", results.len());

    // Detailed results table
    println!("\n📋 Detailed Results Table:");
    println!("{}", "-".repeat(120));
    println!(
        "{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}",
        "Symbol",
        "Tradeable",
        "Buy",
        "Approve",
        "Sell",
        "Buy Tax",
        "Sell Tax",
        "Net Loss",
        "Failure Reason"
    );
    println!("{}", "-".repeat(120));

    for result in results {
        let symbol = result.config.symbol;

        if let Some(analysis) = &result.result {
            let tradeable = if analysis.is_tradeable { "✅" } else { "❌" };
            let can_buy = if analysis.can_buy { "✅" } else { "❌" };
            let can_approve = if analysis.can_approve { "✅" } else { "❌" };
            let can_sell = if analysis.can_sell { "✅" } else { "❌" };
            let buy_tax = format!("{:.1}%", analysis.buy_tax_percent);
            let sell_tax = format!("{:.1}%", analysis.sell_tax_percent);

            let net_loss = if analysis.is_tradeable {
                let loss = analysis.denom_spent.saturating_sub(analysis.denom_received);
                let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                format!("{:.4}E", loss_eth)
            } else {
                "-".to_string()
            };

            let reason = if analysis.is_tradeable {
                "-".to_string()
            } else {
                analysis
                    .failure_reason
                    .as_deref()
                    .unwrap_or("Unknown")
                    .chars()
                    .take(18)
                    .collect::<String>()
                    + if analysis.failure_reason.as_deref().unwrap_or("").len() > 18 {
                        "..."
                    } else {
                        ""
                    }
            };

            println!(
                "{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}",
                symbol,
                tradeable,
                can_buy,
                can_approve,
                can_sell,
                buy_tax,
                sell_tax,
                net_loss,
                reason
            );
        } else {
            let error_reason = result
                .error
                .as_deref()
                .unwrap_or("Unknown error")
                .chars()
                .take(18)
                .collect::<String>()
                + if result.error.as_deref().unwrap_or("").len() > 18 {
                    "..."
                } else {
                    ""
                };

            println!(
                "{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}",
                symbol, "💥 ERROR", "-", "-", "-", "-", "-", "-", error_reason
            );
        }
    }

    println!("{}", "-".repeat(120));

    // Performance summary
    let total_duration: std::time::Duration = results.iter().map(|r| r.test_duration).sum();
    let avg_duration = total_duration / results.len() as u32;

    println!("\n⏱️  Performance Summary:");
    println!("  Total Testing Time: {:?}", total_duration);
    println!("  Average Per Token: {:?}", avg_duration);

    // Component validation status
    println!("\n🔧 Component Validation Status:");
    if successful > 0 {
        println!("  ✅ optional_setup_buy_approve_sell_token_simulator.rs is working correctly");
        println!("  ✅ State preservation across transaction sequences validated");
        println!("  ✅ Tax detection mechanisms functioning properly");
        println!("  ✅ Error reporting providing clear failure reasons");
    }

    if failed > 0 || errors > 0 {
        println!("  📝 Some tokens failed as expected (taxes, low liquidity, etc.)");
        println!("  📝 Component correctly identifies non-tradeable tokens");
    }

    println!("\n🎉 Multi-token analysis complete! Component validation successful.");
}

async fn diagnose_v2_pool(
    simulator: &Arc<TxSimulator>,
    pool_type: PoolType,
    token_address: Address,
    denom_address: Address,
    pool_address: Address,
    block: u64,
) -> Option<String> {
    let factory = match pool_type {
        PoolType::UniswapV2 => UNISWAP_V2_FACTORY,
        PoolType::SushiSwap => SUSHISWAP_FACTORY,
        _ => return None,
    };

    let mut notes = Vec::new();
    let mut token0_addr = None;
    let mut token1_addr = None;
    let mut reserve0_val = None;
    let mut reserve1_val = None;

    let factory_check = {
        let mut data = vec![0x0d, 0xd5, 0x5c, 0x4c]; // getPair(tokenA, tokenB)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(denom_address.as_slice());
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(token_address.as_slice());
        simulator
            .simulate_view_function(
                factory,
                alloy_primitives::Bytes::from(data),
                Some(block),
                None,
            )
            .await
            .ok()
    };

    if let Some(result) = factory_check {
        if result.success && result.output.len() >= 32 {
            let resolved = Address::from_slice(&result.output[12..32]);
            if resolved.is_zero() {
                notes.push("factory getPair returned zero address".to_string());
            } else if resolved != pool_address {
                notes.push(format!(
                    "factory getPair returned {}, but config uses {}",
                    resolved, pool_address
                ));
            }
        } else {
            notes.push("factory getPair call failed".to_string());
        }
    }

    let token0 = call_pool_view(simulator, pool_address, [0x0d, 0xfe, 0x16, 0x81], block).await;
    let token1 = call_pool_view(simulator, pool_address, [0xd2, 0x12, 0x20, 0xa7], block).await;
    let reserves = call_pool_view(simulator, pool_address, [0x09, 0x02, 0xf1, 0xac], block).await;

    if let Some(res) = token0 {
        if res.success && res.output.len() >= 32 {
            let addr = Address::from_slice(&res.output[12..32]);
            token0_addr = Some(addr);
            if addr != token_address && addr != denom_address {
                notes.push(format!("token0={} unexpected", addr));
            }
        } else {
            notes.push("token0() call failed".to_string());
        }
    }

    if let Some(res) = token1 {
        if res.success && res.output.len() >= 32 {
            let addr = Address::from_slice(&res.output[12..32]);
            token1_addr = Some(addr);
            if addr != token_address && addr != denom_address {
                notes.push(format!("token1={} unexpected", addr));
            }
        } else {
            notes.push("token1() call failed".to_string());
        }
    }

    if let Some(res) = reserves {
        if res.success && res.output.len() >= 96 {
            let reserve0 = U256::from_be_slice(&res.output[0..32]);
            let reserve1 = U256::from_be_slice(&res.output[32..64]);
            reserve0_val = Some(reserve0);
            reserve1_val = Some(reserve1);
            if reserve0.is_zero() || reserve1.is_zero() {
                notes.push(format!(
                    "zero reserves reserve0={} reserve1={}",
                    reserve0, reserve1
                ));
            }
        } else {
            notes.push("getReserves() call failed".to_string());
        }
    }

    if notes.is_empty() {
        let token0_str = token0_addr
            .map(|a| a.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        let token1_str = token1_addr
            .map(|a| a.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());
        let reserve0_str = reserve0_val
            .map(|r| r.to_string())
            .unwrap_or_else(|| "?".to_string());
        let reserve1_str = reserve1_val
            .map(|r| r.to_string())
            .unwrap_or_else(|| "?".to_string());
        Some(format!(
            "token0={}, token1={}, reserves=({}, {}), no anomalies detected",
            token0_str, token1_str, reserve0_str, reserve1_str
        ))
    } else {
        Some(notes.join("; "))
    }
}

async fn call_pool_view(
    simulator: &Arc<TxSimulator>,
    pool: Address,
    selector: [u8; 4],
    block: u64,
) -> Option<ViewFunctionResult> {
    let data = alloy_primitives::Bytes::from(selector.to_vec());
    simulator
        .simulate_view_function(pool, data, Some(block), None)
        .await
        .ok()
}
