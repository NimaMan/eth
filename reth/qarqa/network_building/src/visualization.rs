//! Network visualization data conversion

use crate::network::*;
use serde::{Deserialize, Serialize};
use alloy_primitives::Address;

/// Frontend visualization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationData {
    pub nodes: Vec<VisualizationNode>,
    pub edges: Vec<VisualizationEdge>,
    pub stats: VisualizationStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationNode {
    pub id: String,
    pub label: String,
    pub color: String,
    pub size: u32,
    #[serde(rename = "nodeType")]
    pub node_type: String,
    #[serde(rename = "ethIn")]
    pub eth_in: String,
    #[serde(rename = "ethOut")]
    pub eth_out: String,
    #[serde(rename = "netChange")]
    pub net_change: String,
    #[serde(rename = "valueChange")]
    pub value_change: String,
    pub currency: String,
    #[serde(rename = "txCount")]
    pub tx_count: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub weight: u32,
    pub color: String,
    #[serde(rename = "ethAmount")]
    pub eth_amount: String,
    #[serde(rename = "txCount")]
    pub tx_count: String,
    #[serde(rename = "movementTypes")]
    pub movement_types: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationStats {
    #[serde(rename = "totalAddresses")]
    pub total_addresses: usize,
    #[serde(rename = "totalEdges")]
    pub total_edges: usize,
    #[serde(rename = "totalEthVolume")]
    pub total_eth_volume: String,
    #[serde(rename = "networkDensity")]
    pub network_density: String,
    pub currency: String,
}

/// Convert network to visualization format
pub fn convert_to_visualization_data(
    network: &FundFlowNetwork,
    center_address: Option<Address>,
    currency: NetworkCurrency,
) -> VisualizationData {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    
    // Calculate value ranges for sizing
    let value_changes: Vec<f64> = network.nodes.values()
        .map(|node| node.usd_value_change.abs())
        .collect();
    
    let max_change = value_changes.iter()
        .fold(0.0f64, |max, &val| if val > max { val } else { max })
        .max(1.0);
    
    // Convert nodes
    for (address, node) in &network.nodes {
        let normalized_change = (node.usd_value_change.abs() / max_change).min(1.0);
        let size = 20 + (40.0 * normalized_change) as u32;
        
        let color = if Some(*address) == center_address {
            "#e74c3c".to_string() // Red for center
        } else if node.usd_value_change > 0.0 {
            "#27ae60".to_string() // Green for gains
        } else if node.usd_value_change < 0.0 {
            "#e74c3c".to_string() // Red for losses
        } else {
            "#95a5a6".to_string() // Gray for neutral
        };
        
        let node_type_str = match &node.node_type {
            NodeType::ExternalAccount => "External Account",
            NodeType::Contract(_) => "Smart Contract",
            NodeType::Exchange => "Exchange",
            NodeType::Miner => "Miner",
            NodeType::Bridge => "Bridge",
            NodeType::Unknown => "Unknown",
        };
        
        nodes.push(VisualizationNode {
            id: format!("{:?}", address),
            label: format!("{:?}", address).chars().take(10).collect::<String>(),
            color,
            size,
            node_type: node_type_str.to_string(),
            eth_in: format!("{:.6}", wei_to_eth(node.total_eth_in)),
            eth_out: format!("{:.6}", wei_to_eth(node.total_eth_out)),
            net_change: format!("{:.6}", wei_to_eth_signed(node.net_eth_change)),
            value_change: format!("{:.2}", node.usd_value_change),
            currency: format!("{:?}", currency),
            tx_count: (node.incoming_tx_count + node.outgoing_tx_count).to_string(),
        });
    }
    
    // Convert edges
    for (i, edge) in network.edges.iter().enumerate() {
        let eth_amount = wei_to_eth(edge.total_eth);
        let weight = 3 + (eth_amount * 2.0).min(20.0) as u32;
        
        edges.push(VisualizationEdge {
            id: format!("e{}", i),
            source: format!("{:?}", edge.from),
            target: format!("{:?}", edge.to),
            weight,
            color: "#2980b9".to_string(),
            eth_amount: format!("{:.6}", eth_amount),
            tx_count: edge.transaction_count.to_string(),
            movement_types: "Direct".to_string(), // Simplified
            label: format!("{:.2} ETH", eth_amount),
        });
    }
    
    let stats = VisualizationStats {
        total_addresses: network.stats.node_count,
        total_edges: network.stats.edge_count,
        total_eth_volume: format!("{:.6}", wei_to_eth(network.stats.total_eth_volume)),
        network_density: format!("{:.4}", network.stats.network_density),
        currency: format!("{:?}", currency),
    };
    
    VisualizationData { nodes, edges, stats }
}

fn wei_to_eth(wei: alloy_primitives::U256) -> f64 {
    let wei_per_eth = alloy_primitives::U256::from(10).pow(alloy_primitives::U256::from(18));
    let eth_part = wei / wei_per_eth;
    let wei_remainder = wei % wei_per_eth;
    
    eth_part.to::<u64>() as f64 + (wei_remainder.to::<u64>() as f64 / 1e18)
}

fn wei_to_eth_signed(wei: i128) -> f64 {
    wei as f64 / 1e18
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_visualization_conversion() {
        let network = FundFlowNetwork::new();
        let viz_data = convert_to_visualization_data(&network, None, NetworkCurrency::USD);
        
        assert_eq!(viz_data.nodes.len(), 0);
        assert_eq!(viz_data.edges.len(), 0);
    }
}