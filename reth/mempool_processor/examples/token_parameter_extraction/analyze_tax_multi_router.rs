/// Multi-Router Token Tax Analysis
///
/// Tests token buy/sell taxes across multiple routers (Uniswap V2, Maestro, Banana Gun)
/// to identify which routers are whitelisted and what taxes apply on each.
///
/// This helps identify sophisticated honeypot patterns where tokens only allow
/// selling through specific whitelisted routers.

use mempool_processor::token_tracking::token_parameter_extraction::{TaxCalculator, TokenInfo, PoolReserves};
use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_sol_types::{SolCall, SolValue};
use alloy_rpc_types::TransactionRequest;
use std::str::FromStr;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::time::Instant;
use eyre::Result;
use chrono::Local;
use std::collections::HashMap;

// Known router addresses
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const MAESTRO_ROUTER: &str = "0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e";
const BANANA_GUN_ROUTER: &str = "0x3328F7333f2C1d8c16afCB0940B4930Dcc7C35Bb"; // Banana Gun Bot

#[derive(Debug, Clone)]
struct RouterInfo {
    name: String,
    address: Address,
}

impl RouterInfo {
    fn new(name: &str, address: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            address: Address::from_str(address)?,
        })
    }
}

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
    let symbol = String::abi_decode(&symbol_bytes).unwrap_or_else(|_| "???".to_string());
    
    Ok((
        TokenInfo {
            address: token_address,
            decimals,
            total_supply,
        },
        name,
        symbol,
    ))
}

#[derive(Debug)]
struct RouterTestResult {
    router_name: String,
    buy_tax: Option<f64>,
    sell_tax: Option<f64>,
    buy_success: bool,
    sell_success: bool,
    tokens_bought: U256,
    error_msg: Option<String>,
    elapsed_ms: f64,
}

