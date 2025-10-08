import orjson
import networkx as nx
from networkx.readwrite import json_graph

from eth_data.tx_processor.address_balance_change_calculator import AddressBalanceChangeCalculator
from eth_token.erc20_token.network.address_activity_tracker import AddressTokenActivityTracker


class LiveTokenNetworkBuilder:
    def __init__(self, live_token):
        self.live_token = live_token
        self.graph = nx.MultiDiGraph()
        self.state_diff_calculator = AddressBalanceChangeCalculator()

    def __iter__(self):
        return iter(self.graph.nodes)
    
    def __getitem__(self, address):
        return self.graph.nodes[address]
    
    @property
    def fee_sources(self):
        return self.live_token.token_data.fee_sources
    
    @property
    def tx_hashes(self):
        return self.live_token.token_data.tx_hashes
    
    def _add_fee_source_edges(self, fee_source: str, addresses: list):
        """Add edges from fee source to all addresses involved in its transaction"""
        if not fee_source or fee_source not in self.graph:
            return
            
        for address in addresses:
            if address in self.graph and address != fee_source:
                # Add directed edge from fee source to address if it does not exist
                if not self.graph.has_edge(fee_source, address):
                    self.graph.add_edge(fee_source, address, type='tx owner')

    def graph_add_or_update_address(self, 
                               address: str, 
                               state_changes: dict, 
                               block_number: int, 
                               tx_index: int, 
                               fee_source: str, 
                               bribe_amount: float,
                               tx_fee: float):
        """Add new address or update existing one with movement data"""
        is_fee_source = address == fee_source
        if address not in self.graph:
            # Create new node if it doesn't exist
            self.graph.add_node(
                address, 
                data=AddressTokenActivityTracker(
                    address=address,
                    address_type=None,
                    token_data=self.live_token.token_data,
                    entry_block=block_number,
                    latest_block=block_number,
                    entry_index=tx_index,
                    entry_log_index=None,
                    is_fee_source=is_fee_source,
                    fee_source=fee_source,
                )
            )
    
        # Get the user activity tracker
        user_activity = self.graph.nodes[address]['data']
        user_activity.latest_block = block_number
        user_activity.tx_fees.append(tx_fee)
        if is_fee_source:
            # Add bribe amount
            user_activity.bribe_amount += bribe_amount
        
        # Update movements
        movements = state_changes['movements']
        
        # Add token movements (movements['tokens'][token_address]['in'/'out'])
        if 'tokens' in movements:
            for token_address, token_movements in movements['tokens'].items():
                for transfer_id, amount in token_movements.get('in', {}).items():
                    user_activity.token_in_dict[transfer_id] = amount
                for transfer_id, amount in token_movements.get('out', {}).items():
                    user_activity.token_out_dict[transfer_id] = amount
            
        # Add denomination movements (ETH) from currencies map
        if 'currencies' in movements:
            eth_movements = movements['currencies'].get('ETH') if isinstance(movements['currencies'], dict) else None
            if eth_movements:
                # Convert wei to ETH for denom tracking
                for transfer_id, amount in eth_movements.get('in', {}).items():
                    user_activity.denom_in_dict[transfer_id] = amount / 1e18
                for transfer_id, amount in eth_movements.get('out', {}).items():
                    user_activity.denom_out_dict[transfer_id] = amount / 1e18
    
    def update_from_transaction(self, tx_dict: dict):
        """Process transaction and update network with significant changes"""
        block_number = tx_dict['block_number']
        tx_hash = tx_dict['hash']
        tx_index = tx_dict['tx_index']
        fee_source = tx_dict['from_address']
        bribe_amount = tx_dict['bribe_amount']
        tx_fee = tx_dict['fees']['tx_fee']
        erc20_transfers = self.live_token.token_data.erc20_transfers.get(tx_hash, [])
        eth_transfers = self.live_token.token_data.eth_transfers.get(tx_hash, [])
        # Get balance changes (only significant ones are returned by calculator)
        balance_changes = self.state_diff_calculator.calculate_address_balance_changes(
            tx_hash, 
            fee_source,
            block_number, 
            tx_index, 
            eth_transfers, 
            erc20_transfers
            )
        
        # Add or update addresses with their movements
        for address, addr_balance_changes in balance_changes.items():
            self.graph_add_or_update_address(address, 
                                             addr_balance_changes, 
                                             block_number, 
                                             tx_index, 
                                             fee_source, 
                                             bribe_amount,
                                             tx_fee
                                             )
        
        self._add_fee_source_edges(fee_source, balance_changes.keys())

    def serialize_for_frontend(self, graph: nx.MultiDiGraph=None):
        if graph is None:
            graph = self.graph
        graph_data = json_graph.node_link_data(graph)

        # Create nodes with serializable data from UserTokenActivityTracker
        simplified_nodes = []
        for node in graph_data["nodes"]:
            node_id = node["id"]
            # Get the UserTokenActivityTracker object
            if "data" in graph.nodes[node_id]:
                user_tracker = graph.nodes[node_id]["data"]
                # Convert UserTokenActivityTracker to a serializable dict
                user_data = user_tracker.get_user_features()
                simplified_nodes.append({
                    "id": node_id,
                    "data": user_data  # Include serialized user data
                })
            else:
                simplified_nodes.append({"id": node_id})
        
        # Create simplified links
        simplified_links = [
            {"source": link["source"], "target": link["target"], "type": link["type"]}
            for link in graph_data["links"]
        ]

        data = {"nodes": simplified_nodes, "links": simplified_links}

        # Use orjson with NumPy serialization option
        return orjson.dumps(data, option=orjson.OPT_SERIALIZE_NUMPY)
