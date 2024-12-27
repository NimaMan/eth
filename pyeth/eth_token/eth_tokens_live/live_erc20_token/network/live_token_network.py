import os
import networkx as nx
import numpy as np
import pandas as pd
from eth_token_analyzer.utils.logger import get_logger
from eth_token_analyzer.erc20_token.network.graph_simplifier import GraphSimplifier
from eth_token_analyzer.erc20_token.network.sanity_checker import NetworkSanityChecker
from eth_token_analyzer.erc20_token.network.component_analyzer import NetworkSubgraphAnalyzer

from eth_tokens_live.live_erc20_token.network.live_token_network_builder import LiveTokenNetworkBuilder


logger = get_logger(name="pnl", log_folder="eth_token_analyzer")


class LiveTokenNetwork(LiveTokenNetworkBuilder):
    def __init__(self, token_data, high_degree_node_threshold=3, addresses_to_remove=None, verbose=False):
        """
        Initialize LiveTokenNetwork with token data and network analysis parameters
        
        Args:
            token_data: Token data instance containing transfer history
            high_degree_node_threshold: Threshold for high-degree node detection
            addresses_to_remove: List of addresses to exclude from network
            verbose: Enable detailed logging
        """
        # Initialize parent class first
        super().__init__(token_data=token_data)
        
        self.verbose = verbose
        self.token_data = token_data
        self.H = None  # Simplified graph
        self.connected_components = []
        self.high_degree_node_threshold = high_degree_node_threshold
        
        # Initialize analyzers after parent class has set up the base graph (self.G)
        self.sanity_checker = NetworkSanityChecker(self.G, self.token_data)
        self.simplifier = GraphSimplifier(self.G, verbose=self.verbose)
        self.subgraph_analyzer = NetworkSubgraphAnalyzer(graph=self.G, simplified_graph=self.H, verbose=self.verbose)
    
    @property
    def fee_sources(self):
        return [node for node in self.G.nodes() if self.G.nodes[node]['data'].user_is_fee_source]
    
    @property
    def wallets(self):
        return [node for node in self.G.nodes() if self.G.nodes[node]['data'].address_type == 'Wallet']

    @property
    def contracts(self):
        return [node for node in self.G.nodes() if self.G.nodes[node]['data'].address_type == 'Contract']
    
    def get_agg_user_activity_df(self):
        all_users_activity = []
        for address in self.fee_sources:
            node_data = self.G.nodes[address]['data']
            user_activity = node_data.get_user_features()
            # Add component information
            subgraph = self.subgraph_analyzer.get_address_subgraph(address)
            subgraph_addresses = set(subgraph.nodes())
            if subgraph:
                user_activity['agg_denom_balance'] = sum(self.G.nodes[addr]['data'].denom_balance for addr in subgraph_addresses)
                user_activity['agg_token_balance'] = np.round(sum(self.G.nodes[addr]['data'].token_balance for addr in subgraph_addresses), 2)
                user_activity['agg_realized_profit'] = sum(self.G.nodes[addr]['data'].realized_profit for addr in subgraph_addresses)
                user_activity['agg_unrealized_profit'] = sum(self.G.nodes[addr]['data'].unrealized_profit for addr in subgraph_addresses)
                user_activity['agg_total_profit'] = sum(self.G.nodes[addr]['data'].total_profit for addr in subgraph_addresses)
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

     # ---------------------------------------------------------
    # 2) Real-time update method
    # ---------------------------------------------------------
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
        if main_tx_id in self.processed_transactions:
            return
        
        # Mark this transaction-level ID as processed
        self.processed_transactions[main_tx_id] = True
        
        block_number = txn_dict['block_number']
        txn_index = txn_dict['txn_index']
        fee_source = txn_dict['from_address']
        # 2.1) Process ERC20 sub-transfers
        for erc20_transfer in txn_dict.get('erc20_transfers', []):
            sub_txn_id = (block_number, txn_index, erc20_transfer['log_index'])
            if sub_txn_id in self.processed_transactions:
                continue
            self.processed_transactions[sub_txn_id] = True
            
            # Convert or augment data to match _add_transfer_to_graph signature
            amount = erc20_transfer['amount'] if not isinstance(erc20_transfer['amount'], str) else float(erc20_transfer['amount'])/self.token_data.decimals
            sub_dict = {
                'block': block_number,
                'txn_index': txn_index,
                'log_index': erc20_transfer['log_index'],
                'from_address': erc20_transfer['from_address'],
                'to_address': erc20_transfer['to_address'],
                'amount': amount,
                'fee_source': fee_source,
                # is_token_transfer -> True
            }
            self._add_transfer_to_graph(sub_dict, True, sub_txn_id)

        # 2.2) Process WETH or other “denom” transfers
        for weth_transfer in txn_dict.get('weth_transfers', []):
            sub_txn_id = (block_number, txn_index, weth_transfer['log_index'])
            if sub_txn_id in self.processed_transactions:
                continue
            self.processed_transactions[sub_txn_id] = True           
            sub_dict = {
                'block': block_number,
                'txn_index': txn_index,
                'log_index': weth_transfer['log_index'],
                'from_address': weth_transfer['from_address'],
                'to_address': weth_transfer['to_address'],
                'amount': weth_transfer['amount'],
                'fee_source': fee_source,
                # is_token_transfer -> False
            }
            self._add_transfer_to_graph(sub_dict, False, sub_txn_id)

        # 2.3) Process internal transfers (ETH)
        for eth_transfer in txn_dict.get('eth_transfers', []):
            sub_txn_id = (block_number, txn_index, eth_transfer['log_index'])
            if sub_txn_id in self.processed_transactions:
                continue
            self.processed_transactions[sub_txn_id] = True
            
            sub_dict = {
                'block': block_number,
                'txn_index': txn_index,
                'log_index': eth_transfer['log_index'],
                'from_address': eth_transfer['from_address'],
                'to_address': eth_transfer['to_address'],
                'amount': eth_transfer['value'],  # might be “value”
                'fee_source': fee_source,
                # is_token_transfer -> False
            }
            self._add_transfer_to_graph(sub_dict, False, sub_txn_id)
