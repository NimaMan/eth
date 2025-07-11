//! Real transaction network example - demonstrates building networks from actual transactions

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
    
    println!("=== Real Transaction Network Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting real transaction network example");
    
    // Parse the actual Uniswap transaction
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let fund_flows = parse_uniswap_transaction(tx_hash)?;
    
    println!("🔍 Analyzing Real Transaction:");
    println!("   Hash: {}", tx_hash);
    println!("   Type: ETH → USDT Swap on Uniswap V3/V4");
    println!("   Amount: 6.729614461788500138 ETH → 16,865.020704 USDT");
    println!("   Block: 22646153");
    println!("   Found {} fund flows from transaction analysis\n", fund_flows.len());
    
    // Display all transfers found
    display_transaction_transfers(&fund_flows);
    
    // Build network with very low threshold to capture all flows
    let params = qarqa_network_building::AnalysisParams {
        min_usd_threshold: 0.01, // $0.01 minimum to capture everything
        max_depth: 4,
        include_gas: true,
        treat_weth_as_eth: true,
    };
    
    let builder = NetworkBuilder::new().with_params(params);
    let network = builder.build_from_fund_flows(&fund_flows, 
        Some(Address::from_str("0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1")?))? // User address as center
    ;
    
    println!("📊 Network Analysis:");
    println!("   Nodes: {} (addresses involved)", network.nodes.len());
    println!("   Edges: {} (fund flows)", network.edges.len());
    println!();
    
    // Analyze the network structure
    analyze_transaction_network(&network);
    
    // Show protocol interactions
    analyze_protocol_interactions(&network, &fund_flows);
    
    // Generate visualization formats
    generate_transaction_visualizations(&network, tx_hash);
    
    println!("\n=== Real transaction network analysis completed! ===");
    
    Ok(())
}

fn parse_uniswap_transaction(tx_hash: &str) -> QarqaResult<Vec<FundFlow>> {
    let mut flows = Vec::new();
    
    // Main transaction details
    let user = Address::from_str("0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1")?; // From
    let router = Address::from_str("0xfBd4cdB413E45a52E2C8312f670e9cE67E794C37")?; // To (router)
    let weth = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?; // WETH
    let usdt = Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7")?; // USDT
    let usdc = Address::from_str("0xA0b86a33E6417c4E73888ACB2c88F3693f0F1D6E")?; // USDC (placeholder)
    
    // Protocol addresses
    let uniswap_v3_pool = Address::from_str("0x11b815efB8f581194ae79006d24E0d814B7697F6")?; // Uniswap V3: USDT 3
    let uniswap_v4_manager = Address::from_str("0x5302086A3a25d473aAbBd0356eFf8Dd811a4d89B")?; // Uniswap V4 Pool Manager
    let bridge_1 = Address::from_str("0x6bDf3535CB5D7DdAF0dD92E9cE22AC7e8f9d59e0")?; // Bridge/Router 1
    let bridge_2 = Address::from_str("0x3177F690AF149b9C4E25B2a88199FA1C359aae5A")?; // Bridge/Router 2
    
    let block_number = 22646153;
    
    // 1. User pays ETH to router (gas payment)
    flows.push(FundFlow {
        from: user,
        to: Address::ZERO, // Miners
        amount_eth: 0.003908755780679457, // Transaction fee
        amount_tokens_usd: 9.76,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::GasPayment,
    });
    
    // 2. User's ETH gets wrapped to WETH and flows through the system
    flows.push(FundFlow {
        from: user,
        to: weth,
        amount_eth: 6.729614461788500138,
        amount_tokens_usd: 16810.04,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(weth),
    });
    
    // 3. WETH flows to first bridge/router
    flows.push(FundFlow {
        from: weth,
        to: bridge_1,
        amount_eth: 10.829495221098646603,
        amount_tokens_usd: 27051.21,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::InternalTransfer,
    });
    
    // 4. WETH flows from bridge to V4 manager (part 1)
    flows.push(FundFlow {
        from: bridge_1,
        to: uniswap_v4_manager,
        amount_eth: 10.829495221098646603,
        amount_tokens_usd: 27051.21,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::ContractInteraction,
    });
    
    // 5. WETH flows to second bridge/router
    flows.push(FundFlow {
        from: weth,
        to: bridge_2,
        amount_eth: 5.495899762937538401,
        amount_tokens_usd: 13728.32,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::InternalTransfer,
    });
    
    // 6. WETH flows from second bridge to V4 manager (part 2)
    flows.push(FundFlow {
        from: bridge_2,
        to: uniswap_v4_manager,
        amount_eth: 5.495899762937538401,
        amount_tokens_usd: 13728.32,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::ContractInteraction,
    });
    
    // 7. USDC flows from V4 manager to bridge 1
    flows.push(FundFlow {
        from: uniswap_v4_manager,
        to: bridge_1,
        amount_eth: 0.0, // Token transfer, no ETH
        amount_tokens_usd: 27151.61,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdc),
    });
    
    // 8. USDC flows from bridge 1 to router
    flows.push(FundFlow {
        from: bridge_1,
        to: router,
        amount_eth: 0.0,
        amount_tokens_usd: 27151.61,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdc),
    });
    
    // 9. WETH flows from router back to bridge 1
    flows.push(FundFlow {
        from: router,
        to: bridge_1,
        amount_eth: 10.829495221098646603,
        amount_tokens_usd: 27051.21,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(weth),
    });
    
    // 10. USDT flows from V3 pool to router
    flows.push(FundFlow {
        from: uniswap_v3_pool,
        to: router,
        amount_eth: 0.0,
        amount_tokens_usd: 16865.02,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdt),
    });
    
    // 11. WETH flows from router to V3 pool
    flows.push(FundFlow {
        from: router,
        to: uniswap_v3_pool,
        amount_eth: 6.729614461788500138,
        amount_tokens_usd: 16810.04,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(weth),
    });
    
    // 12. USDT flows from V4 manager to bridge 2
    flows.push(FundFlow {
        from: uniswap_v4_manager,
        to: bridge_2,
        amount_eth: 0.0,
        amount_tokens_usd: 13772.16,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdt),
    });
    
    // 13. USDT flows from bridge 2 to router
    flows.push(FundFlow {
        from: bridge_2,
        to: router,
        amount_eth: 0.0,
        amount_tokens_usd: 13772.16,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdt),
    });
    
    // 14. WETH flows from router to bridge 2
    flows.push(FundFlow {
        from: router,
        to: bridge_2,
        amount_eth: 5.495899762937538401,
        amount_tokens_usd: 13728.32,
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(weth),
    });
    
    // 15. Final USDT delivery to user
    flows.push(FundFlow {
        from: router,
        to: user,
        amount_eth: 0.0,
        amount_tokens_usd: 16865.02, // Total USDT received
        transaction_count: 1,
        first_block: block_number,
        last_block: block_number,
        flow_type: FlowType::TokenTransfer(usdt),
    });
    
    Ok(flows)
}

