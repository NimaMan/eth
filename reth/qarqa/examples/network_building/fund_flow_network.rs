//! Fund flow network example - demonstrates building and analyzing fund flow networks

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_network_building::{NetworkBuilder};
use alloy_primitives::Address;
use std::str::FromStr;
use std::collections::HashMap;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Fund Flow Network Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting fund flow network example");
    
    // Create test fund flows
    let fund_flows = create_test_fund_flows()?;
    println!("1. Created {} test fund flows", fund_flows.len());
    
    // Display fund flow summary
    display_fund_flow_summary(&fund_flows);
    
    // Create network builder with configuration
    let params = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 25.0, // $25 minimum (0.01 ETH * $2500)
        max_depth: 3,
        include_gas: true,
        treat_weth_as_eth: true,
    };
    
    let builder = NetworkBuilder::new().with_params(params);
    
    println!("2. Building fund flow network...");
    
    // Build the network
    let network = builder.build_from_fund_flows(&fund_flows, None)?;
    
    println!("   ✓ Network built successfully!");
    println!("   Nodes: {}, Edges: {}", network.nodes.len(), network.edges.len());
    println!();
    
    // Analyze the network
    analyze_network(&network);
    
    // Demonstrate different network configurations
    demonstrate_network_configurations(&fund_flows).await?;
    
    // Show network insights
    demonstrate_network_insights(&network);
    
    println!("\n=== Fund flow network example completed! ===");
    
    Ok(())
}

fn create_test_fund_flows() -> QarqaResult<Vec<FundFlow>> {
    let mut flows = Vec::new();
    
    // Create a network with multiple connected components
    
    // Component 1: Trading cluster
    flows.push(FundFlow {
        from: Address::from_str("0x1111111111111111111111111111111111111111")?,
        to: Address::from_str("0x2222222222222222222222222222222222222222")?,
        amount_eth: 5.5,
        amount_tokens_usd: 0.0,
        transaction_count: 3,
        first_block: 18500000,
        last_block: 18500010,
        flow_type: FlowType::DirectTransfer,
    });
    
    flows.push(FundFlow {
        from: Address::from_str("0x2222222222222222222222222222222222222222")?,
        to: Address::from_str("0x3333333333333333333333333333333333333333")?,
        amount_eth: 2.1,
        amount_tokens_usd: 1500.0,
        transaction_count: 2,
        first_block: 18500001,
        last_block: 18500008,
        flow_type: FlowType::TokenTransfer(Address::from_str("0x6666666666666666666666666666666666666666")?),
    });
    
    flows.push(FundFlow {
        from: Address::from_str("0x3333333333333333333333333333333333333333")?,
        to: Address::from_str("0x4444444444444444444444444444444444444444")?,
        amount_eth: 1.8,
        amount_tokens_usd: 800.0,
        transaction_count: 1,
        first_block: 18500005,
        last_block: 18500005,
        flow_type: FlowType::ContractInteraction,
    });
    
    // Component 2: DeFi protocol interaction
    flows.push(FundFlow {
        from: Address::from_str("0x5555555555555555555555555555555555555555")?,
        to: Address::from_str("0x7777777777777777777777777777777777777777")?, // DeFi contract
        amount_eth: 10.0,
        amount_tokens_usd: 15000.0,
        transaction_count: 5,
        first_block: 18500002,
        last_block: 18500012,
        flow_type: FlowType::ContractInteraction,
    });
    
    flows.push(FundFlow {
        from: Address::from_str("0x7777777777777777777777777777777777777777")?,
        to: Address::from_str("0x8888888888888888888888888888888888888888")?,
        amount_eth: 0.5,
        amount_tokens_usd: 750.0,
        transaction_count: 2,
        first_block: 18500003,
        last_block: 18500009,
        flow_type: FlowType::InternalTransfer,
    });
    
    // Component 3: Gas payments (should be filtered in some configurations)
    flows.push(FundFlow {
        from: Address::from_str("0x1111111111111111111111111111111111111111")?,
        to: Address::ZERO, // Miners
        amount_eth: 0.002,
        amount_tokens_usd: 0.0,
        transaction_count: 10,
        first_block: 18500000,
        last_block: 18500015,
        flow_type: FlowType::GasPayment,
    });
    
    flows.push(FundFlow {
        from: Address::from_str("0x5555555555555555555555555555555555555555")?,
        to: Address::ZERO,
        amount_eth: 0.003,
        amount_tokens_usd: 0.0,
        transaction_count: 8,
        first_block: 18500000,
        last_block: 18500015,
        flow_type: FlowType::GasPayment,
    });
    
    // Circular flow (A -> B -> C -> A)
    flows.push(FundFlow {
        from: Address::from_str("0x4444444444444444444444444444444444444444")?,
        to: Address::from_str("0x1111111111111111111111111111111111111111")?,
        amount_eth: 0.8,
        amount_tokens_usd: 0.0,
        transaction_count: 1,
        first_block: 18500020,
        last_block: 18500020,
        flow_type: FlowType::DirectTransfer,
    });
    
    Ok(flows)
}

