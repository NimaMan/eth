//! Network analysis example - demonstrates network analysis features

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_network_building::{NetworkBuilder, NetworkAnalyzer};
use alloy_primitives::Address;
use std::str::FromStr;
use std::collections::{HashMap, HashSet};
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Network Analysis Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting network analysis example");
    
    // Create complex test network
    let fund_flows = create_complex_test_network()?;
    println!("1. Created complex test network with {} fund flows", fund_flows.len());
    
    // Build the network
    let params = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 1.0, // Very low threshold to include all flows
        max_depth: 3,
        include_gas: true,
        treat_weth_as_eth: true,
    };
    
    let builder = NetworkBuilder::new().with_params(params);
    let network = builder.build_from_fund_flows(&fund_flows, None)?;
    
    println!("2. Built network: {} nodes, {} edges\n", network.nodes.len(), network.edges.len());
    
    // Perform various analyses
    analyze_basic_structure(&network).await?;
    analyze_centrality_simple(&network).await?;
    analyze_connectivity_patterns(&network).await?;
    analyze_path_finding(&network).await?;
    
    // Show analysis summary
    show_analysis_summary(&network);
    
    println!("\n=== Network analysis example completed! ===");
    
    Ok(())
}

fn create_complex_test_network() -> QarqaResult<Vec<FundFlow>> {
    let mut flows = Vec::new();
    
    // Create a more complex network with multiple patterns:
    // 1. Hub and spoke pattern (central exchange)
    let hub = Address::from_str("0x1000000000000000000000000000000000000001")?; // Central exchange
    
    // Spoke connections
    for i in 1..=5 {
        let spoke = Address::from_str(&format!("0x200000000000000000000000000000000000000{}", i))?;
        
        // Inflow to hub
        flows.push(FundFlow {
            from: spoke,
            to: hub,
            amount_eth: 2.0 + (i as f64 * 0.5),
            amount_tokens_usd: 1000.0 * i as f64,
            transaction_count: i as u64,
            first_block: 18500000,
            last_block: 18500010 + i as u64,
            flow_type: FlowType::DirectTransfer,
        });
        
        // Outflow from hub
        flows.push(FundFlow {
            from: hub,
            to: spoke,
            amount_eth: 1.5 + (i as f64 * 0.3),
            amount_tokens_usd: 800.0 * i as f64,
            transaction_count: i as u64,
            first_block: 18500005,
            last_block: 18500015 + i as u64,
            flow_type: FlowType::DirectTransfer,
        });
    }
    
    // 2. Chain pattern (A -> B -> C -> D)
    let chain_addresses = [
        "0x3000000000000000000000000000000000000001",
        "0x3000000000000000000000000000000000000002", 
        "0x3000000000000000000000000000000000000003",
        "0x3000000000000000000000000000000000000004",
    ];
    
    for i in 0..chain_addresses.len()-1 {
        let from = Address::from_str(chain_addresses[i])?;
        let to = Address::from_str(chain_addresses[i+1])?;
        
        flows.push(FundFlow {
            from,
            to,
            amount_eth: 1.0 - (i as f64 * 0.1), // Decreasing amounts
            amount_tokens_usd: 500.0,
            transaction_count: 1,
            first_block: 18500020 + i as u64,
            last_block: 18500020 + i as u64,
            flow_type: FlowType::ContractInteraction,
        });
    }
    
    // 3. Circular pattern
    let circle_addresses = [
        "0x4000000000000000000000000000000000000001",
        "0x4000000000000000000000000000000000000002",
        "0x4000000000000000000000000000000000000003",
    ];
    
    for i in 0..circle_addresses.len() {
        let from = Address::from_str(circle_addresses[i])?;
        let to = Address::from_str(circle_addresses[(i+1) % circle_addresses.len()])?;
        
        flows.push(FundFlow {
            from,
            to,
            amount_eth: 0.8,
            amount_tokens_usd: 0.0,
            transaction_count: 2,
            first_block: 18500030,
            last_block: 18500035,
            flow_type: FlowType::InternalTransfer,
        });
    }
    
    // 4. Gas payments for major participants
    let major_addresses = [hub, Address::from_str(circle_addresses[0])?];
    
    for addr in major_addresses.iter() {
        flows.push(FundFlow {
            from: *addr,
            to: Address::ZERO,
            amount_eth: 0.005,
            amount_tokens_usd: 0.0,
            transaction_count: 15,
            first_block: 18500000,
            last_block: 18500070,
            flow_type: FlowType::GasPayment,
        });
    }
    
    Ok(flows)
}

