/// Transaction Graph Building Example
/// 
/// Demonstrates how to use transaction queries to build a graph of
/// fund flows, similar to what fundflownetwork does.

use reth_chain_query::postgres_db::{PostgresQuery, queries};
use eyre::Result;
use std::env;
use std::collections::{HashMap, HashSet};

#[tokio::main]
async fn main() -> Result<()> {
    // Get database URL from environment or use default
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    println!("Connecting to PostgreSQL database...");
    let pg_query = PostgresQuery::new(&database_url).await?;
    
    // Example seed address (you can change this)
    let seed_address = env::var("SEED_ADDRESS")
        .unwrap_or_else(|_| "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0".to_string());
    
    println!("\n=== Building Transaction Graph from {} ===", seed_address);
    
    // Check if address exists
    let address_id = queries::transactions::get_address_id(pg_query.db(), &seed_address).await?;
    
    match address_id {
        Some(id) => {
            println!("✓ Found address with ID: {}", id);
            
            // Count total transactions
            let tx_count = queries::transactions::count_address_transactions(
                pg_query.db(),
                &seed_address,
            ).await?;
            println!("  Total transactions: {}", tx_count);
        },
        None => {
            println!("✗ Address not found in database");
            println!("  Try with an address that has transaction history");
            return Ok(());
        }
    }
    
    // Get transactions for the seed address
    println!("\n=== Fetching Transactions ===");
    let transactions = queries::transactions::get_address_transactions(
        pg_query.db(),
        &seed_address,
        100,  // limit
        None, // no max_block filter
    ).await?;
    
    println!("Found {} transactions", transactions.len());
    
    // Build graph structure
    let mut graph_edges: HashMap<String, HashSet<String>> = HashMap::new();
    let mut edge_count = 0;
    
    for tx in &transactions {
        // Get all participants in this transaction
        let participants = queries::transactions::get_tx_participants(
            pg_query.db(),
            &tx.tx_hash,
        ).await?;
        
        // Create edges between seed address and all participants
        for participant in participants {
            if participant != seed_address {
                graph_edges
                    .entry(seed_address.clone())
                    .or_insert_with(HashSet::new)
                    .insert(participant.clone());
                
                edge_count += 1;
            }
        }
    }
    
    println!("\n=== Graph Statistics ===");
    println!("Nodes: {}", graph_edges.len() + graph_edges.values().flat_map(|v| v.iter()).count());
    println!("Edges: {}", edge_count);
    println!("Direct connections from seed: {}", 
        graph_edges.get(&seed_address).map(|s| s.len()).unwrap_or(0)
    );
    
    // Show sample of connected addresses
    if let Some(connections) = graph_edges.get(&seed_address) {
        println!("\n=== Sample Connected Addresses (max 10) ===");
        for (i, addr) in connections.iter().take(10).enumerate() {
            // Get metadata for connected address
            let metrics = queries::address_metrics::get_address_metrics(
                pg_query.db(),
                addr,
            ).await?;
            
            if let Some(m) = metrics {
                println!("{}. {}", i + 1, addr);
                println!("   Type: {}", if m.is_contract { "Contract" } else { "EOA" });
                if let Some(category) = m.entity_category {
                    println!("   Category: {}", category);
                }
                if let Some(name) = m.name {
                    println!("   Name: {}", name);
                }
                println!("   Total Volume: ${:.2}", m.total_volume.unwrap_or(0.0));
            } else {
                println!("{}. {} (no metadata)", i + 1, addr);
            }
        }
    }
    
    // Demonstrate depth-2 exploration (optional)
    let explore_depth_2 = env::var("EXPLORE_DEPTH_2")
        .map(|v| v == "true")
        .unwrap_or(false);
    
    if explore_depth_2 && !graph_edges.is_empty() {
        println!("\n=== Exploring Depth 2 ===");
        
        // Take first connected address and explore it
        if let Some(connections) = graph_edges.get(&seed_address) {
            if let Some(first_connection) = connections.iter().next() {
                println!("Exploring: {}", first_connection);
                
                let depth2_txs = queries::transactions::get_address_transactions(
                    pg_query.db(),
                    first_connection,
                    50,   // smaller limit for depth 2
                    None,
                ).await?;
                
                println!("  Found {} transactions", depth2_txs.len());
                
                let mut depth2_connections = HashSet::new();
                for tx in depth2_txs {
                    let participants = queries::transactions::get_tx_participants(
                        pg_query.db(),
                        &tx.tx_hash,
                    ).await?;
                    
                    for p in participants {
                        if p != *first_connection && p != seed_address {
                            depth2_connections.insert(p);
                        }
                    }
                }
                
                println!("  Depth-2 connections: {}", depth2_connections.len());
            }
        }
    }
    
    // Show transaction details for a sample transaction
    if let Some(first_tx) = transactions.first() {
        println!("\n=== Sample Transaction Details ===");
        println!("TX Hash: {}", first_tx.tx_hash);
        println!("Block: {}", first_tx.block_number);
        println!("From: {}", first_tx.from_address);
        println!("To: {}", first_tx.to_address);
        if let Some(value) = first_tx.value {
            println!("Value: {} ETH", value);
        }
        println!("Participants: {}", first_tx.participant_count);
        
        // Get full details with all participants
        let full_tx = queries::transactions::get_transaction_with_participants(
            pg_query.db(),
            &first_tx.tx_hash,
        ).await?;
        
        if let Some(tx) = full_tx {
            println!("All participants in this transaction:");
            for (i, participant) in tx.participants.iter().enumerate() {
                println!("  {}. {}", i + 1, participant);
            }
        }
    }
    
    println!("\n✓ Graph building complete!");
    
    Ok(())
}