fn display_fund_flow_summary(fund_flows: &[FundFlow]) {
    println!("   Fund Flow Summary:");
    for (i, flow) in fund_flows.iter().enumerate() {
        println!("     {}. {} → {}: {} ETH ({:?})",
                 i + 1,
                 format_address(flow.from),
                 format_address(flow.to),
                 flow.amount_eth,
                 flow.flow_type);
    }
    println!();
}

fn analyze_network(network: &qarqa_network_building::FundFlowNetwork) {
    println!("3. Network Analysis:");
    
    // Basic statistics
    println!("   Basic Statistics:");
    println!("     Total nodes: {}", network.nodes.len());
    println!("     Total edges: {}", network.edges.len());
    
    // Calculate network density
    let max_edges = network.nodes.len() * (network.nodes.len() - 1);
    let density = if max_edges > 0 { 
        (network.edges.len() as f64 / max_edges as f64) * 100.0 
    } else { 
        0.0 
    };
    println!("     Network density: {:.2}%", density);
    
    // Node degree analysis
    let mut in_degrees: HashMap<Address, usize> = HashMap::new();
    let mut out_degrees: HashMap<Address, usize> = HashMap::new();
    
    for edge in &network.edges {
        *out_degrees.entry(edge.from).or_insert(0) += 1;
        *in_degrees.entry(edge.to).or_insert(0) += 1;
    }
    
    // Find highest degree nodes
    let mut degree_pairs: Vec<_> = network.nodes.iter()
        .map(|(address, _node)| {
            let in_deg = in_degrees.get(address).unwrap_or(&0);
            let out_deg = out_degrees.get(address).unwrap_or(&0);
            (*address, in_deg + out_deg, *in_deg, *out_deg)
        })
        .collect();
    degree_pairs.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!("   Node Degree Analysis:");
    println!("     Highest degree nodes:");
    for (i, (addr, total_deg, in_deg, out_deg)) in degree_pairs.iter().take(3).enumerate() {
        println!("       {}. {}: {} total (in: {}, out: {})",
                 i + 1,
                 format_address(*addr),
                 total_deg,
                 in_deg,
                 out_deg);
    }
    
    // Value flow analysis
    let total_flow_value: f64 = network.edges.iter().map(|e| wei_to_eth(e.total_eth)).sum();
    let avg_flow_value = if !network.edges.is_empty() {
        total_flow_value / network.edges.len() as f64
    } else {
        0.0
    };
    
    println!("   Value Flow Analysis:");
    println!("     Total ETH flow: {} ETH", total_flow_value);
    println!("     Average flow size: {} ETH", avg_flow_value);
    
    // Find largest flows
    let mut largest_flows = network.edges.clone();
    largest_flows.sort_by(|a, b| b.total_eth.cmp(&a.total_eth));
    
    println!("     Largest flows:");
    for (i, edge) in largest_flows.iter().take(3).enumerate() {
        println!("       {}. {} → {}: {} ETH",
                 i + 1,
                 format_address(edge.from),
                 format_address(edge.to),
                 wei_to_eth(edge.total_eth));
    }
    println!();
}