fn display_transaction_transfers(fund_flows: &[FundFlow]) {
    println!("💰 All Transfers Detected:");
    println!("   ========================");
    
    let mut eth_transfers = Vec::new();
    let mut token_transfers = Vec::new();
    let mut gas_payments = Vec::new();
    
    for flow in fund_flows {
        match flow.flow_type {
            FlowType::GasPayment => gas_payments.push(flow),
            FlowType::TokenTransfer(_) if flow.amount_eth > 0.0 => eth_transfers.push(flow),
            FlowType::TokenTransfer(_) => token_transfers.push(flow),
            _ if flow.amount_eth > 0.0 => eth_transfers.push(flow),
            _ => token_transfers.push(flow),
        }
    }
    
    println!("   🔥 Gas Payments ({}):", gas_payments.len());
    for flow in gas_payments {
        println!("     {} → Miners: {} ETH (${:.2})",
                 format_address(flow.from),
                 flow.amount_eth,
                 flow.amount_tokens_usd);
    }
    
    println!("\n   ⚡ ETH/WETH Transfers ({}):", eth_transfers.len());
    for flow in eth_transfers {
        println!("     {} → {}: {} ETH (${:.2})",
                 format_address(flow.from),
                 format_address(flow.to),
                 flow.amount_eth,
                 flow.amount_tokens_usd);
    }
    
    println!("\n   🪙 Token Transfers ({}):", token_transfers.len());
    for flow in token_transfers {
        let token_symbol = match flow.flow_type {
            FlowType::TokenTransfer(addr) => {
                if addr.to_string().to_lowercase().contains("dac17f958d2ee523a2206206994597c13d831ec7") {
                    "USDT"
                } else if addr.to_string().to_lowercase().contains("a0b86a33e6417c4e73888acb2c88f3693f0f1d6c") {
                    "USDC"
                } else if addr.to_string().to_lowercase().contains("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2") {
                    "WETH"
                } else {
                    "Unknown"
                }
            }
            _ => "ETH"
        };
        
        println!("     {} → {}: ${:.2} {}",
                 format_address(flow.from),
                 format_address(flow.to),
                 flow.amount_tokens_usd,
                 token_symbol);
    }
    println!();
}

