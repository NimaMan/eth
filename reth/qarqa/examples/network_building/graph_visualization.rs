//! Graph visualization example - demonstrates network visualization concepts

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_network_building::NetworkBuilder;
use alloy_primitives::Address;
use std::str::FromStr;
use std::collections::HashMap;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Graph Visualization Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting graph visualization example");
    
    // Create test network for visualization
    let fund_flows = create_visualization_test_network()?;
    println!("1. Created test network with {} fund flows", fund_flows.len());
    
    // Build the network
    let params = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 25.0, // $25 minimum for cleaner visualization
        max_depth: 3,
        include_gas: false, // Exclude gas for cleaner visualization
        treat_weth_as_eth: true,
    };
    
    let builder = NetworkBuilder::new().with_params(params);
    let network = builder.build_from_fund_flows(&fund_flows, None)?;
    println!("2. Built network: {} nodes, {} edges\n", network.nodes.len(), network.edges.len());
    
    // Generate different visualization formats
    generate_ascii_visualization(&network);
    generate_dot_format(&network)?;
    generate_json_format(&network)?;
    generate_adjacency_matrix(&network);
    generate_force_layout_data(&network)?;
    
    // Demonstrate visualization concepts
    demonstrate_visualization_concepts();
    
    println!("\n=== Graph visualization example completed! ===");
    
    Ok(())
}

fn create_visualization_test_network() -> QarqaResult<Vec<FundFlow>> {
    let mut flows = Vec::new();
    
    // Create a simple but interesting network for visualization
    // Central hub with connections
    let central_hub = Address::from_str("0x1111111111111111111111111111111111111111")?;
    
    // Connected nodes
    let nodes = [
        "0x2222222222222222222222222222222222222222", // Exchange
        "0x3333333333333333333333333333333333333333", // DeFi Protocol  
        "0x4444444444444444444444444444444444444444", // Whale
        "0x5555555555555555555555555555555555555555", // Bot
        "0x6666666666666666666666666666666666666666", // Mixer
    ];
    
    // Hub connections
    for (i, node_addr) in nodes.iter().enumerate() {
        let node = Address::from_str(node_addr)?;
        
        // Bidirectional flows with central hub
        flows.push(FundFlow {
            from: central_hub,
            to: node,
            amount_eth: 1.0 + (i as f64 * 0.5),
            amount_tokens_usd: 1000.0 * (i + 1) as f64,
            transaction_count: (i + 1) as u64,
            first_block: 18500000,
            last_block: 18500010,
            flow_type: match i {
                0 => FlowType::DirectTransfer,
                1 => FlowType::ContractInteraction,
                2 => FlowType::DirectTransfer,
                3 => FlowType::InternalTransfer,
                4 => FlowType::InternalTransfer,
                _ => FlowType::DirectTransfer,
            },
        });
        
        flows.push(FundFlow {
            from: node,
            to: central_hub,
            amount_eth: 0.8 + (i as f64 * 0.3),
            amount_tokens_usd: 800.0 * (i + 1) as f64,
            transaction_count: (i + 1) as u64,
            first_block: 18500005,
            last_block: 18500015,
            flow_type: FlowType::DirectTransfer,
        });
    }
    
    // Additional connections between nodes
    flows.push(FundFlow {
        from: Address::from_str(nodes[0])?, // Exchange
        to: Address::from_str(nodes[1])?,   // DeFi Protocol
        amount_eth: 2.5,
        amount_tokens_usd: 3000.0,
        transaction_count: 3,
        first_block: 18500020,
        last_block: 18500025,
        flow_type: FlowType::ContractInteraction,
    });
    
    flows.push(FundFlow {
        from: Address::from_str(nodes[2])?, // Whale
        to: Address::from_str(nodes[0])?,   // Exchange
        amount_eth: 5.0,
        amount_tokens_usd: 8000.0,
        transaction_count: 2,
        first_block: 18500030,
        last_block: 18500035,
        flow_type: FlowType::DirectTransfer,
    });
    
    flows.push(FundFlow {
        from: Address::from_str(nodes[3])?, // Bot
        to: Address::from_str(nodes[4])?,   // Mixer
        amount_eth: 0.5,
        amount_tokens_usd: 0.0,
        transaction_count: 10,
        first_block: 18500040,
        last_block: 18500050,
        flow_type: FlowType::InternalTransfer,
    });
    
    Ok(flows)
}