async fn test_router(
    router: &RouterInfo,
    tax_calculator: &TaxCalculator,
    token_info: &TokenInfo,
    pool_address: Address,
    block_number: u64,
) -> RouterTestResult {
    let start_time = Instant::now();
    let buyer_address = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689").unwrap();
    
    match tax_calculator.calculate_taxes_via_simulation(
        buyer_address,
        token_info,
        pool_address,
        router.address,
        &PoolReserves { token_reserve: U256::ZERO, eth_reserve: U256::ZERO },
        block_number,
    ).await {
        Ok((buy_tax, sell_tax, tokens_bought, _tokens_sold)) => {
            RouterTestResult {
                router_name: router.name.clone(),
                buy_tax: Some(buy_tax),
                sell_tax: if sell_tax >= 0.0 { Some(sell_tax) } else { None },
                buy_success: true,
                sell_success: sell_tax >= 0.0,
                tokens_bought,
                error_msg: if sell_tax < 0.0 { Some("Sell failed".to_string()) } else { None },
                elapsed_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            }
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            let buy_failed = error_msg.contains("Buy simulation failed");
            RouterTestResult {
                router_name: router.name.clone(),
                buy_tax: None,
                sell_tax: None,
                buy_success: !buy_failed,
                sell_success: false,
                tokens_bought: U256::ZERO,
                error_msg: Some(error_msg),
                elapsed_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <token_address> <pool_address> <start_block> [end_block]", args[0]);
        println!("Example: {} 0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84 0x5423621C6D3E22465876deb92aD4f40Dc25b62A3 22939560 22939570", args[0]);
        return Ok(());
    }
    
    let token_address = Address::from_str(&args[1])?;
    let pool_address = Address::from_str(&args[2])?;
    let start_block: u64 = args[3].parse()?;
    let end_block: u64 = if args.len() > 4 {
        args[4].parse()?
    } else {
        start_block + 10
    };
    
    println!("\n🔍 Multi-Router Token Tax Analysis\n");
    println!("Token: {}", token_address);
    println!("Pool: {}", pool_address);
    println!("Blocks: {} to {}", start_block, end_block);
    println!("Testing routers: Uniswap V2, Maestro, Banana Gun\n");
    
    // Create log directory
    let log_dir = "/home/nima/code/crypto/logs/mempool/dev/token_parameter_extraction";
    create_dir_all(log_dir)?;
    
    // Create log file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let token_short = format!("{:?}", token_address).chars().take(8).collect::<String>();
    let log_file_path = format!("{}/multi_router_tax_{}_{}.log", log_dir, token_short, timestamp);
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)?;
    
    println!("Logging to: {}", log_file_path);
    
    // Fetch token info
    let (token_info, name, symbol) = fetch_token_info(token_address, start_block).await?;
    
    println!("Token Info:");
    println!("  Name: {}", name);
    println!("  Symbol: {}", symbol);
    println!("  Decimals: {}", token_info.decimals);
    println!("  Total Supply: {} ({})", token_info.total_supply, format_token_amount(token_info.total_supply, token_info.decimals));
    println!();
    
    // Initialize routers
    let routers = vec![
        RouterInfo::new("Uniswap V2", UNISWAP_V2_ROUTER)?,
        RouterInfo::new("Maestro", MAESTRO_ROUTER)?,
        RouterInfo::new("Banana Gun", BANANA_GUN_ROUTER)?,
    ];
    
    // Create tax calculator (using reth datadir)
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let tax_calculator = TaxCalculator::new(&reth_datadir)?;
    
    // Write header to log file
    writeln!(log_file, "Multi-Router Token Tax Analysis - {}", Local::now())?;
    writeln!(log_file, "Token: {} ({} - {})", token_address, name, symbol)?;
    writeln!(log_file, "Decimals: {}", token_info.decimals)?;
    writeln!(log_file, "Total Supply: {} ({})", token_info.total_supply, format_token_amount(token_info.total_supply, token_info.decimals))?;
    writeln!(log_file, "Pool: {}", pool_address)?;
    writeln!(log_file, "Block Range: {} to {}", start_block, end_block)?;
    writeln!(log_file, "=")?;
    writeln!(log_file, "Block | Router | Buy Result | Buy Tax | Sell Result | Sell Tax | Tokens Bought | Time(ms) | Error")?;
    writeln!(log_file, "------|--------|------------|---------|-------------|----------|---------------|----------|------")?;
    
    println!("Testing blocks {} to {}", start_block, end_block);
    
    // Test each block
    for block_number in start_block..=end_block {
        println!("\n📦 Block {}", block_number);
        println!("{}", "=".repeat(60));
        
        let mut block_results: HashMap<String, RouterTestResult> = HashMap::new();
        
        // Test each router
        for router in &routers {
            print!("  Testing {} ... ", router.name);
            
            let result = test_router(
                router,
                &tax_calculator,
                &token_info,
                pool_address,
                block_number,
            ).await;
            
            // Print result summary
            if result.buy_success && result.sell_success {
                println!("✅ Buy: {:.1}%, Sell: {:.1}% ({:.1}ms)",
                    result.buy_tax.unwrap_or(0.0),
                    result.sell_tax.unwrap_or(0.0),
                    result.elapsed_ms);
            } else if result.buy_success && !result.sell_success {
                println!("⚠️  Buy: {:.1}%, Sell: BLOCKED ({:.1}ms)",
                    result.buy_tax.unwrap_or(0.0),
                    result.elapsed_ms);
            } else {
                println!("❌ BLOCKED ({:.1}ms)", result.elapsed_ms);
            }
            
            // Write to log
            writeln!(log_file, "{} | {} | {} | {} | {} | {} | {} | {:.1} | {}",
                block_number,
                router.name,
                if result.buy_success { "SUCCESS" } else { "FAILED" },
                result.buy_tax.map(|t| format!("{:.1}%", t)).unwrap_or_else(|| "-".to_string()),
                if result.sell_success { "SUCCESS" } else { "FAILED" },
                result.sell_tax.map(|t| format!("{:.1}%", t)).unwrap_or_else(|| "-".to_string()),
                if result.tokens_bought > U256::ZERO {
                    format_token_amount(result.tokens_bought, token_info.decimals)
                } else {
                    "-".to_string()
                },
                result.elapsed_ms,
                result.error_msg.as_deref().unwrap_or("-")
            )?;
            
            block_results.insert(router.name.clone(), result);
        }
        
        // Analyze differences between routers
        let working_routers: Vec<_> = block_results.iter()
            .filter(|(_, r)| r.buy_success || r.sell_success)
            .map(|(name, _)| name.clone())
            .collect();
        
        if !working_routers.is_empty() {
            println!("\n  📊 Summary for block {}:", block_number);
            
            // Check if only specific routers work
            let uniswap_works = block_results.get("Uniswap V2")
                .map(|r| r.buy_success || r.sell_success)
                .unwrap_or(false);
            let maestro_works = block_results.get("Maestro")
                .map(|r| r.buy_success || r.sell_success)
                .unwrap_or(false);
            let banana_works = block_results.get("Banana Gun")
                .map(|r| r.buy_success || r.sell_success)
                .unwrap_or(false);
            
            if maestro_works && !uniswap_works {
                println!("  ⚠️  Maestro-only trading detected (honeypot for Uniswap users)");
            }
            if banana_works && !uniswap_works {
                println!("  ⚠️  Banana Gun-only trading detected (honeypot for Uniswap users)");
            }
            
            // Check for sell restrictions
            for (name, result) in &block_results {
                if result.buy_success && !result.sell_success {
                    println!("  🚨 {} can buy but NOT sell (honeypot behavior)", name);
                }
            }
        }
        
        log_file.flush()?;
    }
    
    writeln!(log_file, "=")?;
    writeln!(log_file, "Analysis Complete: {}", Local::now())?;
    
    println!("\n📊 Final Summary:");
    println!("✅ Results saved to: {}", log_file_path);
    
    Ok(())
}