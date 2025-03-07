"""
# Token Network Subgraph Analyzer
Simplifies token transaction networks by removing common nodes that are not fee sources such as:
    - Token contract address
    - Liquidity pool contract address
    - Dead Address
    - Zero Address
    - Tax collector address
    - Any other high degree node like tax collector, etc that is common in the transactions of several fee sources

    
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

Simplification Process:
--------------------
1. Remove the high degree and default addresses
2. Degree Calculation:
We create a direct graph for the tx_owner type of edges. This shows all the addresses that have been in the txn of another address. 

# TODO:
define the high degree nodes.   
    - Maybe I will need to use teh type fo the address to limit the wallets and the contracts. 
    I might need to do this becuase of the fake transfer done by some addresses to create fake volume.

   - Adjusted Degree Computation
     * Accounts for both in and out edges
     * Excludes self-loops from degree count
     * Separate thresholds for wallets and contracts

Subgraph Structure:
------------------
1. Connected Subgraphs:
   - Groups of interconnected addresses
   - Identified through graph traversal
   - Can be weakly or strongly connected
   - Sorted by size and significance

The connected subgraphs in token transaction networks identifies the subset of related addresses that are most likely related to same people or bots.

"""

import networkx as nx
import numpy as np
import pandas as pd
from collections import Counter
from typing import List, Set, Dict, Optional
from dataclasses import dataclass
from eth_token.live_erc20_token.data.live_token_data import LiveTokenData


@dataclass
class SubgraphMetrics:
    """Stores metrics for a network component"""
    size: int
    num_wallets: int
    num_contracts: int
    denom_balance: float
    token_balance: float
    realized_profit: float
    unrealized_profit: float
    total_profit: float
    avg_realized_profit: float
    fee_sources: List[str]
    

