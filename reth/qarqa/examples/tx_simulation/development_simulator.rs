//! Development simulator example - demonstrates transaction simulation without REVM

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_tx_simulation::{DevelopmentTransactionSimulator, TransactionSimulator};
use alloy_primitives::{Address, U256, B256};
use std::str::FromStr;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Development Transaction Simulator Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting development transaction simulator example");
    
    // Create and initialize simulator
    let mut simulator = DevelopmentTransactionSimulator::new();
    simulator.initialize().await?;
    
    println!("✓ Development simulator initialized");
    println!();
    
    // Test different types of transactions
    let test_transactions = create_test_transactions()?;
    
    for (i, (name, transaction)) in test_transactions.iter().enumerate() {
        println!("{}. Simulating {}:", i + 1, name);
        print_transaction_summary(transaction);
        
        match simulator.simulate_transaction(transaction).await {
            Ok(fund_flows) => {
                println!("   ✓ Simulation successful");
                analyze_fund_flows(&fund_flows);
            }
            Err(e) => {
                println!("   ✗ Simulation failed: {}", e);
            }
        }
        println!();
    }
    
    // Batch simulation test
    println!("5. Testing batch simulation:");
    let transactions: Vec<_> = test_transactions.iter().map(|(_, tx)| tx.clone()).collect();
    
    match simulator.simulate_transactions(&transactions).await {
        Ok(results) => {
            println!("   ✓ Batch simulation successful");
            println!("   Results: {} out of {} transactions simulated", results.len(), transactions.len());
            
            // Aggregate analysis
            aggregate_analysis(&results);
        }
        Err(e) => {
            println!("   ✗ Batch simulation failed: {}", e);
        }
    }
    
    // Demonstrate simulation concepts
    demonstrate_simulation_concepts();
    
    println!("\n=== Development transaction simulator completed! ===");
    
    Ok(())
}

fn create_test_transactions() -> QarqaResult<Vec<(String, Transaction)>> {
    let mut transactions = Vec::new();
    
    // 1. Simple ETH transfer
    transactions.push((
        "Simple ETH Transfer".to_string(),
        Transaction {
            hash: B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")?,
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
        }
    ));
    
    // 2. Contract interaction with ETH
    transactions.push((
        "Contract Call with ETH".to_string(),
        Transaction {
            hash: B256::from_str("0x2222222222222222222222222222222222222222222222222222222222222222")?,
            block_number: 18500001,
            from_address: Address::from_str("0x3333333333333333333333333333333333333333")?,
            to_address: Some(Address::from_str("0x4444444444444444444444444444444444444444")?),
            value: U256::from_str("500000000000000000")?, // 0.5 ETH
            gas_limit: 100000,
            gas_price: U256::from_str("25000000000")?, // 25 gwei
            input_data: vec![0xa9, 0x05, 0x9c, 0xbb, 0x00, 0x00], // Simulated function call
            status: true,
            gas_used: Some(75000),
            timestamp: Some(chrono::Utc::now()),
        }
    ));
    
    // 3. Zero value contract call
    transactions.push((
        "Zero Value Contract Call".to_string(),
        Transaction {
            hash: B256::from_str("0x3333333333333333333333333333333333333333333333333333333333333333")?,
            block_number: 18500002,
            from_address: Address::from_str("0x5555555555555555555555555555555555555555")?,
            to_address: Some(Address::from_str("0x6666666666666666666666666666666666666666")?),
            value: U256::ZERO,
            gas_limit: 200000,
            gas_price: U256::from_str("30000000000")?, // 30 gwei
            input_data: vec![0x18, 0x16, 0x0d, 0xdd], // Different function selector
            status: true,
            gas_used: Some(150000),
            timestamp: Some(chrono::Utc::now()),
        }
    ));
    
    // 4. Failed transaction
    transactions.push((
        "Failed Transaction".to_string(),
        Transaction {
            hash: B256::from_str("0x4444444444444444444444444444444444444444444444444444444444444444")?,
            block_number: 18500003,
            from_address: Address::from_str("0x7777777777777777777777777777777777777777")?,
            to_address: Some(Address::from_str("0x8888888888888888888888888888888888888888")?),
            value: U256::from_str("2000000000000000000")?, // 2 ETH
            gas_limit: 21000,
            gas_price: U256::from_str("15000000000")?, // 15 gwei
            input_data: Vec::new(),
            status: false, // Failed transaction
            gas_used: Some(21000), // Used all gas
            timestamp: Some(chrono::Utc::now()),
        }
    ));
    
    Ok(transactions)
}

