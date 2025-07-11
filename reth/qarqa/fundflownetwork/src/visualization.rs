//! Visualization exporters for fund flow networks

use crate::network_types::*;
use serde_json::json;
use std::collections::HashMap;
use eyre::Result;

/// Cytoscape.js format exporter
pub struct CytoscapeExporter;

impl CytoscapeExporter {
    /// Export network to Cytoscape.js JSON format
    pub fn export(network: &FundFlowNetwork) -> Result<serde_json::Value> {
        let mut elements = Vec::new();
        
        // Add nodes
        for (address, node) in &network.nodes {
            let node_color = match node.node_type {
                NodeType::CEX => "#ff6b6b",      // Red
                NodeType::DEX => "#4ecdc4",      // Teal
                NodeType::DeFi => "#45b7d1",     // Blue
                NodeType::Contract => "#f39c12",  // Orange
                NodeType::EOA => "#95a5a6",      // Gray
                NodeType::Token => "#9b59b6",    // Purple
                NodeType::Unknown => "#7f8c8d",  // Dark gray
            };
            
            let size = 30.0 + (node.balance_change.abs().log10().max(0.0) * 10.0);
            
            elements.push(json!({
                "data": {
                    "id": format!("{:?}", address),
                    "label": node.label.as_ref().unwrap_or(&format!("{:?}", address)),
                    "balance_change": node.balance_change,
                    "usd_change": node.usd_value_change,
                    "tx_count": node.transaction_count,
                    "node_type": format!("{:?}", node.node_type),
                },
                "classes": format!("{:?}", node.node_type).to_lowercase(),
                "style": {
                    "background-color": node_color,
                    "width": size,
                    "height": size,
                }
            }));
        }
        
        // Add edges
        for edge in &network.edges {
            let width = 1.0 + (edge.eth_amount.log10().max(0.0) * 2.0);
            let opacity = 0.3 + (edge.transaction_count as f64 / 100.0).min(0.7);
            
            elements.push(json!({
                "data": {
                    "id": format!("{:?}-{:?}", edge.from, edge.to),
                    "source": format!("{:?}", edge.from),
                    "target": format!("{:?}", edge.to),
                    "eth_amount": edge.eth_amount,
                    "tx_count": edge.transaction_count,
                    "edge_type": format!("{:?}", edge.edge_type),
                },
                "classes": format!("{:?}", edge.edge_type).to_lowercase(),
                "style": {
                    "width": width,
                    "opacity": opacity,
                    "line-color": "#34495e",
                    "target-arrow-color": "#34495e",
                    "target-arrow-shape": "triangle",
                    "curve-style": "bezier"
                }
            }));
        }
        
        Ok(json!({
            "elements": elements,
            "layout": {
                "name": "cose",
                "animate": false,
                "nodeRepulsion": 400000,
                "idealEdgeLength": 100,
                "edgeElasticity": 100,
                "nestingFactor": 5,
                "gravity": 80,
                "numIter": 1000,
                "initialTemp": 200,
                "coolingFactor": 0.95,
                "minTemp": 1.0
            },
            "style": [
                {
                    "selector": "node",
                    "style": {
                        "label": "data(label)",
                        "text-valign": "center",
                        "text-halign": "center",
                        "font-size": "12px",
                        "font-weight": "bold",
                        "text-outline-width": 2,
                        "text-outline-color": "#ffffff",
                        "border-width": 2,
                        "border-color": "#2c3e50"
                    }
                },
                {
                    "selector": "edge",
                    "style": {
                        "label": "data(eth_amount)",
                        "font-size": "10px",
                        "text-rotation": "autorotate",
                        "text-margin-y": -10
                    }
                }
            ]
        }))
    }
}

/// vis.js network format exporter
pub struct VisJsExporter;

impl VisJsExporter {
    /// Export network to vis.js format
    pub fn export(network: &FundFlowNetwork) -> Result<serde_json::Value> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        
        // Create ID mapping
        let mut id_map = HashMap::new();
        for (i, address) in network.nodes.keys().enumerate() {
            id_map.insert(address, i);
        }
        
