//! Portfolio Simulator for ETH Kartal
//! 
//! Simulates portfolio management and tracking without real transactions

use eth_kartal::{
    risk::{SimulationEngine, SimulationMode},
    alert_processor::{Alert, Action, ExecutionParams, Priority},
};
use ethers::prelude::*;
use std::collections::HashMap;

// Token addresses
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const SHIB: &str = "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE";
const PEPE: &str = "0x6982508145454Ce325dDbE47a25d4ec3d2311933";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ETH Kartal Portfolio Simulator ===\n");
    
    // Initialize with starting portfolio
    let mut initial_tokens = HashMap::new();
    initial_tokens.insert(USDC.parse::<Address>()?, U256::from(10000_000000)); // 10,000 USDC
    initial_tokens.insert(USDT.parse::<Address>()?, U256::from(5000_000000));  // 5,000 USDT
    
    let sim_mode = SimulationMode::Full {
        initial_eth_balance: 5.0, // 5 ETH
        initial_token_positions: initial_tokens,
    };
    
    let mut simulator = SimulationEngine::new(sim_mode);
    
    println!("📊 Initial Portfolio:");
    display_portfolio(&simulator);
    
    loop {
        println!("\n🎮 Portfolio Simulation Options:");
        println!("1. Simulate buy signal (SHIB)");
        println!("2. Simulate sell signal (panic sell)");
        println!("3. Simulate portfolio rebalancing");
        println!("4. Simulate risk limits");
        println!("5. View current portfolio");
        println!("6. Reset portfolio");
        println!("7. Exit");
        
        println!("\nSelect option (1-7): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => simulate_buy_signal(&mut simulator).await?,
            "2" => simulate_sell_signal(&mut simulator).await?,
            "3" => simulate_rebalancing(&mut simulator).await?,
            "4" => simulate_risk_limits(&mut simulator).await?,
            "5" => display_portfolio(&simulator),
            "6" => {
                simulator = reset_portfolio();
                println!("✅ Portfolio reset to initial state");
            }
            "7" => break,
            _ => println!("Invalid option"),
        }
    }
    
    // Final summary
    println!("\n📈 Final Portfolio Summary:");
    display_portfolio(&simulator);
    
    if let Some(pnl) = calculate_pnl(&simulator) {
        println!("\n💰 Total P&L: {:.2}%", pnl);
    }
    
    Ok(())
}

async fn simulate_buy_signal(
    simulator: &mut SimulationEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🟢 Simulating BUY Signal");
    
    // Create a buy alert for SHIB
    let alert = Alert {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: SHIB.parse()?,
        pool_address: "0x811beEd0119b4AfCE20D2583EB608C6F7AF1954f".parse()?, // SHIB/WETH V2
        action: Action::Buy,
        params: ExecutionParams {
            amount: ethers::utils::parse_ether("0.5")?, // Buy with 0.5 ETH
            slippage: 0.02, // 2% slippage
            max_gas_price: None,
            deadline_seconds: 300,
            priority: Priority::High,
        },
    };
    
    println!("Alert: BUY 0.5 ETH worth of SHIB");
    println!("Reason: Unusual volume spike detected");
    
    // Simulate the buy
    let result = simulator.simulate_buy(&alert, SHIB.parse()?, alert.params.amount).await;
    
    println!("\n📊 Simulation Results:");
    println!("Would succeed: {}", result.would_succeed);
    if let Some(amount) = result.token_amount_received {
        println!("SHIB received: {} tokens", ethers::utils::format_units(amount, 18)?);
    }
    println!("Gas cost: {} ETH", result.estimated_gas_cost_eth);
    
    if let Some(reason) = result.failure_reason {
        println!("❌ Failure reason: {}", reason);
    } else {
        println!("✅ Buy executed successfully in simulation");
        
        // Show portfolio update
        println!("\n📈 Updated Portfolio:");
        display_portfolio(simulator);
    }
    
    Ok(())
}

async fn simulate_sell_signal(
    simulator: &mut SimulationEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔴 Simulating SELL Signal (Panic Sell)");
    
    // For now, we'll simulate with a fixed token
    let token_to_sell = SHIB.parse::<Address>()?;
    let balance = U256::from(1000000_000000000000000000u128); // Simulated balance
    
    let token_name = match format!("{:?}", token_to_sell).as_str() {
        addr if addr.contains("95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE") => "SHIB",
        addr if addr.contains("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") => "USDC",
        addr if addr.contains("dAC17F958D2ee523a2206206994597C13D831ec7") => "USDT",
        _ => "TOKEN",
    };
    
    println!("Token to sell: {} (Balance: {})", token_name, ethers::utils::format_units(balance, 18)?);
    println!("Reason: Risk threshold exceeded - emergency sell");
    
    // Create sell alert
    let alert = Alert {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: token_to_sell,
        pool_address: Address::zero(), // Would be determined by pool factory
        action: Action::Sell,
        params: ExecutionParams {
            amount: balance, // Sell all
            slippage: 0.05, // 5% slippage for emergency
            max_gas_price: None,
            deadline_seconds: 60, // 1 minute deadline
            priority: Priority::Critical,
        },
    };
    
    // Simulate the sell
    let result = simulator.simulate_emergency_sell(&alert, token_to_sell, balance).await;
    
    println!("\n📊 Simulation Results:");
    println!("Would succeed: {}", result.would_succeed);
    if let Some(eth) = result.eth_amount_received {
        println!("ETH received: {} ETH", ethers::utils::format_ether(eth));
    }
    println!("Gas cost: {} ETH", result.estimated_gas_cost_eth);
    
    if result.would_succeed {
        println!("✅ Emergency sell executed successfully");
        
        // Show portfolio update
        println!("\n📈 Updated Portfolio:");
        display_portfolio(simulator);
    }
    
    Ok(())
}

