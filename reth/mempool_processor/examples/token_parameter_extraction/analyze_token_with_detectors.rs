use alloy_primitives::{Address, I256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{SolCall, SolValue};
use chrono::Local;
use eyre::Result;
use mempool_processor::config::MempoolProcessorConfig;
use mempool_processor::signal_detector::{HoneypotDetector, TradingStatusDetector};
/// Analyze Token with Signal Detectors
///
/// This example demonstrates the complete signal detection pipeline:
/// 1. Uses SequentialBuySellSimulator to get raw simulation results
/// 2. Passes state changes to tax calculator functions
/// 3. Runs various signal detectors on the raw data
/// 4. Logs comprehensive analysis to file
///
/// Usage: cargo run --example analyze_token_with_detectors -- <token_address> <pool_address> [block_number]
use mempool_processor::simulator::{BuySellSimulatorConfig, SequentialBuySellSimulator};
use mempool_processor::token_parameter_extraction::{calculate_buy_tax, calculate_sell_tax};
use reth_chain_query::to_checksum_address;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::str::FromStr;

// ERC20 function signatures
alloy_sol_types::sol! {
    function decimals() external view returns (uint8);
    function symbol() external view returns (string);
    function name() external view returns (string);
    function totalSupply() external view returns (uint256);
}

async fn get_token_info(
    token_address: Address,
    block: Option<u64>,
) -> Result<(u8, String, String, U256)> {
    let provider = ProviderBuilder::new().connect_http("http://localhost:8545".parse()?);

    // Get decimals
    let decimals_data = decimalsCall {}.abi_encode();
    let decimals_tx = TransactionRequest::default()
        .to(token_address)
        .input(decimals_data.into());

    let decimals_call = if let Some(b) = block {
        provider.call(decimals_tx).block(b.into())
    } else {
        provider.call(decimals_tx)
    };

    let decimals_result = decimals_call.await?;
    let decimals_bytes = decimals_result.to_vec();
    let decimals = if decimals_bytes.len() >= 32 {
        decimals_bytes[31]
    } else {
        18
    };

    // Get symbol
    let symbol_data = symbolCall {}.abi_encode();
    let symbol_tx = TransactionRequest::default()
        .to(token_address)
        .input(symbol_data.into());

    let symbol_call = if let Some(b) = block {
        provider.call(symbol_tx).block(b.into())
    } else {
        provider.call(symbol_tx)
    };

    let symbol_result = symbol_call.await?;
    let symbol_bytes = symbol_result.to_vec();
    let symbol = String::abi_decode(&symbol_bytes).unwrap_or_else(|_| "UNKNOWN".to_string());

    // Get name
    let name_data = nameCall {}.abi_encode();
    let name_tx = TransactionRequest::default()
        .to(token_address)
        .input(name_data.into());

    let name_call = if let Some(b) = block {
        provider.call(name_tx).block(b.into())
    } else {
        provider.call(name_tx)
    };

    let name_result = name_call.await?;
    let name_bytes = name_result.to_vec();
    let name = String::abi_decode(&name_bytes).unwrap_or_else(|_| "Unknown Token".to_string());

    // Get total supply
    let supply_data = totalSupplyCall {}.abi_encode();
    let supply_tx = TransactionRequest::default()
        .to(token_address)
        .input(supply_data.into());

    let supply_call = if let Some(b) = block {
        provider.call(supply_tx).block(b.into())
    } else {
        provider.call(supply_tx)
    };

    let supply_result = supply_call.await?;
    let supply_bytes = supply_result.to_vec();
    let total_supply = U256::abi_decode(&supply_bytes)?;

    Ok((decimals, symbol, name, total_supply))
}

fn format_token_amount(amount: U256, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }

    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;

    if fraction == U256::ZERO {
        format!("{}", whole)
    } else {
        let fraction_str = format!("{:0>width$}", fraction, width = decimals as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole)
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = if let Ok(config_path) = std::env::var("MEMPOOL_CONFIG_PATH") {
        MempoolProcessorConfig::from_file(&config_path)
            .map_err(|e| eyre::eyre!("Failed to load config: {}", e))?
    } else {
        MempoolProcessorConfig::from_env()
    };

    let tax_config = &config.tax_detection;

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <token_address> <pool_address> [block_number]",
            args[0]
        );
        println!("Example: {} 0xBCb2479ae9F271BB1561b4823dCaAc8B02860B1E 0xa32d14c0d48ed4835179f33bc00d1bd7acea4aff 23005264", args[0]);
        return Ok(());
    }

    let token_address = Address::from_str(&args[1])?;
    let pool_address = Address::from_str(&args[2])?;
    let block_number: Option<u64> = if args.len() > 3 {
        Some(args[3].parse()?)
    } else {
        None
    };

    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev/token_parameter_extraction";
    create_dir_all(log_dir)?;

    // Create log file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let token_short = to_checksum_address(&token_address)
        .chars()
        .skip(2)
        .take(8)
        .collect::<String>();
    let log_file_path = format!(
        "{}/token_analysis_{}_{}.log",
        log_dir, token_short, timestamp
    );
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)?;

    println!("🔍 Analyzing token with signal detectors");
    println!("📝 Logging to: {}", log_file_path);

    // Write header
    writeln!(log_file, "Token Analysis with Signal Detectors")?;
    writeln!(log_file, "Generated: {}", Local::now())?;
    writeln!(log_file, "=")?;
    writeln!(log_file)?;

    // Get token info
    println!("\n📊 Fetching token information...");
    let (decimals, symbol, name, total_supply) =
        get_token_info(token_address, block_number).await?;

    writeln!(log_file, "Configuration")?;
    writeln!(log_file, "-------------")?;
    writeln!(
        log_file,
        "Max Acceptable Buy Tax: {}%",
        tax_config.max_acceptable_buy_tax
    )?;
    writeln!(
        log_file,
        "Max Acceptable Sell Tax: {}%",
        tax_config.max_acceptable_sell_tax
    )?;
    writeln!(
        log_file,
        "Honeypot Sell Threshold: {}%",
        tax_config.honeypot_sell_threshold
    )?;
    writeln!(log_file)?;

    writeln!(log_file, "Token Information")?;
    writeln!(log_file, "-----------------")?;
    writeln!(log_file, "Address: {}", token_address)?;
    writeln!(log_file, "Name: {}", name)?;
    writeln!(log_file, "Symbol: {}", symbol)?;
    writeln!(log_file, "Decimals: {}", decimals)?;
    writeln!(
        log_file,
        "Total Supply: {} ({})",
        total_supply,
        format_token_amount(total_supply, decimals)
    )?;
    writeln!(log_file, "Pool Address: {}", pool_address)?;
    if let Some(block) = block_number {
        writeln!(log_file, "Block Number: {}", block)?;
    } else {
        writeln!(log_file, "Block Number: Latest")?;
    }
    writeln!(log_file)?;

    // Initialize simulator
    println!("🚀 Initializing simulator...");
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let config = BuySellSimulatorConfig::default();
    let simulator = SequentialBuySellSimulator::with_config(reth_datadir, config.clone())?;

    writeln!(log_file, "Simulator Configuration")?;
    writeln!(log_file, "----------------------")?;
    writeln!(log_file, "Buyer Address: {}", config.buyer_address)?;
    writeln!(
        log_file,
        "Test Buy Amount: {} ETH",
        format_token_amount(config.test_buy_amount, 18)
    )?;
    writeln!(log_file, "Router Address: {}", config.router_address)?;
    writeln!(log_file, "WETH Address: {}", config.weth_address)?;
    writeln!(log_file)?;

    // Run simulation
    println!("⚡ Running buy/sell sequence simulation...");
    match simulator
        .simulate_sequence(token_address, pool_address, block_number)
        .await
    {
        Ok(result) => {
            writeln!(log_file, "Simulation Results")?;
            writeln!(log_file, "-----------------")?;
            writeln!(
                log_file,
                "Simulation Time: {:.2}ms",
                result.simulation_time_ms
            )?;
            writeln!(log_file, "Block Used: {}", result.block_number)?;
            writeln!(log_file)?;

            // Buy transaction analysis
            writeln!(log_file, "Buy Transaction")?;
            writeln!(log_file, "  Success: {}", result.buy_result.success)?;
            writeln!(log_file, "  Gas Used: {}", result.buy_result.gas_used)?;
            if let Some(reason) = &result.buy_result.revert_reason {
                writeln!(log_file, "  Revert Reason: {}", reason)?;
            }

            // Calculate buy tax if successful
            let buy_tax = if result.buy_result.success {
                calculate_buy_tax(
                    &result.buy_result.state_changes,
                    &pool_address,
                    &config.buyer_address,
                    &token_address,
                )
            } else {
                None
            };

            if let Some(tax) = buy_tax {
                writeln!(log_file, "  Buy Tax: {:.2}%", tax)?;
            } else {
                writeln!(log_file, "  Buy Tax: N/A")?;
            }

            // Extract tokens received
            let tokens_received = if result.buy_result.success {
                let token_addr_str = alloy_address_to_checksum(token_address);
                result
                    .buy_result
                    .state_changes
                    .get(&config.buyer_address)
                    .and_then(|changes| changes.token_net.get(&token_addr_str).copied())
                    .map(|i256_val| {
                        if i256_val >= I256::ZERO {
                            i256_val.unsigned_abs()
                        } else {
                            U256::ZERO
                        }
                    })
                    .unwrap_or(U256::ZERO)
            } else {
                U256::ZERO
            };

            writeln!(
                log_file,
                "  Tokens Received: {} ({})",
                tokens_received,
                format_token_amount(tokens_received, decimals)
            )?;
            writeln!(log_file)?;

            // Sell transaction analysis
            writeln!(log_file, "Sell Transaction")?;
            writeln!(log_file, "  Success: {}", result.sell_result.success)?;
            writeln!(log_file, "  Gas Used: {}", result.sell_result.gas_used)?;
            if let Some(reason) = &result.sell_result.revert_reason {
                writeln!(log_file, "  Revert Reason: {}", reason)?;
            }

            // Calculate sell tax if successful
            let sell_tax = if result.sell_result.success {
                calculate_sell_tax(
                    &result.sell_result.state_changes,
                    &pool_address,
                    &config.buyer_address,
                )
            } else {
                None
            };

            if let Some(tax) = sell_tax {
                writeln!(log_file, "  Sell Tax: {:.2}%", tax)?;
            } else {
                writeln!(log_file, "  Sell Tax: N/A")?;
            }

            // Extract ETH received
            let eth_received = if result.sell_result.success {
                result
                    .sell_result
                    .state_changes
                    .get(&config.buyer_address)
                    .map(|changes| {
                        if changes.eth_net >= I256::ZERO {
                            changes.eth_net.unsigned_abs()
                        } else {
                            U256::ZERO
                        }
                    })
                    .unwrap_or(U256::ZERO)
            } else {
                U256::ZERO
            };

            writeln!(
                log_file,
                "  ETH Received: {} ETH",
                format_token_amount(eth_received, 18)
            )?;
            writeln!(log_file)?;

            // Run signal detectors
            println!("🔎 Running signal detectors...");
            writeln!(log_file, "Signal Detection Results")?;
            writeln!(log_file, "-----------------------")?;

            // Trading Status Detection
            let _trading_detector = TradingStatusDetector::new();
            writeln!(log_file, "\nTrading Status Detector:")?;
            writeln!(log_file, "  Can Buy: {}", result.buy_result.success)?;
            writeln!(log_file, "  Can Sell: {}", result.sell_result.success)?;

            if result.buy_result.success && result.sell_result.success {
                writeln!(log_file, "  ✅ Status: Trading Enabled")?;
            } else if result.buy_result.success && !result.sell_result.success {
                writeln!(log_file, "  🍯 Status: Honeypot Detected")?;
            } else {
                writeln!(log_file, "  ❌ Status: Trading Disabled")?;
            }

            // Honeypot Detection
            let _honeypot_detector = HoneypotDetector::new();
            writeln!(log_file, "\nHoneypot Detector:")?;

            let is_honeypot = result.buy_result.success && !result.sell_result.success;
            let high_tax_honeypot = if let (Some(buy), Some(sell)) = (buy_tax, sell_tax) {
                sell > tax_config.honeypot_sell_threshold as f64 || (sell - buy) > 30.0
            } else {
                false
            };

            if is_honeypot {
                writeln!(log_file, "  🍯 HONEYPOT: Can buy but cannot sell")?;
            } else if high_tax_honeypot {
                writeln!(log_file, "  ⚠️  HIGH TAX: Sell tax suspiciously high")?;
            } else {
                writeln!(log_file, "  ✅ No honeypot detected")?;
            }

            // Tax Analysis
            writeln!(log_file, "\nTax Analysis:")?;
            if let (Some(buy), Some(sell)) = (buy_tax, sell_tax) {
                writeln!(log_file, "  Buy Tax: {:.2}%", buy)?;
                writeln!(log_file, "  Sell Tax: {:.2}%", sell)?;
                writeln!(log_file, "  Total Tax: {:.2}%", buy + sell)?;
                writeln!(log_file, "  Tax Difference: {:.2}%", (sell - buy).abs())?;

                if buy > tax_config.max_acceptable_buy_tax as f64
                    || sell > tax_config.max_acceptable_sell_tax as f64
                {
                    writeln!(log_file, "  ⚠️  Warning: High taxes detected")?;
                }
                if (sell - buy).abs() > 5.0 {
                    writeln!(log_file, "  ⚠️  Warning: Significant tax asymmetry")?;
                }
            } else if result.buy_result.success && !result.sell_result.success {
                writeln!(log_file, "  Buy Tax: {:.2}%", buy_tax.unwrap_or(0.0))?;
                writeln!(log_file, "  Sell Tax: Cannot calculate (sell failed)")?;
                writeln!(
                    log_file,
                    "  ❌ Cannot determine total tax due to sell failure"
                )?;
            }

            // State Changes Summary
            writeln!(log_file, "\nState Changes Summary:")?;
            writeln!(log_file, "  Buy Transaction:")?;
            writeln!(
                log_file,
                "    Addresses Affected: {}",
                result.buy_result.state_changes.len()
            )?;
            writeln!(log_file, "    Gas Used: {}", result.buy_result.gas_used)?;

            // Detailed state changes for buy transaction
            writeln!(log_file, "\n    Detailed State Changes (Buy):")?;
            for (address, changes) in &result.buy_result.state_changes {
                writeln!(log_file, "      Address: {}", address)?;
                // Convert wei to ETH for display
                let eth_value = changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                writeln!(
                    log_file,
                    "        ETH Net: {:+.18} ETH (raw: {} wei)",
                    eth_value, changes.eth_net
                )?;
                writeln!(log_file, "        Token Changes:")?;
                for (token, amount) in &changes.token_net {
                    // Check if this is a known stablecoin to apply decimals
                    let display_amount = match token.as_str() {
                        "USDC" | "USDT" | "DAI" | "BUSD" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.6} {} (raw: {})", raw_amount / 1e6, token, amount)
                        }
                        "WBTC" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.8} {} (raw: {})", raw_amount / 1e8, token, amount)
                        }
                        "WETH" | "UNI" | "LINK" | "AAVE" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.18} {} (raw: {})", raw_amount / 1e18, token, amount)
                        }
                        _ => format!("{} (token: {})", amount, token),
                    };
                    writeln!(log_file, "          {}", display_amount)?;
                }
            }

            writeln!(log_file, "\n  Sell Transaction:")?;
            writeln!(
                log_file,
                "    Addresses Affected: {}",
                result.sell_result.state_changes.len()
            )?;
            writeln!(log_file, "    Gas Used: {}", result.sell_result.gas_used)?;

            // Detailed state changes for sell transaction
            writeln!(log_file, "\n    Detailed State Changes (Sell):")?;
            for (address, changes) in &result.sell_result.state_changes {
                writeln!(log_file, "      Address: {}", address)?;
                // Convert wei to ETH for display
                let eth_value = changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                writeln!(
                    log_file,
                    "        ETH Net: {:+.18} ETH (raw: {} wei)",
                    eth_value, changes.eth_net
                )?;
                writeln!(log_file, "        Token Changes:")?;
                for (token, amount) in &changes.token_net {
                    // Check if this is a known stablecoin to apply decimals
                    let display_amount = match token.as_str() {
                        "USDC" | "USDT" | "DAI" | "BUSD" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.6} {} (raw: {})", raw_amount / 1e6, token, amount)
                        }
                        "WBTC" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.8} {} (raw: {})", raw_amount / 1e8, token, amount)
                        }
                        "WETH" | "UNI" | "LINK" | "AAVE" => {
                            let raw_amount = amount.to_string().parse::<f64>().unwrap_or(0.0);
                            format!("{:+.18} {} (raw: {})", raw_amount / 1e18, token, amount)
                        }
                        _ => format!("{} (token: {})", amount, token),
                    };
                    writeln!(log_file, "          {}", display_amount)?;
                }
            }

            // Risk Assessment
            writeln!(log_file)?;
            writeln!(log_file, "{}", "=".repeat(50))?;
            writeln!(log_file, "RISK ASSESSMENT")?;
            writeln!(log_file, "{}", "=".repeat(50))?;

            let mut risk_score = 0;
            let mut risk_factors: Vec<String> = Vec::new();

            if is_honeypot {
                risk_score += 100;
                risk_factors.push("🍯 Honeypot: Cannot sell tokens".to_string());
            }

            if let Some(buy) = buy_tax {
                if buy > tax_config.max_acceptable_buy_tax as f64 {
                    risk_score += 20;
                    risk_factors.push(format!("⚠️  High buy tax: {:.2}%", buy));
                }
                if buy > (tax_config.max_acceptable_buy_tax + 5) as f64 {
                    risk_score = 100; // Set to max score
                    risk_factors.push("❌ Extreme buy tax".to_string());
                }
            }

            if let Some(sell) = sell_tax {
                if sell > tax_config.max_acceptable_sell_tax as f64 {
                    risk_score += 20;
                    risk_factors.push(format!("⚠️  High sell tax: {:.2}%", sell));
                }
                if sell > (tax_config.max_acceptable_sell_tax + 5) as f64 {
                    risk_score = 100; // Set to max score
                    risk_factors.push("❌ Extreme sell tax".to_string());
                }
            }

            if !result.buy_result.success {
                risk_score = 100; // Set to max score
                risk_factors.push("❌ Cannot buy tokens".to_string());
            }

            writeln!(log_file, "Risk Score: {}/100", risk_score.min(100))?;
            writeln!(log_file, "\nRisk Factors:")?;
            if risk_factors.is_empty() {
                writeln!(log_file, "  ✅ No significant risks detected")?;
            } else {
                for factor in risk_factors {
                    writeln!(log_file, "  {}", factor)?;
                }
            }

            writeln!(log_file, "\nRecommendation:")?;
            if risk_score >= 80 {
                writeln!(log_file, "  ❌ AVOID - Extremely high risk")?;
            } else if risk_score >= 50 {
                writeln!(log_file, "  ⚠️  CAUTION - Significant risks detected")?;
            } else if risk_score >= 20 {
                writeln!(log_file, "  ⚠️  MONITOR - Some risks present")?;
            } else {
                writeln!(log_file, "  ✅ LOW RISK - Appears safe for trading")?;
            }

            println!("✅ Analysis complete!");
        }
        Err(e) => {
            writeln!(log_file, "❌ Simulation Failed")?;
            writeln!(log_file, "Error: {}", e)?;
            println!("❌ Simulation failed: {}", e);
        }
    }

    writeln!(log_file)?;
    writeln!(log_file, "{}", "=".repeat(50))?;
    writeln!(log_file, "Analysis completed at: {}", Local::now())?;

    println!("📄 Results saved to: {}", log_file_path);

    Ok(())
}