class NetworkSubgraphAnalyzer:
    def __init__(self, graph: nx.DiGraph, token_data: LiveTokenData, degree_threshold: int = 10):
        """
        Initialize component analyzer with original and simplified graphs.
        
        Args:
            graph: Original network graph
            simplified_graph: Simplified version of the graph
        """
        self.graph = graph
        self.token_data = token_data
        self.degree_threshold = degree_threshold
        self.removed_nodes = set()
        self._edge_types = {'txn owner'}
        self.simplified_graph = None

    def default_addresses_to_remove(self) -> set:
        """Default addresses to remove"""
        addresses_to_remove = {
            self.token_data.contract_address,
            "0x0000000000000000000000000000000000000000",
            "0x000000000000000000000000000000000000dEaD",
        }
        # Add the pool addresses to the set of addresses to remove
        addresses_to_remove.union(set(self.token_data.pool_info.keys()))
        return addresses_to_remove
    
    def simplify(self) -> nx.DiGraph:
        """Create simplified version of the graph"""
        simplified_graph = self.graph.copy()
        for node, degree in self.graph.in_degree():
            # Keep the fee sources 
            if node in self.token_data.fee_sources:
                continue
            if degree > self.degree_threshold:
                self.removed_nodes.add(node)        
        
        # If a fee source is having a significant number of outgoing edges, we remove it (potential spammer or scammer)
        for node, degree in self.graph.out_degree():
            # Keep the fee sources 
            if node in self.token_data.fee_sources:
                continue
            if degree > 2*self.degree_threshold:
                self.removed_nodes.add(node)        
        
        simplified_graph.remove_nodes_from(self.removed_nodes)
        simplified_graph.remove_nodes_from(self.default_addresses_to_remove())
        return simplified_graph
    
    def find_subgraphs(self):
        """Identify and sort connected components"""
        if self.simplified_graph is None:
            self.simplified_graph = self.simplify()
        if isinstance(self.simplified_graph, nx.DiGraph):
            connected_components = list(nx.weakly_connected_components(self.simplified_graph))
        else:
            connected_components = list(nx.connected_components(self.simplified_graph))
        
        connected_components.sort(key=len, reverse=True)
        self.component_sizes = [len(c) for c in connected_components]
        self.size_counts = Counter(self.component_sizes)
        self.connected_components = connected_components
        return connected_components

    def get_related_addresses(self, address):
        """Retrieve the connected component that the given address belongs to."""
        for component in self.connected_components:
            if address in component:
                return component
        
        return None
       
    def get_address_subgraph(self, address, depth=1):
        """
        Get a subgraph centered around a particular address.

        :param address: The central address to build the subgraph around.
        :param depth: The number of hops to include in the subgraph (default is 1).
        :return: A NetworkX subgraph.
        """
        if address not in self.graph:
            print(f"Address {address} not found in the graph.")
            return None

        nodes = set([address])
        for _ in range(depth):
            neighbors = set()
            for node in nodes:
                neighbors.update(self.graph.predecessors(node))
                neighbors.update(self.graph.successors(node))
            nodes.update(neighbors)

        return self.graph.subgraph(nodes)

    def get_subgraph_features(self, subgraph: Set[str]) -> SubgraphMetrics:
        """Calculate comprehensive metrics for a subgraph"""
        wallets = sum(1 for addr in subgraph 
                     if self.graph.nodes[addr]['data'].address_type == 'Wallet')
        contracts = len(subgraph) - wallets
        
        # Calculate financial metrics
        denom_balance = 0
        token_balance = 0
        realized_profit = 0
        unrealized_profit = 0
        fee_sources = []
        
        for addr in subgraph:
            node_data = self.graph.nodes[addr]['data']
            denom_balance += node_data.denom_balance
            token_balance += node_data.token_balance
            realized_profit += node_data.realized_profit
            unrealized_profit += node_data.unrealized_profit
            
            if node_data.user_is_fee_source:
                fee_sources.append(addr)
        
        return SubgraphMetrics(
            size=len(subgraph),
            num_wallets=wallets,
            num_contracts=contracts,
            denom_balance=denom_balance,
            token_balance=token_balance,
            realized_profit=realized_profit,
            unrealized_profit=unrealized_profit,
            total_profit=realized_profit + unrealized_profit,
            avg_realized_profit=realized_profit / len(subgraph) if subgraph else 0,
            fee_sources=fee_sources
        )
    
    def analyze_subgraphs(self) -> pd.DataFrame:
        """Analyze all components and create summary DataFrame"""
        component_features = []
        for idx, subgraph in enumerate(self.connected_components):
            metrics = self.get_subgraph_features(subgraph)
            self.component_metrics[idx] = metrics
            component_features.append(vars(metrics))
        
        self.components_df = pd.DataFrame(component_features)
        self.components_df.sort_values('size', ascending=False, inplace=True)
        
        return self.components_df    

    def get_subgraph_df(self, subgraph):
        all_users_activity = []
        for address in subgraph:
            node = self.graph.nodes[address]['data']
            user_activity = node.get_user_features()
            all_users_activity.append(user_activity)
        
        udf = pd.DataFrame(all_users_activity)
        udf.sort_values(["denom_balance", "denom_received_spent_ratio", "total_profit"], ascending=[False, False, False], inplace=True)
        return udf
    
    def aggregate_subgraph(self, subgraph):
        """
        Aggregate data for a connected component and perform sanity checks.

        :param subgraph: Set of addresses in the connected subgraph
        :return: Dictionary with aggregated data and sanity check results
        """
        subgraph_df = self.get_subgraph_df(subgraph)
        
        aggregated_data = {
            'denom_balance': subgraph_df['denom_balance'].sum(),
            'token_balance': subgraph_df['total_token_bought'].sum() - subgraph_df['total_token_sold'].sum(),
            'realized_profit': subgraph_df['realized_profit'].sum(),
            'unrealized_profit': subgraph_df['unrealized_profit'].sum(),
            'total_profit': subgraph_df['total_profit'].sum(),
            'num_addresses': len(subgraph),
            'num_wallets': (subgraph_df['address_type'] == 'Wallet').sum(),
            'num_contracts': (subgraph_df['address_type'] == 'Contract').sum(),
            
        }
                
        return aggregated_data
    
    def get_subgraph_rankings(self) -> pd.DataFrame:
        """
        Rank components based on multiple metrics:
        - Size rank
        - Profit rank (total and average)
        - Activity rank (transaction volume)
        - Balance rank (token and denomination)
        """
        if self.components_df is None:
            self.analyze_subgraphs()
        
        # Calculate ranks for different metrics (ascending=False for higher values = better rank)
        ranks = pd.DataFrame({
            'size_rank': self.components_df['size'].rank(ascending=False),
            'total_profit_rank': self.components_df['realized_profit'].rank(ascending=False),
            'avg_profit_rank': self.components_df['avg_realized_profit'].rank(ascending=False),
            'denom_balance_rank': self.components_df['denom_balance'].rank(ascending=False),
            'token_balance_rank': self.components_df['token_balance'].rank(ascending=False)
        })
        
        # Calculate composite rank (lower is better)
        ranks['composite_rank'] = ranks.mean(axis=1)
        
        # Add ranks to component metrics
        self.components_df = pd.concat([self.components_df, ranks], axis=1)
         
        return self.components_df
    
    def get_top_components(self, metric: str = 'composite_rank', n: int = 5) -> pd.DataFrame:
        """
        Get top N components based on specified metric.
        
        Args:
            metric: Ranking metric ('composite_rank', 'size_rank', 'total_profit_rank', etc.)
            n: Number of components to return
        """
        if metric.endswith('_rank'):
            return self.components_df.nsmallest(n, metric)
        else:
            return self.components_df.nlargest(n, metric)
    