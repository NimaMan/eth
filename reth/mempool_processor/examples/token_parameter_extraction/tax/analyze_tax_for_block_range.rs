/// Token Tax Analysis for Block Range
///
/// Analyzes buy/sell taxes for a given token across a range of blocks
/// by simulating sequential buy->sell transactions.
///
/// For each block, we log:
/// - Block number
/// - Buy/sell simulation results
/// - Calculated taxes
/// - Any errors or restrictions

use mempool_processor::token_parameter_extraction::{TaxCalculator, TokenInfo, PoolReserves};
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::{SolCall, SolValue};
use alloy_rpc_types::TransactionRequest;
use std::str::FromStr;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use eyre::Result;
use chrono::Local;

fn pow10(decimals: u8) -> U256 {
    U256::from(10).pow(U256::from(decimals))
}

fn format_token_amount(amount: U256, decimals: u8) -> String {
    let divisor = pow10(decimals);
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    if fraction == U256::ZERO {
        format!("{}", whole)
    } else {
        // Format with appropriate decimal places
        let fraction_str = format!("{:0>width$}", fraction, width = decimals as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole)
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

// ERC20 function signatures
alloy_sol_types::sol! {
    function decimals() external view returns (uint8);
    function totalSupply() external view returns (uint256);
    function name() external view returns (string);
    function symbol() external view returns (string);
}

async fn fetch_token_info(token_address: Address, block_number: u64) -> Result<(TokenInfo, String, String)> {
    let provider = ProviderBuilder::new()
        .connect_http("http://localhost:8545".parse()?);
    
    // Get decimals
    let decimals_data = decimalsCall {}.abi_encode();
    let decimals_tx = TransactionRequest::default()
        .to(token_address)
        .input(decimals_data.into());
    let decimals_result = provider
        .call(decimals_tx)
        .block(block_number.into())
        .await?;
    let decimals_bytes = decimals_result.to_vec();
    // For uint8, the result is padded to 32 bytes with the value in the last byte
    let decimals = if decimals_bytes.len() >= 32 {
        decimals_bytes[31]
    } else {
        return Err(eyre::eyre!("Invalid decimals response"));
    };
    
    // Get total supply
    let total_supply_data = totalSupplyCall {}.abi_encode();
    let total_supply_tx = TransactionRequest::default()
        .to(token_address)
        .input(total_supply_data.into());
    let total_supply_result = provider
        .call(total_supply_tx)
        .block(block_number.into())
        .await?;
    let total_supply_bytes = total_supply_result.to_vec();
    let total_supply = U256::abi_decode(&total_supply_bytes)?;
    
    // Get name
    let name_data = nameCall {}.abi_encode();
    let name_tx = TransactionRequest::default()
        .to(token_address)
        .input(name_data.into());
    let name_result = provider
        .call(name_tx)
        .block(block_number.into())
        .await?;
    let name_bytes = name_result.to_vec();
    let name = String::abi_decode(&name_bytes).unwrap_or_else(|_| "Unknown".to_string());
    
    // Get symbol  
    let symbol_data = symbolCall {}.abi_encode();
    let symbol_tx = TransactionRequest::default()
        .to(token_address)
        .input(symbol_data.into());
    let symbol_result = provider
        .call(symbol_tx)
        .block(block_number.into())
        .await?;
    let symbol_bytes = symbol_result.to_vec();
    let symbol = String::abi_decode(&symbol_bytes).unwrap_or_else(|_| "UNKNOWN".to_string());
    
    // Return basic TokenInfo for the tax calculator plus name and symbol
    Ok((TokenInfo {
        address: token_address,
        decimals,
        total_supply,
    }, name, symbol))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <token_address> <pool_address> <start_block> [end_block]", args[0]);
        println!("Example: {} 0x123... 0x456... 23002159 23002359", args[0]);
        return Ok(());
    }
    
    let token_address = Address::from_str(&args[1])?;
    let pool_address = Address::from_str(&args[2])?;
    let start_block: u64 = args[3].parse()?;
    let end_block: u64 = if args.len() > 4 {
        args[4].parse()?
    } else {
        start_block + 200 // Default to 200 blocks
    };
    
    println!("\n🔍 Token Tax Analysis\n");
    println!("Token: {}", token_address);
    println!("Pool: {}", pool_address);
    println!("Blocks: {} to {}\n", start_block, end_block);
    
    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/test";
    create_dir_all(log_dir)?;
    
    // Create log file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let token_short = format!("{:?}", token_address).chars().take(8).collect::<String>();
    let log_file_path = format!("{}/token_tax_{}_{}.log", log_dir, token_short, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)?;
    
    println!("Logging to: {}", log_file_path);
    
    // Try to fetch token info
    let (token_info, name, symbol) = fetch_token_info(token_address, start_block).await?;
    
    // Print token info to console
    println!("Token Info:");
    println!("  Name: {}", name);
    println!("  Symbol: {}", symbol);
    println!("  Decimals: {}", token_info.decimals);
    println!("  Total Supply: {} ({})", token_info.total_supply, format_token_amount(token_info.total_supply, token_info.decimals));
    println!();
    
    // Write header
    writeln!(log_file, "Token Tax Analysis - {}", Local::now())?;
    writeln!(log_file, "Token: {} ({} - {})", token_address, name, symbol)?;
    writeln!(log_file, "Decimals: {}", token_info.decimals)?;
    writeln!(log_file, "Total Supply: {} ({})", token_info.total_supply, format_token_amount(token_info.total_supply, token_info.decimals))?;
    writeln!(log_file, "Pool: {}", pool_address)?;
    writeln!(log_file, "Block Range: {} to {}", start_block, end_block)?;
    writeln!(log_file, "=")?;
    writeln!(log_file, "Block | Buy Amount | Tokens Received | Buy Tax | Sell Amount | Sell Result | Sell Tax | Error")?;
    writeln!(log_file, "------|------------|----------------|---------|------------|-------------|----------|------")?;
    
    // Initialize tax calculator
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let tax_calculator = TaxCalculator::new(reth_datadir)?;
    
    let router_address = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?;
    
    // Test buyer address
    let buyer_address = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?;
    
    // Dummy pool reserves - not used in simulation but required by API
    let pool_reserves = PoolReserves {
        token_reserve: U256::from(1),
        eth_reserve: U256::from(1),
    };
    
    println!("Testing blocks {} to {}", start_block, end_block);
    
    for block_number in start_block..=end_block {
        print!("Block {} ... ", block_number);
        
        match tax_calculator.calculate_taxes_via_simulation(
            buyer_address,
            &token_info,
            pool_address,
            router_address,
            &pool_reserves,
            block_number,
        ).await {
            Ok((buy_tax, sell_tax, tokens_bought, tokens_sold)) => {
                if sell_tax >= 0.0 {
                    writeln!(log_file, "{} | 0.1 ETH | {} | {:.1}% | {} | SUCCESS | {:.1}% | -", 
                        block_number, tokens_bought, buy_tax, tokens_sold, sell_tax)?;
                    println!("✅ Buy: {:.1}%, Sell: {:.1}% (Bought: {}, Sold: {})", 
                        buy_tax, sell_tax, 
                        format_token_amount(tokens_bought, token_info.decimals), 
                        format_token_amount(tokens_sold, token_info.decimals));
                } else {
                    writeln!(log_file, "{} | 0.1 ETH | {} | {:.1}% | {} | FAILED | - | Transfer failed", 
                        block_number, tokens_bought, buy_tax, tokens_sold)?;
                    println!("❌ Buy: {:.1}%, Sell: FAILED (Bought: {}, Tried: {})", 
                        buy_tax, 
                        format_token_amount(tokens_bought, token_info.decimals), 
                        format_token_amount(tokens_sold, token_info.decimals));
                }
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                // Log full error for debugging
                println!("❌ Full error: {}", error_msg);
                
                // Extract key error info
                let short_error = if error_msg.contains("revert") {
                    if let Some(start) = error_msg.find("revert") {
                        let revert_part = &error_msg[start..];
                        if let Some(end) = revert_part.find('"').and_then(|i| revert_part[i+1..].find('"').map(|j| i + j + 1)) {
                            &revert_part[..end+1]
                        } else {
                            "Reverted"
                        }
                    } else {
                        "Reverted"
                    }
                } else if error_msg.contains("failed") {
                    "Simulation failed"
                } else {
                    &error_msg[..error_msg.len().min(50)]
                };
                writeln!(log_file, "{} | - | - | - | - | FAILED | - | {}", block_number, short_error)?;
            }
        }
        
        log_file.flush()?;
    }
    
    writeln!(log_file, "=")?;
    writeln!(log_file, "Analysis Complete: {}", Local::now())?;
    
    println!("\n📊 Summary:");
    println!("✅ Results saved to: {}", log_file_path);
    
    Ok(())
}