fn generate_ascii_visualization(network: &qarqa_network_building::FundFlowNetwork) {
    println!("3. ASCII Network Visualization:");
    println!();
    
    // Create a simple ASCII representation
    let node_labels = create_node_labels(network);
    
    println!("   Network Structure:");
    println!("   ==================");
    
    // Show nodes
    println!("   Nodes:");
    for (i, (address, _node)) in network.nodes.iter().enumerate() {
        let default_label = "Unknown".to_string();
        let label = node_labels.get(address).unwrap_or(&default_label);
        println!("     [{}] {} ({})", 
                 char::from(b'A' + i as u8),
                 format_address(*address),
                 label);
    }
    
    println!();
    println!("   Edges:");
    for edge in &network.edges {
        let from_label = get_node_letter(network, edge.from);
        let to_label = get_node_letter(network, edge.to);
        let thickness = if wei_to_eth(edge.total_eth) > 2.0 { "==>" } else { "-->" };
        
        let flow_type_str = if edge.flow_types.is_empty() {
            "Unknown".to_string()
        } else {
            format!("{:?}", edge.flow_types[0])
        };
        println!("     {} {} {} ({:.2} ETH, {})",
                 from_label,
                 thickness,
                 to_label,
                 wei_to_eth(edge.total_eth),
                 flow_type_str);
    }
    
    // Simple visual representation
    println!();
    println!("   Visual Representation:");
    println!("   ======================");
    
    // Create a simple layout
    create_simple_ascii_layout(network, &node_labels);
    
    println!();
}

fn create_node_labels(network: &qarqa_network_building::FundFlowNetwork) -> HashMap<Address, String> {
    let mut labels = HashMap::new();
    
    // Calculate degree for each node to assign meaningful labels
    let mut degrees: HashMap<Address, usize> = HashMap::new();
    
    for edge in &network.edges {
        *degrees.entry(edge.from).or_insert(0) += 1;
        *degrees.entry(edge.to).or_insert(0) += 1;
    }
    
    // Assign labels based on characteristics
    for (address, _node) in &network.nodes {
        let degree = degrees.get(address).unwrap_or(&0);
        
        let label = match degree {
            0 => "Isolated".to_string(),
            1 => "Endpoint".to_string(),
            2..=3 => "Regular".to_string(),
            4..=6 => "Hub".to_string(),
            _ => "Major Hub".to_string(),
        };
        
        labels.insert(*address, label);
    }
    
    labels
}

fn get_node_letter(network: &qarqa_network_building::FundFlowNetwork, address: Address) -> char {
    for (i, (node_address, _node)) in network.nodes.iter().enumerate() {
        if *node_address == address {
            return char::from(b'A' + i as u8);
        }
    }
    '?'
}

fn create_simple_ascii_layout(network: &qarqa_network_building::FundFlowNetwork, labels: &HashMap<Address, String>) {
    // Find central node (highest degree)
    let mut node_degrees: Vec<_> = network.nodes.iter()
        .map(|(address, _node)| {
            let degree = network.edges.iter()
                .filter(|e| e.from == *address || e.to == *address)
                .count();
            (*address, degree)
        })
        .collect();
    
    node_degrees.sort_by(|a, b| b.1.cmp(&a.1));
    
    if let Some((central_node, _)) = node_degrees.first() {
        let central_letter = get_node_letter(network, *central_node);
        let default_label = "Central".to_string();
        let central_label = labels.get(central_node).unwrap_or(&default_label);
        
        println!("     Central Node: [{}] {} ({})", central_letter, format_address(*central_node), central_label);
        println!();
        
        // Show connections to central node
        let connections: Vec<_> = network.edges.iter()
            .filter(|e| e.from == *central_node || e.to == *central_node)
            .collect();
        
        if connections.len() <= 8 { // Only show if manageable
            println!("            Connected Nodes:");
            for edge in connections.iter() {
                let other_node = if edge.from == *central_node { edge.to } else { edge.from };
                let other_letter = get_node_letter(network, other_node);
                let direction = if edge.from == *central_node { "→" } else { "←" };
                
                println!("         [{}] {} {} [{}]",
                         central_letter,
                         direction,
                         format!("{:.1}ETH", edge.total_eth),
                         other_letter);
            }
        }
    }
}

