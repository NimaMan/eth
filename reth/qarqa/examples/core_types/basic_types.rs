//! Basic types example - demonstrates core QARQA data structures

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;
use chrono;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== QARQA Core Types Example ===\n");
    
    if let Err(e) = run_example() {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

fn run_example() -> QarqaResult<()> {
    
    // 1. Create a sample transaction
    let transaction = Transaction {
        hash: B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?,
        block_number: 18500000,
        from_address: Address::from_str("0x1234567890123456789012345678901234567890")?,
        to_address: Some(Address::from_str("0x9876543210987654321098765432109876543210")?),
        value: U256::from_str("1000000000000000000")?, // 1 ETH
        gas_limit: 21000,
        gas_price: U256::from_str("20000000000")?, // 20 gwei
        input_data: Vec::new(),
        status: true,
        gas_used: Some(21000),
        timestamp: Some(chrono::Utc::now()),
    };
    
    println!("1. Transaction:");
    println!("   Hash: {:?}", transaction.hash);
    println!("   Block: {}", transaction.block_number);
    println!("   From: {:?}", transaction.from_address);
    println!("   To: {:?}", transaction.to_address);
    println!("   Value: {} ETH", wei_to_eth(transaction.value));
    println!("   Gas Used: {:?}", transaction.gas_used);
    println!();
    
    // 2. Create ETH movements
    let eth_movement = EthMovement {
        from: transaction.from_address,
        to: transaction.to_address.unwrap(),
        amount: transaction.value,
        movement_type: EthMovementType::Direct,
    };
    
    println!("2. ETH Movement:");
    println!("   From: {:?}", eth_movement.from);
    println!("   To: {:?}", eth_movement.to);
    println!("   Amount: {} ETH", wei_to_eth(eth_movement.amount));
    println!("   Type: {:?}", eth_movement.movement_type);
    println!();
    
    // 3. Create a token movement
    let usdc_address = Address::from_str("0x1111111111111111111111111111111111111111")?;
    let token_movement = TokenMovement {
        from: transaction.from_address,
        to: transaction.to_address.unwrap(),
        token_address: usdc_address,
        amount: U256::from_str("1000000")?, // 1 USDC (6 decimals)
        symbol: Some("USDC".to_string()),
        decimals: Some(6),
    };
    
    println!("3. Token Movement:");
    println!("   Token: {:?} (USDC)", token_movement.token_address);
    println!("   From: {:?}", token_movement.from);
    println!("   To: {:?}", token_movement.to);
    println!("   Amount: {} USDC", token_movement.amount.to::<u64>() as f64 / 1_000_000.0);
    println!();
    
    // 4. Create complete fund flows
    let complete_flows = CompleteFundFlows {
        transaction_hash: transaction.hash,
        block_number: transaction.block_number,
        timestamp: chrono::Utc::now(),
        eth_movements: vec![eth_movement],
        token_movements: vec![token_movement],
        gas_used: transaction.gas_used.unwrap_or(21000),
        status: transaction.status,
    };
    
    println!("4. Complete Fund Flows:");
    println!("   Transaction: {:?}", complete_flows.transaction_hash);
    println!("   Block: {}", complete_flows.block_number);
    println!("   ETH movements: {}", complete_flows.eth_movements.len());
    println!("   Token movements: {}", complete_flows.token_movements.len());
    println!("   Gas used: {}", complete_flows.gas_used);
    println!("   Status: {}", complete_flows.status);
    println!();
    
    // 5. Create a fund flow
    let fund_flow = FundFlow {
        from: transaction.from_address,
        to: transaction.to_address.unwrap(),
        amount_eth: wei_to_eth(transaction.value),
        amount_tokens_usd: 1000.0, // $1000 worth of tokens
        transaction_count: 1,
        first_block: transaction.block_number,
        last_block: transaction.block_number,
        flow_type: FlowType::DirectTransfer,
    };
    
    println!("5. Fund Flow:");
    println!("   From: {:?}", fund_flow.from);
    println!("   To: {:?}", fund_flow.to);
    println!("   ETH Amount: {}", fund_flow.amount_eth);
    println!("   USD Value: ${}", fund_flow.amount_tokens_usd);
    println!("   TX Count: {}", fund_flow.transaction_count);
    println!("   Flow Type: {:?}", fund_flow.flow_type);
    println!();
    
    // 6. Demonstrate utility functions
    println!("6. Utility Functions:");
    
    // Address formatting
    let formatted_addr = format_address(transaction.from_address);
    println!("   Formatted address: {}", formatted_addr);
    
    // Wei/ETH conversion
    let wei_amount = eth_to_wei(2.5);
    println!("   2.5 ETH in wei: {}", wei_amount);
    println!("   Back to ETH: {}", wei_to_eth(wei_amount));
    
    // Parse address
    let parsed_addr = parse_address("0x1234567890123456789012345678901234567890")?;
    println!("   Parsed address: {:?}", parsed_addr);
    
    // Moving average
    let prices = vec![100.0, 105.0, 103.0, 108.0, 110.0];
    let moving_avg = moving_average(&prices, 3);
    println!("   Moving average (window=3): {:?}", moving_avg);
    
    // Percentage change
    let change = percentage_change(100.0, 105.0);
    println!("   Percentage change (100->105): {:.2}%", change);
    
    println!("\n=== Example completed successfully! ===");
    
    Ok(())
}