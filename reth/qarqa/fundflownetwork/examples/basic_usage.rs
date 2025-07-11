//! Basic usage example for fundflownetwork

use qarqa_fundflownetwork::{
    FundFlowAnalyzer, NetworkBuilder, 
    CytoscapeExporter, VisJsExporter,
};
use qarqa_core_types::{CompleteFundFlows, EthMovement, EthMovementType};
use alloy_primitives::{Address, U256};
use std::str::FromStr;

fn main() {
    println!("Fund Flow Network Example");
    
    // Create test transaction data
    let flows = create_test_flows();
    
    // Analyze fund flows
    let analyzer = FundFlowAnalyzer::new()
        .with_min_value(U256::from(1_000_000_000_000_000u128)); // 0.001 ETH minimum
    
    let fund_flows = analyzer.analyze_fund_flows(&flows);
    println!("\nExtracted {} unique fund flows:", fund_flows.len());
    
    for flow in &fund_flows {
        println!("  {} → {}: {:.4} ETH",
                 format!("{:?}", flow.from)[..10].to_string(),
                 format!("{:?}", flow.to)[..10].to_string(),
                 flow.amount_eth);
    }
    
    // Calculate net balances
    let balances = analyzer.calculate_net_balances(&fund_flows);
    println!("\nNet balances:");
    for (addr, balance) in &balances {
        if balance.net_eth.abs() > 0.0 {
            println!("  {}: {:+.4} ETH",
                     format!("{:?}", addr)[..10].to_string(),
                     balance.net_eth);
        }
    }
    
    // Build network
    let builder = NetworkBuilder::new()
        .with_min_eth_amount(0.01);
    
    let network = builder.build_from_flows(&fund_flows);
    
    println!("\nNetwork statistics:");
    let stats = network.calculate_stats();
    println!("  Nodes: {}", stats.node_count);
    println!("  Edges: {}", stats.edge_count);
    println!("  Total volume: {:.4} ETH", stats.total_volume_eth);
    
    // Export for visualization
    let cytoscape = CytoscapeExporter::export(&network).unwrap();
    std::fs::write("network_cytoscape.json", 
                   serde_json::to_string_pretty(&cytoscape).unwrap()).unwrap();
    
    let visjs = VisJsExporter::export(&network).unwrap();
    std::fs::write("network_visjs.json", 
                   serde_json::to_string_pretty(&visjs).unwrap()).unwrap();
    
    println!("\n✅ Exported network to network_cytoscape.json and network_visjs.json");
}

fn create_test_flows() -> Vec<CompleteFundFlows> {
    let addr1 = Address::from_str("0x1111111111111111111111111111111111111111").unwrap();
    let addr2 = Address::from_str("0x2222222222222222222222222222222222222222").unwrap();
    let addr3 = Address::from_str("0x3333333333333333333333333333333333333333").unwrap();
    let addr4 = Address::from_str("0x4444444444444444444444444444444444444444").unwrap();
    
    vec![
        // Transaction 1: Direct transfer
        CompleteFundFlows {
            tx_hash: "0xabc1".to_string(),
            block_number: 12345,
            from_address: addr1,
            to_address: Some(addr2),
            eth_movements: vec![
                EthMovement {
                    from: addr1,
                    to: addr2,
                    amount: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
                    movement_type: EthMovementType::Direct,
                },
            ],
            token_movements: Vec::new(),
        },
        // Transaction 2: Contract interaction with internal transfers
        CompleteFundFlows {
            tx_hash: "0xabc2".to_string(),
            block_number: 12346,
            from_address: addr1,
            to_address: Some(addr3),
            eth_movements: vec![
                EthMovement {
                    from: addr1,
                    to: addr3,
                    amount: U256::from(500_000_000_000_000_000u128), // 0.5 ETH
                    movement_type: EthMovementType::Direct,
                },
                EthMovement {
                    from: addr3,
                    to: addr4,
                    amount: U256::from(250_000_000_000_000_000u128), // 0.25 ETH
                    movement_type: EthMovementType::Internal,
                },
                EthMovement {
                    from: addr3,
                    to: addr2,
                    amount: U256::from(250_000_000_000_000_000u128), // 0.25 ETH
                    movement_type: EthMovementType::Internal,
                },
            ],
            token_movements: Vec::new(),
        },
        // Transaction 3: Another transfer
        CompleteFundFlows {
            tx_hash: "0xabc3".to_string(),
            block_number: 12347,
            from_address: addr2,
            to_address: Some(addr4),
            eth_movements: vec![
                EthMovement {
                    from: addr2,
                    to: addr4,
                    amount: U256::from(2_000_000_000_000_000_000u128), // 2 ETH
                    movement_type: EthMovementType::Direct,
                },
            ],
            token_movements: Vec::new(),
        },
    ]
}