        // Add nodes
        for (address, node) in &network.nodes {
            let node_id = id_map[&address];
            
            let color = match node.node_type {
                NodeType::CEX => "#ff6b6b",
                NodeType::DEX => "#4ecdc4",
                NodeType::DeFi => "#45b7d1",
                NodeType::Contract => "#f39c12",
                NodeType::EOA => "#95a5a6",
                NodeType::Token => "#9b59b6",
                NodeType::Unknown => "#7f8c8d",
            };
            
            let size = 10.0 + (node.balance_change.abs().log10().max(0.0) * 5.0);
            
            nodes.push(json!({
                "id": node_id,
                "label": node.label.as_ref()
                    .unwrap_or(&format!("{:?}", address))
                    .chars()
                    .take(20)
                    .collect::<String>(),
                "title": format!(
                    "Address: {:?}\nType: {:?}\nBalance Change: {:.4} ETH\nTransactions: {}",
                    address, node.node_type, node.balance_change, node.transaction_count
                ),
                "color": color,
                "size": size,
                "shape": if node.is_contract { "box" } else { "circle" },
                "font": {
                    "size": 12,
                    "color": "#2c3e50"
                }
            }));
        }
        
        // Add edges
        for edge in &network.edges {
            if let (Some(&from_id), Some(&to_id)) = (id_map.get(&edge.from), id_map.get(&edge.to)) {
                let width = 1.0 + (edge.eth_amount.log10().max(0.0));
                
                edges.push(json!({
                    "from": from_id,
                    "to": to_id,
                    "label": format!("{:.3} ETH", edge.eth_amount),
                    "title": format!(
                        "Amount: {:.4} ETH\nTransactions: {}\nType: {:?}",
                        edge.eth_amount, edge.transaction_count, edge.edge_type
                    ),
                    "width": width,
                    "arrows": "to",
                    "color": {
                        "color": "#34495e",
                        "opacity": 0.6
                    },
                    "font": {
                        "size": 10,
                        "color": "#7f8c8d"
                    }
                }));
            }
        }
        
        Ok(json!({
            "nodes": nodes,
            "edges": edges,
            "options": {
                "physics": {
                    "enabled": true,
                    "solver": "forceAtlas2Based",
                    "forceAtlas2Based": {
                        "gravitationalConstant": -50,
                        "centralGravity": 0.01,
                        "springLength": 100,
                        "springConstant": 0.08,
                        "damping": 0.4
                    }
                },
                "interaction": {
                    "hover": true,
                    "navigationButtons": true,
                    "keyboard": true
                },
                "layout": {
                    "improvedLayout": true
                }
            }
        }))
    }
}

/// GraphML format exporter
pub struct GraphMLExporter;

impl GraphMLExporter {
    /// Export network to GraphML format
    pub fn export(network: &FundFlowNetwork) -> Result<String> {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<graphml xmlns="http://graphml.graphdrawing.org/xmlns"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns
         http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">
"#);
        
        // Define attributes
        xml.push_str(r#"  <key id="label" for="node" attr.name="label" attr.type="string"/>
  <key id="balance_change" for="node" attr.name="balance_change" attr.type="double"/>
  <key id="node_type" for="node" attr.name="node_type" attr.type="string"/>
  <key id="tx_count" for="node" attr.name="tx_count" attr.type="long"/>
  <key id="eth_amount" for="edge" attr.name="eth_amount" attr.type="double"/>
  <key id="edge_type" for="edge" attr.name="edge_type" attr.type="string"/>
  
  <graph id="G" edgedefault="directed">
"#);
        
        // Add nodes
        for (address, node) in &network.nodes {
            xml.push_str(&format!(
                r#"    <node id="{:?}">
      <data key="label">{}</data>
      <data key="balance_change">{}</data>
      <data key="node_type">{:?}</data>
      <data key="tx_count">{}</data>
    </node>
"#,
                address,
                node.label.as_ref().unwrap_or(&format!("{:?}", address)),
                node.balance_change,
                node.node_type,
                node.transaction_count
            ));
        }
        
        // Add edges
        for (i, edge) in network.edges.iter().enumerate() {
            xml.push_str(&format!(
                r#"    <edge id="e{}" source="{:?}" target="{:?}">
      <data key="eth_amount">{}</data>
      <data key="edge_type">{:?}</data>
    </edge>
"#,
                i, edge.from, edge.to, edge.eth_amount, edge.edge_type
            ));
        }
        
        xml.push_str("  </graph>\n</graphml>");
        
        Ok(xml)
    }
}