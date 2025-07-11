//! Simple test to verify fundflownetwork compilation and basic usage

use qarqa_fundflownetwork::{
    FundFlowAnalyzer, NetworkBuilder, 
    CytoscapeExporter, VisJsExporter,
};
use qarqa_core_types::{CompleteFundFlows, EthMovement, EthMovementType, FlowType};
use alloy_primitives::{Address, U256};
use std::str::FromStr;

fn main() {
    println!("Testing fund flow network compilation...");
    
    // Create test data
    let eth_movement = EthMovement {
        from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
        to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
        amount: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
        movement_type: EthMovementType::Direct,
    };
    
    let complete_flows = CompleteFundFlows {
        tx_hash: "0xabcd".to_string(),
        block_number: 12345,
        from_address: eth_movement.from,
        to_address: Some(eth_movement.to),
        eth_movements: vec![eth_movement],
        token_movements: Vec::new(),
    };
    
    // Test fund flow analyzer
    let analyzer = FundFlowAnalyzer::new();
    let fund_flows = analyzer.analyze_fund_flows(&[complete_flows]);
    println!("Extracted {} fund flows", fund_flows.len());
    
    // Test network builder
    let builder = NetworkBuilder::new();
    let network = builder.build_from_flows(&fund_flows);
    println!("Built network with {} nodes and {} edges", 
             network.nodes.len(), network.edges.len());
    
    // Test visualization export
    let cytoscape_json = CytoscapeExporter::export(&network).unwrap();
    println!("Exported to Cytoscape format: {} bytes", 
             serde_json::to_string(&cytoscape_json).unwrap().len());
    
    let visjs_json = VisJsExporter::export(&network).unwrap();
    println!("Exported to vis.js format: {} bytes", 
             serde_json::to_string(&visjs_json).unwrap().len());
    
    println!("\n✅ All components working correctly!");
}