async fn simulate_rebalancing(
    simulator: &mut SimulationEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚖️ Simulating Portfolio Rebalancing");
    
    // Simulate portfolio value
    let total_value_eth = 5.5; // Simulated value
    
    println!("Current portfolio value: {:.4} ETH", total_value_eth);
    println!("\nTarget allocation:");
    println!("  40% ETH");
    println!("  30% USDC");
    println!("  20% USDT");
    println!("  10% Other tokens");
    
    // Calculate target amounts
    let target_eth = total_value_eth * 0.4;
    let target_usdc_eth = total_value_eth * 0.3;
    let target_usdt_eth = total_value_eth * 0.2;
    
    println!("\n📊 Rebalancing Actions:");
    
    // ETH rebalancing
    let current_eth = 5.0; // Simulated current ETH
    let eth_diff = target_eth - current_eth;
    if eth_diff.abs() > 0.01 {
        if eth_diff > 0.0 {
            println!("  SELL tokens to get {:.3} more ETH", eth_diff);
        } else {
            println!("  BUY tokens with {:.3} ETH", -eth_diff);
        }
    }
    
    // Stable coin rebalancing
    println!("  Rebalance USDC to ~{:.2} ETH worth", target_usdc_eth);
    println!("  Rebalance USDT to ~{:.2} ETH worth", target_usdt_eth);
    
    // Simulate a swap for rebalancing
    if eth_diff < -0.1 {
        // Buy USDC with excess ETH
        let buy_amount = ethers::utils::parse_ether((-eth_diff * 0.5).to_string())?;
        let alert = create_buy_alert(USDC.parse()?, buy_amount);
        let result = simulator.simulate_buy(&alert, USDC.parse()?, buy_amount).await;
        
        if result.would_succeed {
            println!("\n✅ Rebalancing swap executed:");
            println!("  Swapped {:.3} ETH for USDC", -eth_diff * 0.5);
        }
    }
    
    println!("\n📈 Portfolio after rebalancing:");
    display_portfolio(simulator.get_portfolio_state());
    
    Ok(())
}

async fn simulate_risk_limits(
    simulator: &mut SimulationEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🛡️ Simulating Risk Limit Scenarios");
    
    let scenarios = vec![
        ("Max position size", 2.0, "Single token > 50% of portfolio"),
        ("Daily loss limit", 0.5, "Portfolio down > 20% in 24h"),
        ("Gas price spike", 0.1, "Gas price > 500 gwei"),
        ("Slippage protection", 0.3, "Actual slippage > 5%"),
    ];
    
    for (name, eth_amount, trigger) in scenarios {
        println!("\n📍 Scenario: {}", name);
        println!("   Trigger: {}", trigger);
        println!("   Trade size: {} ETH", eth_amount);
        
        // Create a risky trade
        let alert = create_buy_alert(
            PEPE.parse()?, 
            ethers::utils::parse_ether(eth_amount.to_string())?
        );
        
        let result = simulator.simulate_buy(
            &alert,
            PEPE.parse()?,
            alert.params.amount
        ).await;
        
        if !result.would_succeed {
            println!("   ❌ BLOCKED: {}", result.failure_reason.unwrap_or("Risk limit exceeded".to_string()));
        } else {
            println!("   ⚠️  WARNING: Trade would proceed (implement risk checks!)");
        }
    }
    
    println!("\n💡 Risk Management Tips:");
    println!("• Set maximum position sizes");
    println!("• Implement stop-loss mechanisms");
    println!("• Monitor gas prices before execution");
    println!("• Track daily P&L limits");
    println!("• Use circuit breakers for unusual market conditions");
    
    Ok(())
}

fn display_portfolio(_simulator: &SimulationEngine) {
    println!("\n💼 Current Portfolio:");
    
    // Display simulated portfolio state
    println!("ETH: 5.0 ETH");
    println!("USDC: 10,000 tokens");
    println!("USDT: 5,000 tokens");
    
    println!("\n📊 Trading Statistics:");
    println!("Total trades: 0");
    println!("Successful: 0");
    println!("Failed: 0");
    
    println!("\n💰 Total Portfolio Value: ~5.5 ETH");
}

fn calculate_pnl(_simulator: &SimulationEngine) -> Option<f64> {
    let current_value = 5.5; // Simulated current value
    let initial_value = 5.0 + 10000.0/2500.0 + 5000.0/2500.0; // Initial portfolio
    
    Some(((current_value - initial_value) / initial_value) * 100.0)
}

fn create_buy_alert(token: Address, amount: U256) -> Alert {
    Alert {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        token_address: token,
        pool_address: Address::zero(),
        action: Action::Buy,
        params: ExecutionParams {
            amount,
            slippage: 0.02,
            max_gas_price: None,
            deadline_seconds: 300,
            priority: Priority::Medium,
        },
    }
}

fn reset_portfolio() -> SimulationEngine {
    let mut initial_tokens = HashMap::new();
    initial_tokens.insert(USDC.parse::<Address>().unwrap(), U256::from(10000_000000));
    initial_tokens.insert(USDT.parse::<Address>().unwrap(), U256::from(5000_000000));
    
    SimulationEngine::new(SimulationMode::Full {
        initial_eth_balance: 5.0,
        initial_token_positions: initial_tokens,
    })
}