async fn analyze_basic_structure(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("3. Basic Network Structure Analysis:");
    
    // Calculate basic metrics
    let node_count = network.nodes.len();
    let edge_count = network.edges.len();
    
    let max_edges = if node_count > 1 {
        node_count * (node_count - 1)
    } else {
        1
    };
    let density = edge_count as f64 / max_edges as f64;
    
    // Calculate degree statistics
    let mut degrees: Vec<usize> = Vec::new();
    for (address, _) in &network.nodes {
        let in_degree = network.get_edges_to(address).len();
        let out_degree = network.get_edges_from(address).len();
        degrees.push(in_degree + out_degree);
    }
    
    let max_degree = degrees.iter().max().unwrap_or(&0);
    let avg_degree = if !degrees.is_empty() {
        degrees.iter().sum::<usize>() as f64 / degrees.len() as f64
    } else {
        0.0
    };
    
    println!("   Basic Metrics:");
    println!("     Nodes: {}", node_count);
    println!("     Edges: {}", edge_count);
    println!("     Network density: {:.4}", density);
    println!("     Average degree: {:.2}", avg_degree);
    println!("     Max degree: {}", max_degree);
    
    println!();
    Ok(())
}

async fn analyze_centrality_simple(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("4. Centrality Analysis:");
    
    // Calculate degree centrality using the available method
    let centrality = NetworkAnalyzer::calculate_centrality(network);
    
    // Sort by centrality
    let mut centrality_pairs: Vec<_> = centrality.iter().collect();
    centrality_pairs.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    println!("   Degree Centrality (top 5):");
    for (i, (addr, centrality)) in centrality_pairs.iter().take(5).enumerate() {
        println!("     {}. {}: {} connections", i + 1, format_address(**addr), centrality);
    }
    
    // Calculate flow-based centrality
    let mut flow_centrality: HashMap<Address, f64> = HashMap::new();
    for (address, _) in &network.nodes {
        let total_flow: f64 = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .map(|e| wei_to_eth(e.total_eth))
            .sum();
        flow_centrality.insert(*address, total_flow);
    }
    
    let mut flow_pairs: Vec<_> = flow_centrality.iter().collect();
    flow_pairs.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    println!("   Flow Centrality (top 5):");
    for (i, (addr, flow_value)) in flow_pairs.iter().take(5).enumerate() {
        println!("     {}. {}: {} ETH total flow", i + 1, format_address(**addr), flow_value);
    }
    
    println!();
    Ok(())
}

async fn analyze_connectivity_patterns(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("5. Connectivity Pattern Analysis:");
    
    // Find connected components manually
    let mut visited: HashSet<Address> = HashSet::new();
    let mut components: Vec<Vec<Address>> = Vec::new();
    
    for (address, _) in &network.nodes {
        if !visited.contains(address) {
            let mut component = Vec::new();
            let mut stack = vec![*address];
            
            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }
                visited.insert(current);
                component.push(current);
                
                // Add neighbors
                for edge in &network.edges {
                    if edge.from == current && !visited.contains(&edge.to) {
                        stack.push(edge.to);
                    }
                    if edge.to == current && !visited.contains(&edge.from) {
                        stack.push(edge.from);
                    }
                }
            }
            
            if !component.is_empty() {
                components.push(component);
            }
        }
    }
    
    println!("   Connected Components: {}", components.len());
    for (i, component) in components.iter().enumerate() {
        println!("     Component {}: {} nodes", i + 1, component.len());
        if component.len() <= 5 {
            for addr in component {
                println!("       - {}", format_address(*addr));
            }
        }
    }
    
    // Detect cycles using the available method
    let cycles = NetworkAnalyzer::detect_cycles(network);
    println!("   Detected Cycles: {}", cycles.len());
    
    for (i, cycle) in cycles.iter().take(3).enumerate() {
        println!("     Cycle {}: {} nodes", i + 1, cycle.len());
        for addr in cycle {
            println!("       → {}", format_address(*addr));
        }
    }
    
    println!();
    Ok(())
}

