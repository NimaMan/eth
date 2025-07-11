//! Transaction parsing example - demonstrates working with blockchain data

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;
use chrono;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Transaction Parsing Example ===\n");
    
    if let Err(e) = run_example() {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

fn run_example() -> QarqaResult<()> {
    
    // Simulate parsing different types of transactions
    
    // 1. Simple ETH transfer
    let eth_transfer = create_eth_transfer()?;
    print_transaction_summary(&eth_transfer, "Simple ETH Transfer");
    
    // 2. Contract interaction (no ETH transfer)
    let contract_call = create_contract_call()?;
    print_transaction_summary(&contract_call, "Contract Call");
    
    // 3. Token transfer
    let token_transfer = create_token_transfer()?;
    print_transaction_summary(&token_transfer, "Token Transfer");
    
    // 4. Complex transaction with multiple operations
    let complex_tx = create_complex_transaction()?;
    print_transaction_summary(&complex_tx, "Complex Transaction");
    
    // 5. Demonstrate transaction participants
    // demonstrate_participants()?;
    
    println!("\n=== Transaction parsing completed! ===");
    
    Ok(())
}

fn create_eth_transfer() -> QarqaResult<Transaction> {
    Ok(Transaction {
        hash: B256::from_str("0xabc123def456789abc123def456789abc123def456789abc123def456789abc12")?,
        block_number: 18500001,
        from_address: Address::from_str("0x1234567890123456789012345678901234567890")?,
        to_address: Some(Address::from_str("0x9876543210987654321098765432109876543210")?),
        value: U256::from_str("2000000000000000000")?, // 2 ETH
        gas_limit: 21000,
        gas_price: U256::from_str("25000000000")?, // 25 gwei
        input_data: Vec::new(),
        status: true,
        gas_used: Some(21000),
        timestamp: Some(chrono::Utc::now()),
    })
}

fn create_contract_call() -> QarqaResult<Transaction> {
    // Simulate a contract call with input data
    let input_data = vec![
        0xa9, 0x05, 0x9c, 0xbb, // function selector for transfer(address,uint256)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
        0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, // 20-byte address
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d, 0xe0, 0xb6, 0xb3, 0xa7, 0x64, 0x00, // amount (1000 * 10^18)
    ];
    
    Ok(Transaction {
        hash: B256::from_str("0xdef456789abc123def456789abc123def456789abc123def456789abc123def45")?,
        block_number: 18500002,
        from_address: Address::from_str("0x2345678901234567890123456789012345678901")?,
        to_address: Some(Address::from_str("0x2222222222222222222222222222222222222222")?), // Token contract
        value: U256::ZERO, // No ETH transfer
        gas_limit: 65000,
        gas_price: U256::from_str("30000000000")?, // 30 gwei
        input_data,
        status: true,
        gas_used: Some(52000),
        timestamp: Some(chrono::Utc::now()),
    })
}

fn create_token_transfer() -> QarqaResult<Transaction> {
    Ok(Transaction {
        hash: B256::from_str("0x789abc123def456789abc123def456789abc123def456789abc123def45678901")?,
        block_number: 18500003,
        from_address: Address::from_str("0x3456789012345678901234567890123456789012")?,
        to_address: Some(Address::from_str("0x3333333333333333333333333333333333333333")?), // Token contract
        value: U256::ZERO,
        gas_limit: 55000,
        gas_price: U256::from_str("22000000000")?, // 22 gwei
        input_data: vec![0xa9, 0x05, 0x9c, 0xbb], // Simplified input data
        status: true,
        gas_used: Some(45000),
        timestamp: Some(chrono::Utc::now()),
    })
}

fn create_complex_transaction() -> QarqaResult<Transaction> {
    Ok(Transaction {
        hash: B256::from_str("0x456789abc123def456789abc123def456789abc123def456789abc123def456701")?,
        block_number: 18500004,
        from_address: Address::from_str("0x4567890123456789012345678901234567890123")?,
        to_address: Some(Address::from_str("0x4444444444444444444444444444444444444444")?), // DEX Router
        value: U256::from_str("500000000000000000")?, // 0.5 ETH
        gas_limit: 200000,
        gas_price: U256::from_str("35000000000")?, // 35 gwei
        input_data: vec![0x38, 0xed, 0x17, 0x39], // swapExactETHForTokens function selector
        status: true,
        gas_used: Some(150000),
        timestamp: Some(chrono::Utc::now()),
    })
}

fn print_transaction_summary(tx: &Transaction, title: &str) {
    println!("{}:", title);
    println!("  Hash: {}", format_hash(&tx.hash));
    println!("  Block: {}", tx.block_number);
    println!("  From: {}", format_address(tx.from_address));
    println!("  To: {}", tx.to_address.map_or("None".to_string(), |addr| format_address(addr)));
    println!("  Value: {} ETH", wei_to_eth(tx.value));
    println!("  Gas: {} / {} ({}%)", 
             tx.gas_used.unwrap_or(0), 
             tx.gas_limit, 
             (tx.gas_used.unwrap_or(0) * 100 / tx.gas_limit));
    println!("  Gas Price: {} gwei", tx.gas_price.to::<u64>() / 1_000_000_000);
    println!("  Input Data: {} bytes", tx.input_data.len());
    println!("  Status: {}", if tx.status { "Success" } else { "Failed" });
    
    // Analyze transaction type
    let tx_type = analyze_transaction_type(tx);
    println!("  Type: {}", tx_type);
    println!();
}

fn analyze_transaction_type(tx: &Transaction) -> String {
    if tx.value > U256::ZERO && tx.input_data.is_empty() {
        "Simple ETH Transfer".to_string()
    } else if tx.value == U256::ZERO && !tx.input_data.is_empty() {
        "Contract Call (No ETH)".to_string()
    } else if tx.value > U256::ZERO && !tx.input_data.is_empty() {
        "Contract Call with ETH".to_string()
    } else {
        "Unknown/Empty Transaction".to_string()
    }
}

fn demonstrate_participants() -> QarqaResult<()> {
    println!("Transaction Participants Example:");
    
    // Create sample participants
    let participants = vec![
        TransactionParticipant {
            address: Address::from_str("0x1234567890123456789012345678901234567890")?,
            transaction_hash: B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
            direction: ParticipantDirection::Out,
            block_number: 18500001,
            value_change: Some(-1000000000000000000), // -1 ETH sent
            token_transfers: None,
        },
        TransactionParticipant {
            address: Address::from_str("0x9876543210987654321098765432109876543210")?,
            transaction_hash: B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
            direction: ParticipantDirection::In,
            block_number: 18500001,
            value_change: Some(1000000000000000000), // +1 ETH received
            token_transfers: None,
        },
        TransactionParticipant {
            address: Address::from_str("0x1111111111111111111111111111111111111111")?, // Token contract
            transaction_hash: B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
            direction: ParticipantDirection::Both,
            block_number: 18500001,
            value_change: None,
            token_transfers: Some(serde_json::json!({"transfers": 1})),
        },
    ];
    
    for (i, participant) in participants.iter().enumerate() {
        println!("  Participant {}: {}", 
                 i + 1, 
                 format_address(participant.address));
        println!("    Direction: {:?}", participant.direction);
        println!("    Block: {}", participant.block_number);
    }
    println!();
    
    Ok(())
}

fn format_hash(hash: &B256) -> String {
    format!("0x{}...{}", 
            &hash.to_string()[2..8], 
            &hash.to_string()[hash.to_string().len()-6..])
}