fn print_transaction_summary(tx: &Transaction) {
    println!("   Hash: {}", format_hash(&tx.hash));
    println!("   From: {} → To: {}", 
             format_address(tx.from_address),
             tx.to_address.map_or("None".to_string(), |addr| format_address(addr)));
    println!("   Value: {} ETH", wei_to_eth(tx.value));
    println!("   Gas: {} / {} ({:.1}%)", 
             tx.gas_used.unwrap_or(0),
             tx.gas_limit,
             (tx.gas_used.unwrap_or(0) as f64 / tx.gas_limit as f64) * 100.0);
    println!("   Status: {}", if tx.status { "Success" } else { "Failed" });
    println!("   Input: {} bytes", tx.input_data.len());
}

fn analyze_fund_flows(fund_flows: &CompleteFundFlows) {
    println!("   Fund Flow Analysis:");
    println!("     Block: {}", fund_flows.block_number);
    println!("     ETH movements: {}", fund_flows.eth_movements.len());
    println!("     Token movements: {}", fund_flows.token_movements.len());
    println!("     Gas used: {}", fund_flows.gas_used);
    println!("     Status: {}", fund_flows.status);
    
    if !fund_flows.eth_movements.is_empty() {
        println!("     ETH Movement Details:");
        for (i, movement) in fund_flows.eth_movements.iter().enumerate() {
            println!("       {}. {} → {}: {} ETH ({})",
                     i + 1,
                     format_address(movement.from),
                     format_address(movement.to),
                     wei_to_eth(movement.amount),
                     format!("{:?}", movement.movement_type));
        }
    }
    
    if !fund_flows.token_movements.is_empty() {
        println!("     Token Movement Details:");
        for (i, movement) in fund_flows.token_movements.iter().enumerate() {
            println!("       {}. {} → {}: {} tokens ({})",
                     i + 1,
                     format_address(movement.from),
                     format_address(movement.to),
                     movement.amount,
                     format_address(movement.token_address));
        }
    }
}

fn aggregate_analysis(results: &[CompleteFundFlows]) {
    println!("   Aggregate Analysis:");
    
    let total_eth_movements: usize = results.iter().map(|r| r.eth_movements.len()).sum();
    let total_token_movements: usize = results.iter().map(|r| r.token_movements.len()).sum();
    let total_gas: u64 = results.iter().map(|r| r.gas_used).sum();
    let successful_count = results.iter().filter(|r| r.status).count();
    
    println!("     Total ETH movements: {}", total_eth_movements);
    println!("     Total token movements: {}", total_token_movements);
    println!("     Total gas used: {}", total_gas);
    println!("     Success rate: {}/{} ({:.1}%)", 
             successful_count, 
             results.len(),
             (successful_count as f64 / results.len() as f64) * 100.0);
    
    // Calculate total ETH volume
    let total_eth_volume: f64 = results.iter()
        .flat_map(|r| &r.eth_movements)
        .filter(|m| matches!(m.movement_type, EthMovementType::Direct | EthMovementType::Internal))
        .map(|m| wei_to_eth(m.amount))
        .sum();
    
    println!("     Total ETH volume: {} ETH", total_eth_volume);
}

fn demonstrate_simulation_concepts() {
    println!("Transaction Simulation Concepts:");
    println!();
    
    println!("Development Simulator Features:");
    println!("  • Analyzes transaction structure without full EVM execution");
    println!("  • Extracts basic fund flows from transaction data");
    println!("  • Simulates gas payments to miners");
    println!("  • No mock data - uses actual transaction parameters");
    println!("  • Fast and lightweight for testing");
    println!();
    
    println!("Fund Flow Extraction:");
    println!("  1. Direct ETH transfers (transaction.value > 0)");
    println!("  2. Gas payments (gas_used * gas_price → miners)");
    println!("  3. Input data analysis for contract calls");
    println!("  4. Token transfer detection (future enhancement)");
    println!();
    
    println!("Limitations vs. Full REVM:");
    println!("  • Cannot trace internal contract calls");
    println!("  • No state changes simulation");
    println!("  • Limited token transfer detection");
    println!("  • No EVM execution errors");
    println!();
    
    println!("Use Cases:");
    println!("  • Component testing and development");
    println!("  • Basic fund flow analysis");
    println!("  • Transaction structure validation");
    println!("  • Performance testing (much faster than REVM)");
}

fn format_hash(hash: &B256) -> String {
    format!("0x{}...{}", 
            &hash.to_string()[2..8], 
            &hash.to_string()[hash.to_string().len()-6..])
}