//! Basic usage example for fundflownetwork
//! Uses real addresses from MEXC → Creator → Pool flow

use alloy_primitives::{Address, U256};
use std::str::FromStr;
use tx_fund_flow_core_types::{CompleteFundFlows, EthMovement, EthMovementType};
use tx_fund_flow_fundflownetwork::{
    CytoscapeExporter, FundFlowAnalyzer, NetworkBuilder, VisJsExporter,
};

fn main() {
    println!("Fund Flow Network Example");

    // Create test transaction data
    let flows = create_test_flows();

    // Analyze fund flows
    let analyzer = FundFlowAnalyzer::new().with_min_value(U256::from(1_000_000_000_000_000u128)); // 0.001 ETH minimum

    let fund_flows = analyzer.analyze_fund_flows(&flows);
    println!("\nExtracted {} unique fund flows:", fund_flows.len());

    for flow in &fund_flows {
        println!(
            "  {} → {}: {:.4} ETH",
            format!("{:?}", flow.from)[..10].to_string(),
            format!("{:?}", flow.to)[..10].to_string(),
            flow.amount_eth
        );
    }

    // Calculate net balances
    let balances = analyzer.calculate_net_balances(&fund_flows);
    println!("\nNet balances:");
    for (addr, balance) in &balances {
        if balance.net_eth.abs() > 0.0 {
            println!(
                "  {}: {:+.4} ETH",
                format!("{:?}", addr)[..10].to_string(),
                balance.net_eth
            );
        }
    }

    // Build network
    let builder = NetworkBuilder::new().with_min_eth_amount(0.01);

    let network = builder.build_from_flows(&fund_flows);

    println!("\nNetwork statistics:");
    let stats = network.calculate_stats();
    println!("  Nodes: {}", stats.node_count);
    println!("  Edges: {}", stats.edge_count);
    println!("  Total volume: {:.4} ETH", stats.total_volume_eth);

    // Export for visualization
    let cytoscape = CytoscapeExporter::export(&network).unwrap();
    std::fs::write(
        "network_cytoscape.json",
        serde_json::to_string_pretty(&cytoscape).unwrap(),
    )
    .unwrap();

    let visjs = VisJsExporter::export(&network).unwrap();
    std::fs::write(
        "network_visjs.json",
        serde_json::to_string_pretty(&visjs).unwrap(),
    )
    .unwrap();

    println!("\n✅ Exported network to network_cytoscape.json and network_visjs.json");
}

fn create_test_flows() -> Vec<CompleteFundFlows> {
    // Real checksummed addresses from MEXC → Creator → Pool flow
    let mexc = Address::from_str("0x9642b23Ed1E01Df1092B92641051881a322F5D4E").unwrap(); // MEXC 16
    let creator = Address::from_str("0xC04B517E75907965AD59976c63912C8C8af97D96").unwrap(); // Pool Creator
    let router = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap(); // Uniswap V2: Router 2
    let pool = Address::from_str("0x0e9797F0f05A3dE8384D76467E98DA03874c86a6").unwrap(); // Uniswap V2: ByteBond
    let weth = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(); // Wrapped Ether

    vec![
        // Transaction 1: MEXC withdrawal to Creator
        CompleteFundFlows {
            tx_hash: "0xaa8b115c150332e9d049ef513636b3031f40f28a8f13534c9050aa08c93ddd32"
                .to_string(),
            block_number: 22885482,
            from_address: mexc,
            to_address: Some(creator),
            eth_movements: vec![EthMovement {
                from: mexc,
                to: creator,
                amount: U256::from(1_399_850_000_000_000_000u128), // 1.39985 ETH
                movement_type: EthMovementType::Direct,
            }],
            token_movements: Vec::new(),
        },
        // Transaction 2: Pool creation and liquidity addition
        CompleteFundFlows {
            tx_hash: "0x5e439aa276849d6ba592b5bce910fefd79c0d26b9d185d95f367848d6a1f6164"
                .to_string(),
            block_number: 22885510,
            from_address: creator,
            to_address: Some(router),
            eth_movements: vec![
                // Creator sends 1 ETH to Router
                EthMovement {
                    from: creator,
                    to: router,
                    amount: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
                    movement_type: EthMovementType::Direct,
                },
                // Router wraps ETH to WETH
                EthMovement {
                    from: router,
                    to: weth,
                    amount: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
                    movement_type: EthMovementType::Internal,
                },
                // WETH flows to Pool (internal transfer)
                EthMovement {
                    from: weth,
                    to: pool,
                    amount: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
                    movement_type: EthMovementType::Internal,
                },
            ],
            token_movements: Vec::new(),
        },
        // Transaction 3: Example of token swap (common pattern)
        CompleteFundFlows {
            tx_hash: "0x7f5c356a6a215e969630a885933680d5c49545c80bb785ab1b7b6ca1cfef757b"
                .to_string(),
            block_number: 22885530,
            from_address: creator,
            to_address: Some(router),
            eth_movements: vec![EthMovement {
                from: creator,
                to: router,
                amount: U256::from(100_000_000_000_000_000u128), // 0.1 ETH
                movement_type: EthMovementType::Direct,
            }],
            token_movements: Vec::new(),
        },
    ]
}
