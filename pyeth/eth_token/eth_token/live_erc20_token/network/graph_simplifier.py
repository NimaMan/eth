"""
# Token Network Graph Simplifier
Simplifies token transaction networks by removing central nodes such as:
    - Token contract address
    - Liquidity pool contract address

Network Structure:
----------------
1. Nodes:
   - Represent addresses (wallets or contracts) with significant activity
   - Store UserTokenActivityTracker data containing:
     * Token balances and movements
     * Denomination (ETH) balances
     * If the address is a fee source
     * Transaction history
     * Profit/loss tracking

2. Edges:
   - Types:
     * 'txn owner': if address has been in the txn of another address, we have an edge from the txn owner to the address
   
3. Special Cases:
   - Zero Address
   - Dead Address


Simplification Process:
--------------------
1. High-Degree Node Filtering:
    - remove token contract address
    - remove liquidity pool contract address
    - any other high degree node like tax collector, etc.

2. Default Address Removal:
   - Zero Address (0x000...000)
   - Dead Address (0x000...dEaD)

4. Degree Calculation:
   - Adjusted Degree Computation
     * Accounts for both in and out edges
     * Excludes self-loops from degree count
     * Separate thresholds for wallets and contracts

5. Graph Maintenance:
   - Copy original graph
   - Remove filtered nodes
   - Maintain graph properties
   - Track removed nodes
"""

import networkx as nx
from typing import Dict


class GraphSimplifier:
    def __init__(self, token_network_obj, degree_threshold: int = 5):
        """
        Initialize graph simplifier with original graph and configuration.
        
        Args:
            graph: NetworkX DiGraph with UserTokenActivityTracker data
            degree_threshold: Base threshold for node degree filtering
        """
        self.token_network_obj = token_network_obj
        self.graph = token_network_obj.graph
        self.token_data = token_network_obj.token_data
        self.degree_threshold = degree_threshold

        # Initialize tracking sets
        self.simplified_graph = None
        self.removed_nodes = set()
        self._edge_types = {'txn owner'}
       
    @property
    def node_degrees(self) -> Dict[str, int]:
        """Calculate node degrees excluding self-loops"""
        return dict(self.graph.in_degree())
        
    def _exceeds_degree_threshold(self, node: str, degree: int) -> bool:
        """Check if node exceeds its degree threshold for non-fee sources"""
        if node in self.token_network_obj.fee_sources:
            return False
        return degree > self.degree_threshold

    def default_addresses_to_remove(self) -> set:
        """Default addresses to remove"""
        addresses_to_remove = {
            "contract_address": self.token_data.contract_address,
            "lp_address": self.token_data.lp_address,
            "null_address": "0x0000000000000000000000000000000000000000",
            "dead_address": "0x000000000000000000000000000000000000dEaD",
        }
        return set(addresses_to_remove.values())
    
    def simplify(self) -> nx.DiGraph:
        """Create simplified version of the graph"""
        self.simplified_graph = self.graph.copy()
        
        for node, degree in self.node_degrees.items():
            if self._exceeds_degree_threshold(node, degree):   
                self.removed_nodes.add(node)        
        
        self.simplified_graph.remove_nodes_from(self.removed_nodes)
        self.simplified_graph.remove_nodes_from(self.default_addresses_to_remove())
        return self.simplified_graph
    