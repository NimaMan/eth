//! State changes example - demonstrates state change analysis and tracking

use qarqa_core_types::*;
use qarqa_tx_simulation::*;
use alloy_primitives::Address;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== State Changes Analysis Example ===\n");
    
    // Create state change analyzer
    let analyzer = StateChangeAnalyzer::new()
        .with_significance_threshold(0.1) // 0.1 ETH minimum
        .with_top_movers_count(10);
    
    // Demonstrate different types of state change analysis
    demonstrate_simple_transfer_analysis(&analyzer).await?;
    demonstrate_complex_defi_analysis(&analyzer).await?;
    demonstrate_batch_analysis(&analyzer).await?;
    demonstrate_time_series_analysis(&analyzer).await?;
    
    println!("\n=== State Changes Analysis Complete ===");
    Ok(())
}

async fn demonstrate_simple_transfer_analysis(
    analyzer: &StateChangeAnalyzer
) -> QarqaResult<()> {
    println!("1. Simple Transfer State Changes:");
    
    // Create a simple ETH transfer transaction
    let transaction = create_simple_transfer_transaction();
    
    println!("  Analyzing transaction: {}", transaction.hash);
    display_transaction_summary(&transaction);
    
    // Analyze state changes
    let state_changes = analyzer
        .analyze_transaction(&transaction)
        .await?;
    
    println!("  State Changes Detected:");
    display_state_changes(&state_changes);
    
    // Verify conservation (total in should equal total out)
    verify_conservation(&state_changes);
    
    println!();
    Ok(())
}

async fn demonstrate_complex_defi_analysis(
    analyzer: &StateChangeAnalyzer
) -> QarqaResult<()> {
    println!("2. Complex DeFi Transaction Analysis:");
    
    // Create a complex DeFi transaction with multiple tokens
    let transaction = create_complex_defi_transaction();
    
    println!("  Analyzing DeFi transaction: {}", transaction.hash);
    display_transaction_summary(&transaction);
    
    // Analyze state changes
    let state_changes = analyzer
        .analyze_transaction(&transaction)
        .await?;
    
    println!("  State Changes Detected:");
    display_state_changes(&state_changes);
    
    // Identify significant movers
    identify_significant_movers(&state_changes);
    
    // Analyze token flow patterns
    analyze_token_flows(&state_changes);
    
    println!();
    Ok(())
}

async fn demonstrate_batch_analysis(
    analyzer: &StateChangeAnalyzer
) -> QarqaResult<()> {
    println!("3. Batch Transaction Analysis:");
    
    // Create multiple transactions for batch analysis
    let transactions = create_batch_transactions();
    
    println!("  Analyzing {} transactions in batch", transactions.len());
    
    let mut all_state_changes = Vec::new();
    let mut total_gas_used = 0u64;
    
    for (i, transaction) in transactions.iter().enumerate() {
        println!("    Transaction {}: {}", i + 1, transaction.hash);
        
        let state_changes = analyzer
            .analyze_transaction(transaction)
            .await?;
        
        total_gas_used += transaction.gas_used;
        all_state_changes.push(state_changes);
    }
    
    // Aggregate analysis
    println!("  Batch Analysis Results:");
    println!("    • Total Gas Used: {} gas", total_gas_used);
    println!("    • Average Gas per Transaction: {} gas", 
             total_gas_used / transactions.len() as u64);
    
    // Find most active addresses across all transactions
    let most_active = find_most_active_addresses(&all_state_changes);
    println!("    • Most Active Addresses:");
    for (i, (address, tx_count)) in most_active.iter().take(3).enumerate() {
        println!("      {}. {}: {} transactions", 
                i + 1, 
                format_address(address), 
                tx_count);
    }
    
    println!();
    Ok(())
}