async fn analyze_path_finding(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("6. Path Finding Analysis:");
    
    if network.nodes.len() < 2 {
        println!("   Not enough nodes for path finding");
        return Ok(());
    }
    
    // Get first and last nodes
    let addresses: Vec<_> = network.nodes.keys().collect();
    let source = *addresses[0];
    let target = *addresses[addresses.len() - 1];
    
    println!("   Finding paths from {} to {}:", format_address(source), format_address(target));
    
    // Find shortest path using the available method
    match NetworkAnalyzer::find_shortest_path(network, source, target) {
        Some(path) => {
            println!("     Shortest path ({} hops):", path.len() - 1);
            for (i, addr) in path.iter().enumerate() {
                if i == path.len() - 1 {
                    println!("       {}", format_address(*addr));
                } else {
                    println!("       {} →", format_address(*addr));
                }
            }
        }
        None => {
            println!("     No path found (nodes not connected)");
        }
    }
    
    // Test path finding between different pairs
    let mut connected_pairs = 0;
    let mut total_tests = 0;
    
    for i in 0..std::cmp::min(5, addresses.len()) {
        for j in i+1..std::cmp::min(5, addresses.len()) {
            total_tests += 1;
            if NetworkAnalyzer::find_shortest_path(network, *addresses[i], *addresses[j]).is_some() {
                connected_pairs += 1;
            }
        }
    }
    
    println!("   Connectivity Statistics:");
    println!("     Connected pairs: {} out of {} tested", connected_pairs, total_tests);
    if total_tests > 0 {
        println!("     Connectivity ratio: {:.2}%", (connected_pairs as f64 / total_tests as f64) * 100.0);
    }
    
    println!();
    Ok(())
}

fn show_analysis_summary(network: &qarqa_network_building::FundFlowNetwork) {
    println!("7. Analysis Summary:");
    
    // Flow type distribution
    let mut flow_type_stats: HashMap<FlowType, (usize, f64)> = HashMap::new();
    
    for edge in &network.edges {
        for flow_type in &edge.flow_types {
            let entry = flow_type_stats.entry(flow_type.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += wei_to_eth(edge.total_eth);
        }
    }
    
    println!("   Flow type distribution:");
    for (flow_type, (count, total_eth)) in flow_type_stats.iter() {
        println!("     {:?}: {} flows, {} ETH", flow_type, count, total_eth);
    }
    
    // Network health indicators
    let total_flow: f64 = network.edges.iter().map(|e| wei_to_eth(e.total_eth)).sum();
    let gas_flow: f64 = network.edges.iter()
        .filter(|e| e.flow_types.iter().any(|ft| matches!(ft, FlowType::GasPayment)))
        .map(|e| wei_to_eth(e.total_eth))
        .sum();
    
    println!("   Network health:");
    println!("     Total flow volume: {} ETH", total_flow);
    if total_flow > 0.0 {
        println!("     Gas payment ratio: {:.2}%", (gas_flow / total_flow) * 100.0);
    }
    if !network.edges.is_empty() {
        println!("     Average edge weight: {} ETH", total_flow / network.edges.len() as f64);
    }
    
    // Complexity metrics
    let unique_flow_types = flow_type_stats.len();
    let node_edge_ratio = if !network.nodes.is_empty() {
        network.edges.len() as f64 / network.nodes.len() as f64
    } else {
        0.0
    };
    
    println!("   Complexity metrics:");
    println!("     Unique flow types: {}", unique_flow_types);
    println!("     Edge-to-node ratio: {:.2}", node_edge_ratio);
    
    println!("   Analysis completed successfully!");
}