async fn demonstrate_network_configurations(fund_flows: &[FundFlow]) -> QarqaResult<()> {
    println!("4. Testing different network configurations:");
    
    // Configuration 1: Exclude gas payments
    let params_no_gas = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 10.0,
        max_depth: 3,
        include_gas: false,
        treat_weth_as_eth: true,
    };
    let builder_no_gas = NetworkBuilder::new().with_params(params_no_gas);
    let network_no_gas = builder_no_gas.build_from_fund_flows(fund_flows, None)?;
    println!("   Without gas payments: {} nodes, {} edges", 
             network_no_gas.nodes.len(), network_no_gas.edges.len());
    
    // Configuration 2: Higher minimum flow amount
    let params_high_min = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 2500.0, // $2500 minimum (1 ETH)
        max_depth: 3,
        include_gas: true,
        treat_weth_as_eth: true,
    };
    let builder_high_min = NetworkBuilder::new().with_params(params_high_min);
    let network_high_min = builder_high_min.build_from_fund_flows(fund_flows, None)?;
    println!("   High minimum ($2500): {} nodes, {} edges",
             network_high_min.nodes.len(), network_high_min.edges.len());
    
    // Configuration 3: Lower threshold
    let params_low_thresh = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 1.0, // $1 minimum
        max_depth: 2,
        include_gas: true,
        treat_weth_as_eth: false,
    };
    let builder_low_thresh = NetworkBuilder::new().with_params(params_low_thresh);
    let network_low_thresh = builder_low_thresh.build_from_fund_flows(fund_flows, None)?;
    println!("   Low threshold ($1): {} nodes, {} edges",
             network_low_thresh.nodes.len(), network_low_thresh.edges.len());
    
    println!();
    
    Ok(())
}

fn demonstrate_network_insights(network: &qarqa_network_building::FundFlowNetwork) {
    println!("5. Network Insights:");
    
    // Identify potential clusters
    identify_clusters(network);
    
    // Identify important nodes
    identify_important_nodes(network);
    
    // Flow pattern analysis
    analyze_flow_patterns(network);
}

fn identify_clusters(network: &qarqa_network_building::FundFlowNetwork) {
    println!("   Cluster Analysis:");
    
    // Simple clustering based on flow types
    let mut flow_type_clusters: HashMap<FlowType, Vec<Address>> = HashMap::new();
    
    for edge in &network.edges {
        for flow_type in &edge.flow_types {
            flow_type_clusters.entry(flow_type.clone()).or_default().push(edge.from);
            flow_type_clusters.entry(flow_type.clone()).or_default().push(edge.to);
        }
    }
    
    for (flow_type, addresses) in flow_type_clusters.iter() {
        let unique_addresses: std::collections::HashSet<_> = addresses.iter().collect();
        println!("     {:?} cluster: {} unique addresses", flow_type, unique_addresses.len());
    }
}

fn identify_important_nodes(network: &qarqa_network_building::FundFlowNetwork) {
    println!("   Important Node Analysis:");
    
    // Calculate centrality metrics
    let mut flow_centrality: HashMap<Address, f64> = HashMap::new();
    
    for (address, _node) in &network.nodes {
        // Simple flow-based centrality
        let total_flow: f64 = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .map(|e| wei_to_eth(e.total_eth))
            .sum();
        
        flow_centrality.insert(*address, total_flow);
    }
    
    // Find top nodes by flow centrality
    let mut centrality_pairs: Vec<_> = flow_centrality.iter().collect();
    centrality_pairs.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    println!("     Most central nodes (by total flow):");
    for (i, (addr, centrality)) in centrality_pairs.iter().take(3).enumerate() {
        println!("       {}. {}: {} ETH total flow",
                 i + 1,
                 format_address(**addr),
                 centrality);
    }
}

fn analyze_flow_patterns(network: &qarqa_network_building::FundFlowNetwork) {
    println!("   Flow Pattern Analysis:");
    
    // Detect potential circular flows
    let mut potential_circles = 0;
    for edge in &network.edges {
        // Check if there's a reverse edge
        if network.edges.iter().any(|e| e.from == edge.to && e.to == edge.from) {
            potential_circles += 1;
        }
    }
    
    println!("     Potential circular flows: {}", potential_circles / 2); // Divide by 2 to avoid double counting
    
    // Analyze flow types distribution
    let mut flow_type_counts: HashMap<FlowType, usize> = HashMap::new();
    let mut flow_type_values: HashMap<FlowType, f64> = HashMap::new();
    
    for edge in &network.edges {
        for flow_type in &edge.flow_types {
            *flow_type_counts.entry(flow_type.clone()).or_insert(0) += 1;
            *flow_type_values.entry(flow_type.clone()).or_insert(0.0) += wei_to_eth(edge.total_eth);
        }
    }
    
    println!("     Flow type distribution:");
    for (flow_type, count) in flow_type_counts.iter() {
        let total_value = flow_type_values.get(flow_type).unwrap_or(&0.0);
        println!("       {:?}: {} flows, {} ETH total", flow_type, count, total_value);
    }
    
    // Network connectivity
    let zero_address_connections = network.edges.iter()
        .filter(|e| e.to == Address::ZERO)
        .count();
    
    if zero_address_connections > 0 {
        println!("     Gas payment flows: {} (to miners)", zero_address_connections);
    }
}