fn generate_dot_format(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("4. DOT Format (Graphviz) Output:");
    println!();
    
    let mut dot_output = String::new();
    dot_output.push_str("digraph FundFlowNetwork {\n");
    dot_output.push_str("    rankdir=LR;\n");
    dot_output.push_str("    node [shape=circle, style=filled];\n");
    dot_output.push_str("    edge [fontsize=8];\n\n");
    
    // Add nodes with styling based on their characteristics
    for (i, (address, _node)) in network.nodes.iter().enumerate() {
        let node_id = format!("node{}", i);
        let label = format!("{}\\n{}", 
                           char::from(b'A' + i as u8),
                           &format_address(*address)[..10]);
        
        // Calculate node importance (degree)
        let degree = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .count();
        
        let (color, size) = match degree {
            0..=1 => ("lightblue", "0.5"),
            2..=3 => ("yellow", "0.7"),
            4..=6 => ("orange", "1.0"),
            _ => ("red", "1.2"),
        };
        
        dot_output.push_str(&format!("    {} [label=\"{}\", fillcolor={}, width={}];\n",
                                   node_id, label, color, size));
    }
    
    dot_output.push_str("\n");
    
    // Add edges with weights
    for edge in &network.edges {
        let from_id = get_node_id(network, edge.from);
        let to_id = get_node_id(network, edge.to);
        
        let weight = (wei_to_eth(edge.total_eth) * 2.0).max(0.5).min(5.0);
        let color = if edge.flow_types.is_empty() {
            "black"
        } else {
            match &edge.flow_types[0] {
                FlowType::DirectTransfer => "black",
                FlowType::InternalTransfer => "blue", 
                FlowType::TokenTransfer(_) => "green",
                FlowType::ContractInteraction => "purple",
                FlowType::GasPayment => "gray",
            }
        };
        
        dot_output.push_str(&format!("    {} -> {} [label=\"{:.1}\", penwidth={:.1}, color={}];\n",
                                   from_id, to_id, wei_to_eth(edge.total_eth), weight, color));
    }
    
    dot_output.push_str("}\n");
    
    println!("   DOT Graph Definition:");
    println!("   =====================");
    println!("{}", dot_output);
    
    println!("   Usage: Save this to a .dot file and run:");
    println!("     dot -Tpng graph.dot -o graph.png");
    println!("     dot -Tsvg graph.dot -o graph.svg");
    println!();
    
    Ok(())
}

fn get_node_id(network: &qarqa_network_building::FundFlowNetwork, address: Address) -> String {
    for (i, (node_address, _node)) in network.nodes.iter().enumerate() {
        if *node_address == address {
            return format!("node{}", i);
        }
    }
    "unknown".to_string()
}

fn generate_json_format(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("5. JSON Format (D3.js/Cytoscape.js compatible):");
    println!();
    
    // Create nodes array
    let mut nodes_json = String::from("  \"nodes\": [\n");
    for (i, (address, _node)) in network.nodes.iter().enumerate() {
        let degree = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .count();
        
        let total_flow: f64 = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .map(|e| wei_to_eth(e.total_eth))
            .sum();
        
        nodes_json.push_str(&format!(
            "    {{ \"id\": \"{}\", \"address\": \"{}\", \"degree\": {}, \"totalFlow\": {} }}",
            format_address(*address),
            address,
            degree,
            total_flow
        ));
        
        if i < network.nodes.len() - 1 {
            nodes_json.push(',');
        }
        nodes_json.push('\n');
    }
    nodes_json.push_str("  ]");
    
    // Create edges array
    let mut edges_json = String::from("  \"edges\": [\n");
    for (i, edge) in network.edges.iter().enumerate() {
        let flow_type_str = if edge.flow_types.is_empty() {
            "Unknown".to_string()
        } else {
            format!("{:?}", edge.flow_types[0])
        };
        
        edges_json.push_str(&format!(
            "    {{ \"source\": \"{}\", \"target\": \"{}\", \"weight\": {}, \"type\": \"{}\", \"transactions\": {} }}",
            format_address(edge.from),
            format_address(edge.to),
            wei_to_eth(edge.total_eth),
            flow_type_str,
            edge.transaction_count
        ));
        
        if i < network.edges.len() - 1 {
            edges_json.push(',');
        }
        edges_json.push('\n');
    }
    edges_json.push_str("  ]");
    
    let json_output = format!("{{\n{},\n{}\n}}", nodes_json, edges_json);
    
    println!("   JSON Graph Data:");
    println!("   ================");
    println!("{}", json_output);
    
    println!("\n   Usage with D3.js force simulation:");
    println!("     const simulation = d3.forceSimulation(data.nodes)");
    println!("       .force(\"link\", d3.forceLink(data.edges).id(d => d.id))");
    println!("       .force(\"charge\", d3.forceManyBody())");
    println!("       .force(\"center\", d3.forceCenter(width / 2, height / 2));");
    println!();
    
    Ok(())
}

