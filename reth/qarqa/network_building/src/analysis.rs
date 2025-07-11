//! Network analysis utilities

use crate::network::*;
use alloy_primitives::Address;
use std::collections::{HashMap, HashSet};

/// Network analyzer
pub struct NetworkAnalyzer;

impl NetworkAnalyzer {
    /// Find shortest path between two nodes
    pub fn find_shortest_path(
        network: &FundFlowNetwork,
        from: Address,
        to: Address,
    ) -> Option<Vec<Address>> {
        // Simple BFS implementation
        let mut queue = vec![(from, vec![from])];
        let mut visited = HashSet::new();
        
        while let Some((current, path)) = queue.pop() {
            if current == to {
                return Some(path);
            }
            
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            
            for edge in network.get_edges_from(&current) {
                if !visited.contains(&edge.to) {
                    let mut new_path = path.clone();
                    new_path.push(edge.to);
                    queue.push((edge.to, new_path));
                }
            }
        }
        
        None
    }
    
    /// Calculate centrality measures
    pub fn calculate_centrality(network: &FundFlowNetwork) -> HashMap<Address, f64> {
        // Simple degree centrality
        let mut centrality = HashMap::new();
        
        for (address, _) in &network.nodes {
            let in_degree = network.get_edges_to(address).len();
            let out_degree = network.get_edges_from(address).len();
            let total_degree = in_degree + out_degree;
            
            centrality.insert(*address, total_degree as f64);
        }
        
        centrality
    }
    
    /// Detect cycles in the network
    pub fn detect_cycles(network: &FundFlowNetwork) -> Vec<Vec<Address>> {
        // Simple cycle detection using DFS
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for (address, _) in &network.nodes {
            if !visited.contains(address) {
                Self::dfs_cycles(
                    network,
                    *address,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                    &mut cycles,
                );
            }
        }
        
        cycles
    }
    
    fn dfs_cycles(
        network: &FundFlowNetwork,
        current: Address,
        visited: &mut HashSet<Address>,
        rec_stack: &mut HashSet<Address>,
        path: &mut Vec<Address>,
        cycles: &mut Vec<Vec<Address>>,
    ) {
        visited.insert(current);
        rec_stack.insert(current);
        path.push(current);
        
        for edge in network.get_edges_from(&current) {
            if let Some(pos) = path.iter().position(|&x| x == edge.to) {
                // Found cycle
                cycles.push(path[pos..].to_vec());
            } else if !visited.contains(&edge.to) {
                Self::dfs_cycles(network, edge.to, visited, rec_stack, path, cycles);
            }
        }
        
        path.pop();
        rec_stack.remove(&current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_analyzer() {
        let network = FundFlowNetwork::new();
        let centrality = NetworkAnalyzer::calculate_centrality(&network);
        assert_eq!(centrality.len(), 0);
    }
}