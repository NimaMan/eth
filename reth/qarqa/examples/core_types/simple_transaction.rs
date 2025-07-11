//! Simple transaction example - minimal working example

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;
use chrono;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Simple Transaction Example ===\n");
    
    if let Err(e) = run_example() {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

fn run_example() -> QarqaResult<()> {
    // Create a simple ETH transfer transaction
    let transaction = Transaction {
        hash: B256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?,
        block_number: 18500000,
        from_address: Address::from_str("0x1111111111111111111111111111111111111111")?,
        to_address: Some(Address::from_str("0x2222222222222222222222222222222222222222")?),
        value: U256::from_str("1000000000000000000")?, // 1 ETH
        gas_limit: 21000,
        gas_price: U256::from_str("20000000000")?, // 20 gwei
        input_data: Vec::new(),
        status: true,
        gas_used: Some(21000),
        timestamp: Some(chrono::Utc::now()),
    };

    println!("Transaction Created:");
    println!("  Hash: {}", transaction.hash);
    println!("  From: {}", format_address(transaction.from_address));
    println!("  To: {}", format_address(transaction.to_address.unwrap()));
    println!("  Value: {} ETH", wei_to_eth(transaction.value));
    println!("  Gas: {}", transaction.gas_limit);
    
    // Create ETH movement
    let eth_movement = EthMovement {
        from: transaction.from_address,
        to: transaction.to_address.unwrap(),
        amount: transaction.value,
        movement_type: EthMovementType::Direct,
    };
    
    println!("\nETH Movement:");
    println!("  Type: {:?}", eth_movement.movement_type);
    println!("  Amount: {} ETH", wei_to_eth(eth_movement.amount));
    
    // Create fund flow
    let fund_flow = FundFlow {
        from: transaction.from_address,
        to: transaction.to_address.unwrap(),
        amount_eth: wei_to_eth(transaction.value),
        amount_tokens_usd: 0.0,
        transaction_count: 1,
        first_block: transaction.block_number,
        last_block: transaction.block_number,
        flow_type: FlowType::DirectTransfer,
    };
    
    println!("\nFund Flow:");
    println!("  ETH Amount: {}", fund_flow.amount_eth);
    println!("  Flow Type: {:?}", fund_flow.flow_type);
    
    println!("\n=== Simple transaction example completed! ===");
    
    Ok(())
}