async fn demonstrate_time_series_analysis(
    analyzer: &StateChangeAnalyzer
) -> QarqaResult<()> {
    println!("4. Time Series State Change Analysis:");
    
    // Create transactions over time
    let time_series_transactions = create_time_series_transactions();
    
    println!("  Analyzing {} transactions over time", time_series_transactions.len());
    
    let mut address_balances: HashMap<Address, f64> = HashMap::new();
    
    for (i, transaction) in time_series_transactions.iter().enumerate() {
        let state_changes = analyzer
            .analyze_transaction(transaction)
            .await?;
        
        // Update running balances
        for change in &state_changes.changes {
            *address_balances.entry(change.address).or_insert(0.0) += change.eth_change;
        }
        
        if i % 5 == 0 { // Print every 5th transaction
            println!("    After transaction {}: Top balances:", i + 1);
            let mut sorted_balances: Vec<_> = address_balances.iter().collect();
            sorted_balances.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            
            for (j, (address, balance)) in sorted_balances.iter().take(3).enumerate() {
                println!("      {}. {}: {} ETH", 
                        j + 1, 
                        format_address(address), 
                        balance);
            }
        }
    }
    
    // Final analysis
    println!("  Final State Summary:");
    let mut final_balances: Vec<_> = address_balances.iter().collect();
    final_balances.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    
    let total_eth: f64 = final_balances.iter().map(|(_, balance)| *balance).sum();
    println!("    • Total ETH in system: {} ETH", total_eth);
    println!("    • Net ETH change: {} ETH (should be ~0 excluding gas)", total_eth);
    
    println!();
    Ok(())
}

// Helper functions for creating sample transactions

