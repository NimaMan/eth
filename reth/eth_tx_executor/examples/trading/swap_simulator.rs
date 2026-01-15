//! Swap Simulator for ETH Kartal
//! 
//! Simulates swap transactions without executing them on-chain
//! Perfect for testing swap logic with KARTAL_KILIT wallet

use eth_kartal::pools::{PoolFactory, SwapParams};
use eth_kartal::common::validation::{validate_slippage, MIN_SLIPPAGE_PERCENT, MAX_SLIPPAGE_PERCENT, DEFAULT_SLIPPAGE_PERCENT};
use ethers::prelude::*;
use ethers::utils::{format_units, parse_ether, parse_units};
use std::sync::Arc;

// Token addresses on Ethereum mainnet
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";

// KARTAL_KILIT wallet address
const KARTAL_WALLET: &str = "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ETH Kartal Swap Simulator ===");
    println!("Wallet: {}", KARTAL_WALLET);
    println!("Mode: SIMULATION ONLY (no real transactions)\n");
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    let provider = Arc::new(provider);
    
    // Create pool factory
    let pool_factory = PoolFactory::new(provider.clone());
    
    // Check current ETH balance
    let eth_balance = provider.get_balance(KARTAL_WALLET.parse::<Address>()?, None).await?;
    println!("Current ETH Balance: {} ETH", format_units(eth_balance, "ether")?);
    
    // Note: Simulation engine is available in the risk module but we'll use direct pool quotes
    println!("Simulation Mode: Direct pool quotes without executing transactions");
    
    println!("\n📊 Available Simulations:");
    println!("1. Swap 0.05 ETH → USDC (test slippage validation)");
    println!("2. Swap 0.05 ETH → USDT");
    println!("3. Swap 100 USDC → ETH");
    println!("4. Custom swap");
    println!("5. Test extreme slippage values");
    println!("6. Exit");
    
    loop {
        println!("\nSelect option (1-6): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => simulate_eth_to_usdc(&pool_factory, &provider).await?,
            "2" => simulate_eth_to_usdt(&pool_factory, &provider).await?,
            "3" => simulate_usdc_to_eth(&pool_factory, &provider).await?,
            "4" => simulate_custom_swap(&pool_factory, &provider).await?,
            "5" => test_slippage_validation().await?,
            "6" => break,
            _ => println!("Invalid option"),
        }
    }
    
    Ok(())
}

/// Simulate ETH → USDC swap with slippage testing
async fn simulate_eth_to_usdc(
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Simulating: 0.05 ETH → USDC (with slippage validation testing)");
    
    let eth_amount = parse_ether("0.05")?;
    
    // Find pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDC.parse()?,
    ).await?;
    
    println!("Found pool: {} ({})", pool.address(), pool.protocol());
    
    // Get quote
    let usdc_out = pool.get_amount_out(eth_amount, WETH.parse()?).await?;
    let usdc_out_human = format_units(usdc_out, 6)?; // USDC has 6 decimals
    
    println!("\n📈 Swap Quote:");
    println!("Input: 0.05 ETH");
    println!("Output: {} USDC", usdc_out_human);
    
    // Calculate price
    let price = usdc_out_human.parse::<f64>()? / 0.05;
    println!("Implied ETH Price: ${:.2} USDC", price);
    
    // Test different slippage values
    println!("\n🔬 Testing Slippage Validation:");
    let test_slippages = vec![0.001, 0.01, 0.03, 0.05, 0.10, 0.15]; // 0.1%, 1%, 3%, 5%, 10%, 15%
    
    for slippage in test_slippages {
        print!("Slippage {:.1}%: ", slippage * 100.0);
        match validate_slippage(slippage) {
            Ok(validated) => {
                let min_out = apply_slippage(usdc_out, validated);
                println!("✅ Valid → Min output: {} USDC", format_units(min_out, 6)?);
            }
            Err(e) => {
                println!("❌ Invalid → {}", e);
            }
        }
    }
    
    // Simulate transaction cost
    let gas_price = provider.get_gas_price().await?;
    let estimated_gas = U256::from(200_000); // Typical swap gas
    let gas_cost = gas_price * estimated_gas;
    let gas_cost_eth = format_units(gas_cost, "ether")?;
    
    println!("\n💸 Transaction Costs:");
    println!("Gas Price: {} gwei", format_units(gas_price, "gwei")?);
    println!("Estimated Gas: {}", estimated_gas);
    println!("Total Cost: {} ETH", gas_cost_eth);
    
    // Build swap params with validated slippage
    let validated_slippage = validate_slippage(DEFAULT_SLIPPAGE_PERCENT / 100.0)?;
    let swap_params = SwapParams {
        token_in: WETH.parse()?,
        token_out: USDC.parse()?,
        amount_in: eth_amount,
        amount_out_min: apply_slippage(usdc_out, validated_slippage),
        recipient: KARTAL_WALLET.parse()?,
        deadline: current_timestamp() + 300, // 5 minutes
    };
    
    println!("\n📝 Swap Parameters (with validated {}% slippage):", DEFAULT_SLIPPAGE_PERCENT);
    println!("Min Output: {} USDC", format_units(swap_params.amount_out_min, 6)?);
    println!("Deadline: {} seconds from now", 300);
    
    println!("\n✅ SIMULATION COMPLETE - No transaction sent");
    println!("Would receive approximately {} USDC for 0.05 ETH", usdc_out_human);
    
    Ok(())
}

