/// View Function Call Example
/// 
/// This example demonstrates how to call view/pure functions on smart contracts
/// without creating transactions. View functions are read-only contract methods
/// that return data without modifying blockchain state.
///
/// WHAT IT DEMONSTRATES:
/// 1. Calling ERC20 view functions (totalSupply, balanceOf, decimals, symbol, name)
/// 2. Encoding function calls with parameters
/// 3. Decoding different return types (uint256, uint8, string)
/// 4. Working with popular tokens (WETH, USDC, USDT)
///
/// USE CASES:
/// - Reading token balances
/// - Getting token metadata
/// - Checking contract state
/// - Any read-only contract interaction
///
/// OUTPUT:
/// Shows token information for WETH, USDC, and USDT including:
/// - Total supply
/// - Decimals
/// - Symbol and name
/// - Specific address balances

use tx_simulator::TxSimulator;
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use hex;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n📖 View Function Call Demonstration");
    println!("====================================\n");
    
    // Initialize simulator
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ Simulator initialized");
    
    // Token addresses
    let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
    let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
    let usdt: Address = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse()?;
    
    // Test address to check balances for (Binance hot wallet)
    let test_address: Address = "0xF977814e90dA44bFA03b6295A0616a897441aceC".parse()?;
    
    println!("\n=== WETH (Wrapped Ether) ===");
    check_token_info(&simulator, weth, Some(test_address)).await?;
    
    println!("\n=== USDC (USD Coin) ===");
    check_token_info(&simulator, usdc, Some(test_address)).await?;
    
    println!("\n=== USDT (Tether) ===");
    check_token_info(&simulator, usdt, Some(test_address)).await?;
    
    println!("\n✅ View function calls completed successfully!");
    println!("This demonstrates read-only contract interactions without transactions.");
    
    Ok(())
}

async fn check_token_info(
    simulator: &TxSimulator,
    token: Address,
    check_balance_for: Option<Address>,
) -> Result<()> {
    // totalSupply() - 0x18160ddd
    let total_supply_data = Bytes::from(hex::decode("18160ddd")?);
    let result = simulator.simulate_view_function(token, total_supply_data, None).await?;
    
    if result.success {
        let total_supply = result.decode_uint256();
        println!("Total Supply (raw): {}", total_supply);
    }
    
    // decimals() - 0x313ce567
    let decimals_data = Bytes::from(hex::decode("313ce567")?);
    let result = simulator.simulate_view_function(token, decimals_data, None).await?;
    
    let decimals = if result.success {
        let d = result.decode_uint8();
        println!("Decimals: {}", d);
        d
    } else {
        18 // Default for most tokens
    };
    
    // symbol() - 0x95d89b41
    let symbol_data = Bytes::from(hex::decode("95d89b41")?);
    let result = simulator.simulate_view_function(token, symbol_data, None).await?;
    
    if result.success {
        let symbol = result.decode_string();
        println!("Symbol: {}", symbol);
    }
    
    // name() - 0x06fdde03
    let name_data = Bytes::from(hex::decode("06fdde03")?);
    let result = simulator.simulate_view_function(token, name_data, None).await?;
    
    if result.success {
        let name = result.decode_string();
        println!("Name: {}", name);
    }
    
    // Check balance for specific address if provided
    if let Some(address) = check_balance_for {
        // balanceOf(address) - 0x70a08231 + address
        let selector = hex::decode("70a08231")?;
        let balance_data = tx_simulator::contract_method_simulator::encode_view_function_with_address(
            selector.try_into().unwrap(),
            address,
        );
        
        let result = simulator.simulate_view_function(token, balance_data, None).await?;
        
        if result.success {
            let balance = result.decode_uint256();
            let human_readable = format_with_decimals(balance, decimals);
            println!("Balance of {}: {} ({})", 
                format!("{:?}", address).split_at(10).0,
                balance,
                human_readable
            );
        }
    }
    
    Ok(())
}

fn format_with_decimals(value: U256, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }
    
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = value / divisor;
    let fraction = value % divisor;
    
    if fraction == U256::ZERO {
        whole.to_string()
    } else {
        let fraction_str = format!("{:0>width$}", fraction, width = decimals as usize);
        let trimmed = fraction_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}