fn create_simple_transfer_transaction() -> Transaction {
    Transaction {
        hash: [1u8; 32].into(),
        from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
            .parse()
            .unwrap(),
        to_address: Some("0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()),
        value: eth_to_wei(2.5),
        gas_price: 20_000_000_000,
        gas_limit: 21_000,
        gas_used: 21_000,
        block_number: 18_500_000,
        transaction_index: 1,
        status: TransactionStatus::Success,
        internal_transfers: Vec::new(),
        token_transfers: Vec::new(),
        timestamp: 1698765432,
    }
}

fn create_complex_defi_transaction() -> Transaction {
    Transaction {
        hash: [2u8; 32].into(),
        from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
            .parse()
            .unwrap(),
        to_address: Some("0xE592427A0AEce92De3Edee1F18E0157C05861564"
            .parse()
            .unwrap()), // Uniswap V3 SwapRouter
        value: 0,
        gas_price: 30_000_000_000,
        gas_limit: 300_000,
        gas_used: 287_456,
        block_number: 18_500_001,
        transaction_index: 5,
        status: TransactionStatus::Success,
        internal_transfers: vec![
            EthMovement {
                from_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(), // WETH
                to_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
                    .parse()
                    .unwrap(),
                amount: eth_to_wei(1.5),
                movement_type: MovementType::Call,
            }
        ],
        token_transfers: vec![
            TokenMovement {
                token_address: "0xA0b86a33E6441b8Ec1d04A13b29c50E5C3F9F15"
                    .parse()
                    .unwrap(), // USDC
                from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
                    .parse()
                    .unwrap(),
                to_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
                    .parse()
                    .unwrap(), // USDC/ETH pool
                amount: 3000_000_000u128.into(), // 3000 USDC
                token_symbol: Some("USDC".to_string()),
                token_decimals: Some(6),
            },
            TokenMovement {
                token_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(), // WETH
                from_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
                    .parse()
                    .unwrap(),
                to_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
                    .parse()
                    .unwrap(),
                amount: eth_to_wei(1.5).into(),
                token_symbol: Some("WETH".to_string()),
                token_decimals: Some(18),
            }
        ],
        timestamp: 1698765500,
    }
}

fn create_batch_transactions() -> Vec<Transaction> {
    let mut transactions = Vec::new();
    
    // Create 5 different types of transactions
    for i in 0..5 {
        let mut hash = [0u8; 32];
        hash[0] = i + 10;
        
        let transaction = Transaction {
            hash: hash.into(),
            from_address: generate_address(i),
            to_address: Some(generate_address(i + 1)),
            value: eth_to_wei((i as f64 + 1.0) * 0.5),
            gas_price: 25_000_000_000,
            gas_limit: 50_000 + i * 10_000,
            gas_used: 45_000 + i * 8_000,
            block_number: 18_500_010 + i,
            transaction_index: i as u32,
            status: TransactionStatus::Success,
            internal_transfers: if i % 2 == 0 {
                vec![
                    EthMovement {
                        from_address: generate_address(i),
                        to_address: generate_address(i + 2),
                        amount: eth_to_wei(0.1),
                        movement_type: MovementType::Call,
                    }
                ]
            } else {
                Vec::new()
            },
            token_transfers: Vec::new(),
            timestamp: 1698765600 + (i * 300) as u64,
        };
        
        transactions.push(transaction);
    }
    
    transactions
}

fn create_time_series_transactions() -> Vec<Transaction> {
    let mut transactions = Vec::new();
    
    // Create 20 transactions simulating trading activity over time
    for i in 0..20 {
        let mut hash = [0u8; 32];
        hash[0] = i + 50;
        
        let from_index = i % 3; // Rotate between 3 main addresses
        let to_index = (i + 1) % 3;
        
        let transaction = Transaction {
            hash: hash.into(),
            from_address: generate_address(from_index),
            to_address: Some(generate_address(to_index)),
            value: eth_to_wei(0.1 + (i as f64 * 0.05)), // Increasing amounts
            gas_price: 20_000_000_000,
            gas_limit: 21_000,
            gas_used: 21_000,
            block_number: 18_500_100 + i,
            transaction_index: i as u32,
            status: TransactionStatus::Success,
            internal_transfers: Vec::new(),
            token_transfers: Vec::new(),
            timestamp: 1698800000 + (i * 600) as u64, // Every 10 minutes
        };
        
        transactions.push(transaction);
    }
    
    transactions
}

fn generate_address(seed: u64) -> Address {
    let mut bytes = [0u8; 20];
    bytes[0..8].copy_from_slice(&seed.to_be_bytes());
    Address::from(bytes)
}

fn display_transaction_summary(transaction: &Transaction) {
    println!("    • From: {}", format_address(&transaction.from_address));
    if let Some(to) = transaction.to_address {
        println!("    • To: {}", format_address(&to));
    }
    println!("    • Value: {} ETH", wei_to_eth(transaction.value));
    println!("    • Gas Used: {}", transaction.gas_used);
}

fn display_state_changes(state_changes: &StateChangeAnalysisResult) {
    for (i, change) in state_changes.changes.iter().enumerate() {
        println!("    {}. {}: {} ETH", 
                i + 1,
                format_address(&change.address),
                change.eth_change);
        
        if !change.token_changes.is_empty() {
            for token_change in &change.token_changes {
                println!("       + {} {}", 
                        token_change.amount_change,
                        token_change.symbol.as_deref().unwrap_or("???"));
            }
        }
    }
}

fn verify_conservation(state_changes: &StateChangeAnalysisResult) {
    let total_eth_change: f64 = state_changes.changes
        .iter()
        .map(|change| change.eth_change)
        .sum();
    
    println!("  Conservation Check:");
    if total_eth_change.abs() < 0.001 {
        println!("    ✓ ETH conservation verified (total change: {} ETH)", total_eth_change);
    } else {
        println!("    ⚠ ETH not conserved (total change: {} ETH)", total_eth_change);
        println!("      This may include gas fees or be expected for certain transaction types");
    }
}

fn identify_significant_movers(state_changes: &StateChangeAnalysisResult) {
    println!("  Significant Movers:");
    
    let mut changes_with_index: Vec<_> = state_changes.changes
        .iter()
        .enumerate()
        .collect();
    
    // Sort by absolute ETH change
    changes_with_index.sort_by(|a, b| {
        b.1.eth_change.abs().partial_cmp(&a.1.eth_change.abs()).unwrap()
    });
    
    for (i, (_, change)) in changes_with_index.iter().take(3).enumerate() {
        let direction = if change.eth_change > 0.0 { "gained" } else { "lost" };
        println!("    {}. {}: {} {} ETH", 
                i + 1,
                format_address(&change.address),
                direction,
                change.eth_change.abs());
    }
}

fn analyze_token_flows(state_changes: &StateChangeAnalysisResult) {
    println!("  Token Flow Analysis:");
    
    let mut token_summary: HashMap<String, f64> = HashMap::new();
    
    for change in &state_changes.changes {
        for token_change in &change.token_changes {
            if let Some(symbol) = &token_change.symbol {
                *token_summary.entry(symbol.clone()).or_insert(0.0) += token_change.amount_change;
            }
        }
    }
    
    if token_summary.is_empty() {
        println!("    • No token transfers detected");
    } else {
        for (symbol, total_change) in token_summary {
            println!("    • {}: {} total net change", symbol, total_change);
        }
    }
}

fn find_most_active_addresses(
    all_state_changes: &[StateChangeAnalysisResult]
) -> Vec<(Address, usize)> {
    let mut address_counts: HashMap<Address, usize> = HashMap::new();
    
    for state_changes in all_state_changes {
        for change in &state_changes.changes {
            *address_counts.entry(change.address).or_insert(0) += 1;
        }
    }
    
    let mut sorted_addresses: Vec<_> = address_counts.into_iter().collect();
    sorted_addresses.sort_by(|a, b| b.1.cmp(&a.1));
    
    sorted_addresses
}

fn format_address(address: &Address) -> String {
    let addr_str = format!("{:?}", address);
    if addr_str.len() >= 10 {
        format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..])
    } else {
        addr_str
    }
}