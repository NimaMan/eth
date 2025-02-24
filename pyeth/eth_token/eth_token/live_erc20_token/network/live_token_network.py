
import numpy as np
import pandas as pd
from eth_token.live_erc20_token.network.live_token_network_builder import LiveTokenNetworkBuilder
from eth_token.live_erc20_token.network.graph_simplifier import GraphSimplifier
from eth_token.live_erc20_token.network.subgraph_analyzer import NetworkSubgraphAnalyzer


class LiveTokenNetwork(LiveTokenNetworkBuilder):
    def __init__(self, token_data, logger, high_degree_node_threshold=3):
        """
        Initialize LiveTokenNetwork with token data and network analysis parameters
        
        Args:
            token_data: Token data instance containing transfer history
            high_degree_node_threshold: Threshold for high-degree node detection
            addresses_to_remove: List of addresses to exclude from network
            verbose: Enable detailed logging
        """
        # Initialize parent class first
        super().__init__(token_data=token_data, logger=logger)
        
        self.token_data = token_data
        self.simplified_graph = None  # Simplified graph
        self.connected_components = []
        self.high_degree_node_threshold = high_degree_node_threshold
        
        # Initialize analyzers after parent class has set up the base graph (self.graph)
        self.simplifier = GraphSimplifier(self)
        self.subgraph_analyzer = NetworkSubgraphAnalyzer(graph=self.graph, simplified_graph=self.simplified_graph)
    
    def get_agg_user_activity_df(self):
        all_users_activity = []
        for address in self.fee_sources:
            node_data = self.graph.nodes[address]['data']
            user_activity = node_data.get_user_features()
            # Add component information
            subgraph = self.subgraph_analyzer.get_address_subgraph(address)
            subgraph_addresses = set(subgraph.nodes())
            if subgraph:
                user_activity['agg_denom_balance'] = sum(self.graph.nodes[addr]['data'].denom_balance for addr in subgraph_addresses)
                user_activity['agg_token_balance'] = np.round(sum(self.graph.nodes[addr]['data'].token_balance for addr in subgraph_addresses), 2)
                user_activity['agg_realized_profit'] = sum(self.graph.nodes[addr]['data'].realized_profit for addr in subgraph_addresses)
                user_activity['agg_unrealized_profit'] = sum(self.graph.nodes[addr]['data'].unrealized_profit for addr in subgraph_addresses)
                user_activity['agg_total_profit'] = sum(self.graph.nodes[addr]['data'].total_profit for addr in subgraph_addresses)
                user_activity['related_addresses'] = tuple(subgraph_addresses - {address})
            else:
                user_activity['agg_denom_balance'] = np.nan
                user_activity['agg_token_balance'] = np.nan
                user_activity['agg_realized_profit'] = np.nan
                user_activity['agg_unrealized_profit'] = np.nan
                user_activity['agg_total_profit'] = np.nan
                user_activity['related_addresses'] = tuple()
            all_users_activity.append(user_activity)
        
        udf = pd.DataFrame(all_users_activity)
        udf.set_index("address", inplace=True)
        udf.sort_values(["denom_balance", "denom_received_spent_ratio", "total_profit"], ascending=[False, False, False], inplace=True)
        return udf

    def update_from_transaction(self, txn_dict: dict):
        """
        Process a single, real-time transaction that may have multiple sub-transfers:
        
        Example structure:
        {
            'block_number': 1234567,
            'txn_index': 5,
            'hash': '0xabc123...',
            'erc20_transfers': [
                {
                    'log_index': 0,
                    'from_address': '0x111...',
                    'to_address': '0x222...',
                    'amount': 100.5,
                    'fee_source': '0xFEE...',
                },
                ...
            ],
            'weth_transfers': [...],
            'eth_transfers': [...],
            ...
        }
        """
        # Build a unique ID for the entire transaction (not each log)
        main_tx_id = (txn_dict['block_number'], txn_dict['txn_index'], txn_dict.get('hash',''))
        
        # If we already processed the entire transaction, skip
        if main_tx_id in self.processed_txn.keys():
            return
        
        self.update_network_from_txn(txn_dict)
