
import numpy as np
import pandas as pd
from eth_token.erc20_token.network.token_network_builder import LiveTokenNetworkBuilder
from eth_token.erc20_token.network.subgraph_analyzer import NetworkSubgraphAnalyzer


class LiveTokenNetwork(LiveTokenNetworkBuilder):
    def __init__(self, live_token, logger, high_degree_node_threshold=5):
        """
        Initialize LiveTokenNetwork with token data and network analysis parameters
        
        Args:
            token_data: Token data instance containing transfer history
            high_degree_node_threshold: Threshold for high-degree node detection
            addresses_to_remove: List of addresses to exclude from network
            verbose: Enable detailed logging
        """
        # Initialize parent class first
        super().__init__(live_token=live_token, logger=logger)
        
        self.live_token = live_token
        self.connected_components = []
        self.high_degree_node_threshold = high_degree_node_threshold
        
        # Initialize analyzers after parent class has set up the base graph (self.graph)
        self.subgraph_analyzer = NetworkSubgraphAnalyzer(graph=self.graph, token_data=self.live_token.token_data, degree_threshold=self.high_degree_node_threshold)
    
    def get_agg_user_activity_df(self):
        all_users_activity = []
        self.simplified_graph = self.subgraph_analyzer.simplify()
        self.connected_components = self.subgraph_analyzer.find_subgraphs()
        
        for address in self.fee_sources:
            try:
                node_data = self.graph.nodes[address]['data']
            except:
                self.log(f"Address {address} not found in the graph of {self.live_token.contract_address}.")
                continue
            user_activity = node_data.get_user_features()
            # Add component information
            subgraph_addresses = self.subgraph_analyzer.get_related_addresses(address)
            if subgraph_addresses:
                user_activity['agg_denom_balance'] = sum(self.graph.nodes[addr]['data'].denom_balance for addr in subgraph_addresses)
                user_activity['agg_token_balance'] = np.round(sum(self.graph.nodes[addr]['data'].token_balance for addr in subgraph_addresses), 2)
                user_activity['agg_realized_profit'] = sum(self.graph.nodes[addr]['data'].realized_profit for addr in subgraph_addresses)
                user_activity['agg_unrealized_profit'] = sum(self.graph.nodes[addr]['data'].unrealized_profit for addr in subgraph_addresses)
                user_activity['agg_total_profit'] = sum(self.graph.nodes[addr]['data'].total_profit for addr in subgraph_addresses)
                related_addresses = subgraph_addresses - {address}
                user_activity['related_addresses'] = tuple(related_addresses) if related_addresses else None 
            else:
                user_activity['agg_denom_balance'] = np.nan
                user_activity['agg_token_balance'] = np.nan
                user_activity['agg_realized_profit'] = np.nan
                user_activity['agg_unrealized_profit'] = np.nan
                user_activity['agg_total_profit'] = np.nan
                user_activity['related_addresses'] = None
            all_users_activity.append(user_activity)
        
        udf = pd.DataFrame(all_users_activity)
        udf.set_index("address", inplace=True)
        udf.sort_values(["denom_balance", "denom_received_spent_ratio", "total_profit"], ascending=[False, False, False], inplace=True)
        return udf