/// Simulate ETH → USDT swap
async fn simulate_eth_to_usdt(
    pool_factory: &PoolFactory,
    _provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Simulating: 0.05 ETH → USDT");
    
    let eth_amount = parse_ether("0.05")?;
    
    // Find pool
    let pool = pool_factory.find_best_pool(
        WETH.parse()?,
        USDT.parse()?,
    ).await?;
    
    println!("Found pool: {} ({})", pool.address(), pool.protocol());
    
    // Get quote
    let usdt_out = pool.get_amount_out(eth_amount, WETH.parse()?).await?;
    let usdt_out_human = format_units(usdt_out, 6)?; // USDT has 6 decimals
    
    println!("\n📈 Swap Quote:");
    println!("Input: 0.05 ETH");
    println!("Output: {} USDT", usdt_out_human);
    
    // Calculate price
    let price = usdt_out_human.parse::<f64>()? / 0.05;
    println!("Implied ETH Price: ${:.2} USDT", price);
    
    println!("\n✅ SIMULATION COMPLETE - No transaction sent");
    
    Ok(())
}

/// Simulate USDC → ETH swap
async fn simulate_usdc_to_eth(
    pool_factory: &PoolFactory,
    _provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Simulating: 100 USDC → ETH");
    
    let usdc_amount = parse_units("100", 6)?.into(); // 100 USDC
    
    // Find pool
    let pool = pool_factory.find_best_pool(
        USDC.parse()?,
        WETH.parse()?,
    ).await?;
    
    println!("Found pool: {} ({})", pool.address(), pool.protocol());
    
    // Get quote
    let eth_out = pool.get_amount_out(usdc_amount, USDC.parse::<Address>()?).await?;
    let eth_out_human = format_units(eth_out, "ether")?;
    
    println!("\n📈 Swap Quote:");
    println!("Input: 100 USDC");
    println!("Output: {} ETH", eth_out_human);
    
    // Calculate price
    let price = 100.0 / eth_out_human.parse::<f64>()?;
    println!("Implied ETH Price: ${:.2} USDC", price);
    
    // Check if user has USDC balance (simulation)
    println!("\n⚠️  Note: This assumes you have 100 USDC in your wallet");
    println!("In a real transaction, we would check your USDC balance first");
    
    println!("\n✅ SIMULATION COMPLETE - No transaction sent");
    
    Ok(())
}

/// Simulate custom swap
async fn simulate_custom_swap(
    pool_factory: &PoolFactory,
    provider: &Arc<Provider<Http>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Custom Swap Simulation");
    
    println!("Select token pair:");
    println!("1. ETH/USDC");
    println!("2. ETH/USDT");
    println!("3. ETH/DAI");
    println!("4. USDC/USDT");
    
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;
    
    let (token_in, token_out, decimals_in, decimals_out) = match choice.trim() {
        "1" => (WETH, USDC, 18, 6),
        "2" => (WETH, USDT, 18, 6),
        "3" => (WETH, DAI, 18, 18),
        "4" => (USDC, USDT, 6, 6),
        _ => {
            println!("Invalid choice");
            return Ok(());
        }
    };
    
    println!("Enter amount to swap: ");
    let mut amount_str = String::new();
    std::io::stdin().read_line(&mut amount_str)?;
    let amount: U256 = parse_units(amount_str.trim(), decimals_in)?.into();
    
    // Find pool and get quote
    let pool = pool_factory.find_best_pool(
        token_in.parse()?,
        token_out.parse()?,
    ).await?;
    
    let amount_out = pool.get_amount_out(amount, token_in.parse::<Address>()?).await?;
    
    println!("\n📈 Simulation Results:");
    println!("Pool: {} ({})", pool.address(), pool.protocol());
    println!("Input: {} tokens", format_units(amount, decimals_in)?);
    println!("Output: {} tokens", format_units(amount_out, decimals_out)?);
    
    // Estimate gas
    let gas_price = provider.get_gas_price().await?;
    let gas_cost = gas_price * U256::from(200_000);
    println!("Estimated Gas Cost: {} ETH", format_units(gas_cost, "ether")?);
    
    println!("\n✅ SIMULATION COMPLETE - No transaction sent");
    
    Ok(())
}