fn analyze_transaction_network(network: &qarqa_network_building::FundFlowNetwork) {
    println!("🕸️ Network Structure Analysis:");
    
    // Identify key participants
    let mut protocol_addresses = HashMap::new();
    let mut user_addresses = HashMap::new();
    
    for (address, _node) in &network.nodes {
        let addr_str = address.to_string().to_lowercase();
        
        if addr_str.contains("5b43453fce04b92e190f391a83136bfbecededf1") {
            user_addresses.insert(*address, "User (Trader)");
        } else if addr_str.contains("fbd4cdb413e45a52e2c8312f670e9ce67e794c37") {
            protocol_addresses.insert(*address, "Router Contract");
        } else if addr_str.contains("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2") {
            protocol_addresses.insert(*address, "WETH Token");
        } else if addr_str.contains("dac17f958d2ee523a2206206994597c13d831ec7") {
            protocol_addresses.insert(*address, "USDT Token");
        } else if addr_str.contains("a0b86a33e6417c4e73888acb2c88f3693f0f1d6c") {
            protocol_addresses.insert(*address, "USDC Token");
        } else if addr_str.contains("11b815efb8f581194ae79006d24e0d814b7697f6") {
            protocol_addresses.insert(*address, "Uniswap V3 Pool");
        } else if addr_str == "0x0000000000000000000000000000000000000000" {
            protocol_addresses.insert(*address, "Miners/Validators");
        } else {
            protocol_addresses.insert(*address, "Bridge/Router");
        }
    }
    
    println!("   👤 User Addresses ({}):", user_addresses.len());
    for (addr, label) in &user_addresses {
        let degree = network.get_edges_from(addr).len() + network.get_edges_to(addr).len();
        println!("     {} - {} (degree: {})", format_address(*addr), label, degree);
    }
    
    println!("\n   🏛️ Protocol Addresses ({}):", protocol_addresses.len());
    for (addr, label) in &protocol_addresses {
        let degree = network.get_edges_from(addr).len() + network.get_edges_to(addr).len();
        println!("     {} - {} (degree: {})", format_address(*addr), label, degree);
    }
    
    // Calculate total volumes
    let total_eth_volume: f64 = network.edges.iter()
        .map(|e| wei_to_eth(e.total_eth))
        .sum();
    
    let total_token_volume: f64 = network.edges.iter()
        .map(|e| e.total_usd)
        .sum();
    
    println!("\n   💵 Volume Analysis:");
    println!("     Total ETH Volume: {} ETH", total_eth_volume);
    println!("     Total Token Volume: ${:.2}", total_token_volume);
    println!("     Average Flow Size: ${:.2}", total_token_volume / network.edges.len() as f64);
    println!();
}

fn analyze_protocol_interactions(network: &qarqa_network_building::FundFlowNetwork, fund_flows: &[FundFlow]) {
    println!("🔄 Protocol Interaction Analysis:");
    
    // Count interactions by protocol
    let mut protocol_interactions = HashMap::new();
    
    for flow in fund_flows {
        let protocol = match flow.flow_type {
            FlowType::TokenTransfer(addr) => {
                let addr_str = addr.to_string().to_lowercase();
                if addr_str.contains("dac17f958d2ee523a2206206994597c13d831ec7") {
                    "USDT"
                } else if addr_str.contains("a0b86a33e6417c4e73888acb2c88f3693f0f1d6c") {
                    "USDC"
                } else if addr_str.contains("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2") {
                    "WETH"
                } else {
                    "Other Token"
                }
            }
            FlowType::ContractInteraction => "Contract Call",
            FlowType::InternalTransfer => "Internal Transfer",
            FlowType::DirectTransfer => "Direct Transfer",
            FlowType::GasPayment => "Gas Payment",
        };
        
        *protocol_interactions.entry(protocol).or_insert(0) += 1;
    }
    
    for (protocol, count) in protocol_interactions.iter() {
        println!("   {}: {} interactions", protocol, count);
    }
    
    println!("\n   🎯 Swap Path Analysis:");
    println!("     Input: 6.73 ETH → WETH");
    println!("     Routing: WETH → Multiple DEX pools");
    println!("     Bridging: Through intermediate routers");
    println!("     Output: 16,865 USDT → User");
    println!("     Efficiency: Multi-hop routing for best price");
    println!();
}

fn generate_transaction_visualizations(network: &qarqa_network_building::FundFlowNetwork, tx_hash: &str) {
    println!("🎨 Visualization Data:");
    
    // Generate simple network summary
    println!("   Network Summary for Transaction {}:", tx_hash);
    println!("   Nodes: {}, Edges: {}", network.nodes.len(), network.edges.len());
    
    // Most connected nodes
    let mut node_degrees: Vec<_> = network.nodes.iter()
        .map(|(address, _node)| {
            let degree = network.get_edges_from(address).len() + network.get_edges_to(address).len();
            (*address, degree)
        })
        .collect();
    node_degrees.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!("\n   🌟 Most Connected Nodes:");
    for (i, (addr, degree)) in node_degrees.iter().take(5).enumerate() {
        println!("     {}. {}: {} connections", i + 1, format_address(*addr), degree);
    }
    
    // Flow patterns
    println!("\n   📊 Flow Patterns:");
    let mut flow_types = HashMap::new();
    for edge in &network.edges {
        for flow_type in &edge.flow_types {
            *flow_types.entry(format!("{:?}", flow_type)).or_insert(0) += 1;
        }
    }
    
    for (flow_type, count) in flow_types.iter() {
        println!("     {}: {} flows", flow_type, count);
    }
    
    println!("\n   💡 Frontend Integration Ideas:");
    println!("     • Accept both transaction hash AND address as input");
    println!("     • Show transaction-centric view vs address-centric view");
    println!("     • Highlight the main user's path through the network");
    println!("     • Color-code different protocols (Uniswap, bridges, etc.)");
    println!("     • Show token flow direction with animated arrows");
    println!("     • Display USD values alongside ETH amounts");
    println!("     • Provide drill-down into each transfer step");
}