fn generate_adjacency_matrix(network: &qarqa_network_building::FundFlowNetwork) {
    println!("6. Adjacency Matrix:");
    println!();
    
    let n = network.nodes.len();
    if n > 10 {
        println!("   Matrix too large to display ({} x {})", n, n);
        println!("   Consider using a smaller network for visualization");
        return;
    }
    
    // Create adjacency matrix
    let mut matrix = vec![vec![0.0_f64; n]; n];
    
    for edge in &network.edges {
        let from_idx = network.nodes.iter().position(|(addr, _)| *addr == edge.from);
        let to_idx = network.nodes.iter().position(|(addr, _)| *addr == edge.to);
        
        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            matrix[from][to] = wei_to_eth(edge.total_eth);
        }
    }
    
    // Print matrix with labels
    print!("       ");
    for i in 0..n {
        print!("    {} ", char::from(b'A' + i as u8));
    }
    println!();
    
    for i in 0..n {
        print!("   {} ", char::from(b'A' + i as u8));
        for j in 0..n {
            if matrix[i][j] > 0.0 {
                print!(" {:4.1} ", matrix[i][j]);
            } else {
                print!("   - ");
            }
        }
        println!();
    }
    
    println!();
    println!("   Legend: Rows = From, Columns = To, Values = ETH Amount");
    println!();
}

fn generate_force_layout_data(network: &qarqa_network_building::FundFlowNetwork) -> QarqaResult<()> {
    println!("7. Force-Directed Layout Data:");
    println!();
    
    // Calculate suggested positions using a simple force-directed algorithm
    let n = network.nodes.len();
    if n == 0 {
        println!("   No nodes to layout");
        return Ok(());
    }
    
    // Initialize positions randomly in a circle
    let center_x = 50.0;
    let center_y = 50.0;
    let radius = 30.0;
    
    println!("   Suggested Node Positions (for visualization):");
    println!("   =============================================");
    
    for (i, (address, _node)) in network.nodes.iter().enumerate() {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        
        // Calculate node importance for sizing
        let degree = network.edges.iter()
            .filter(|e| e.from == *address || e.to == *address)
            .count();
        
        let size = 5 + (degree * 2);
        
        println!("   Node {}: x={:.1}, y={:.1}, size={} (degree={})",
                 char::from(b'A' + i as u8),
                 x, y, size, degree);
    }
    
    println!();
    println!("   Layout Algorithm Suggestions:");
    println!("   - Use force-directed algorithms (Fruchterman-Reingold, Force Atlas)");
    println!("   - Apply edge bundling for dense networks");
    println!("   - Use community detection for node coloring");
    println!("   - Implement zoom and pan for large networks");
    println!("   - Add node filtering by flow amount or type");
    println!();
    
    Ok(())
}

fn demonstrate_visualization_concepts() {
    println!("8. Visualization Concepts and Best Practices:");
    println!();
    
    println!("   Visual Encoding Strategies:");
    println!("   ==========================");
    println!("   • Node Size: Proportional to transaction volume or degree centrality");
    println!("   • Node Color: Entity type, risk level, or community membership");
    println!("   • Edge Width: Proportional to ETH amount transferred");
    println!("   • Edge Color: Flow type (direct, internal, token, gas, contract)");
    println!("   • Edge Style: Solid for successful transactions, dashed for failed");
    println!();
    
    println!("   Layout Algorithms:");
    println!("   ==================");
    println!("   • Force-Directed: Good for general network exploration");
    println!("   • Hierarchical: Useful when there's a clear flow direction");
    println!("   • Circular: Effective for showing cyclical patterns");
    println!("   • Arc Diagrams: Compact representation for large networks");
    println!("   • Matrix View: Alternative for dense networks");
    println!();
    
    println!("   Interactive Features:");
    println!("   ====================");
    println!("   • Hover: Show detailed transaction information");
    println!("   • Click: Highlight connected nodes and edges");
    println!("   • Filter: By time range, amount, flow type, or risk level");
    println!("   • Search: Find specific addresses or transaction hashes");
    println!("   • Zoom: Navigate large networks effectively");
    println!();
    
    println!("   Performance Considerations:");
    println!("   ===========================");
    println!("   • Level of Detail: Show less detail when zoomed out");
    println!("   • Edge Bundling: Reduce visual clutter in dense areas");
    println!("   • Clustering: Group similar nodes for better performance");
    println!("   • Pagination: Load network data incrementally");
    println!("   • WebGL: Use hardware acceleration for large datasets");
    println!();
    
    println!("   Risk Visualization:");
    println!("   ===================");
    println!("   • Heat Maps: Show risk levels across the network");
    println!("   • Alert Highlighting: Emphasize suspicious patterns");
    println!("   • Time Animation: Show how risk patterns evolve");
    println!("   • Path Highlighting: Trace fund flows through the network");
    println!("   • Comparison Views: Before/after analysis results");
}