/// Apply slippage to amount
fn apply_slippage(amount: U256, slippage: f64) -> U256 {
    let factor = 1.0 - slippage;
    let adjusted = amount.as_u128() as f64 * factor;
    U256::from(adjusted as u128)
}

/// Get current timestamp
fn current_timestamp() -> U256 {
    U256::from(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs())
}

/// Test extreme slippage values to verify validation
async fn test_slippage_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔬 Comprehensive Slippage Validation Testing");
    println!("Testing boundary conditions and edge cases");
    println!("Validation range: {:.1}% - {:.1}%", MIN_SLIPPAGE_PERCENT, MAX_SLIPPAGE_PERCENT);
    
    // Test cases: (input_value, description, expected_result)
    let test_cases = vec![
        (0.0001, "0.01% (way too low)", false),
        (0.0005, "0.05% (still too low)", false),
        (0.001, "0.1% (minimum valid)", true),
        (0.005, "0.5% (low but valid)", true),
        (0.01, "1% (typical low)", true),
        (0.03, "3% (default)", true),
        (0.05, "5% (typical high)", true),
        (0.10, "10% (maximum valid)", true),
        (0.11, "11% (too high)", false),
        (0.15, "15% (way too high)", false),
        (0.50, "50% (extreme)", false),
        (1.0, "100% (impossible)", false),
        (-0.01, "Negative slippage", false),
        (1.5, "150% (impossible)", false),
    ];
    
    println!("\n📊 Test Results:");
    println!("{:<15} {:<20} {:<10} {:<30}", "Input", "Description", "Expected", "Result");
    println!("{:-<75}", "");
    
    let mut passed = 0;
    let total = test_cases.len();
    
    for (input, description, should_pass) in test_cases {
        let result = validate_slippage(input);
        let actual_pass = result.is_ok();
        let status = if actual_pass == should_pass {
            passed += 1;
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        
        let result_text = match result {
            Ok(validated) => format!("Valid: {:.4}", validated),
            Err(e) => format!("Error: {}", e.to_string().chars().take(25).collect::<String>()),
        };
        
        println!("{:<15} {:<20} {:<10} {:<30} {}", 
                format!("{:.1}%", input * 100.0), 
                description, 
                if should_pass { "PASS" } else { "FAIL" },
                result_text,
                status);
    }
    
    println!("{:-<75}", "");
    println!("Test Summary: {}/{} tests passed ({:.1}%)", 
             passed, total, (passed as f64 / total as f64) * 100.0);
    
    if passed == total {
        println!("🎉 All slippage validation tests PASSED!");
        println!("\n💡 Key Takeaways:");
        println!("  • Minimum slippage: {:.1}% (protects against failed trades)", MIN_SLIPPAGE_PERCENT);
        println!("  • Maximum slippage: {:.1}% (protects against excessive losses)", MAX_SLIPPAGE_PERCENT);
        println!("  • Default slippage: {:.1}% (balanced protection)", DEFAULT_SLIPPAGE_PERCENT);
        println!("  • Validation prevents dangerous slippage settings");
    } else {
        println!("⚠️  Some validation tests failed - check implementation!");
    }
    
    // Interactive slippage testing
    println!("\n🎮 Interactive Slippage Testing");
    println!("Enter custom slippage values to test (or 'done' to finish):");
    
    loop {
        println!("\nEnter slippage percentage (e.g., 2.5 for 2.5%): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.to_lowercase() == "done" {
            break;
        }
        
        match input.parse::<f64>() {
            Ok(percentage) => {
                let decimal = percentage / 100.0;
                match validate_slippage(decimal) {
                    Ok(validated) => {
                        println!("✅ Valid slippage: {:.1}% → normalized to {:.6}", 
                                percentage, validated);
                        println!("   This would be safe to use in production trading");
                    }
                    Err(e) => {
                        println!("❌ Invalid slippage: {}", e);
                        println!("   This would be rejected by eth_kartal risk management");
                    }
                }
            }
            Err(_) => {
                println!("❌ Invalid input - please enter a number");
            }
        }
    }
    
    Ok(())
}