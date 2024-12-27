import pandas as pd
import networkx as nx

from eth_data.addresses.common_addresses import fee_recipients_set
from eth_token_analyzer.users.user_activity_tracker import UserTokenActivityTracker


class LiveTokenNetworkBuilder:
    def __init__(self, token_data):
        self.token_data = token_data
        self.G = nx.MultiDiGraph()
        self.special_addresses = {"0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"}
        
        # Keeps track of processed sub-transfers (block, txn_index, log_index)
        self.processed_transactions = {}
        
    def __iter__(self):
        return iter(self.G.nodes)
    
    def __getitem__(self, address):
        return self.G.nodes[address]
    
    @property
    def graph(self):
        return self.G
    
    # ---------------------------------------------------------
    # 1) Historical build for DataFrame-based token_data
    # ---------------------------------------------------------
    def build_historical_graph(self):
        token_transfer_df = self.token_data.transfer_df
        self._process_all_transfers(token_transfer_df, is_token=True)
        
        denom_transfer_df = self.token_data.denom_transfers_df
        self._process_all_transfers(denom_transfer_df, is_token=False)
        
        int_transaction_df = self.token_data.int_transaction_df
        self._process_all_transfers(int_transaction_df, is_token=False)
        
        return self.G

    def _process_all_transfers(self, transfers_df, is_token: bool):
        """
        Batch processing for historical data. Each row is one transfer.
        """
        for idx, transfer in transfers_df.iterrows():
            txn_id = (transfer['block'], transfer['txn_index'], transfer['log_index'])
            if txn_id in self.processed_transactions:
                continue
            self.processed_transactions[txn_id] = idx
            self._add_transfer_to_graph(transfer.to_dict(), is_token, txn_id)

    # ---------------------------------------------------------
    # 2) Common addition logic
    # ---------------------------------------------------------
    def _add_transfer_to_graph(self, transfer: dict, is_token: bool, txn_id: tuple):
        fee_source = transfer['fee_source']
        from_address = transfer['from_address']
        to_address = transfer['to_address']
        amount = transfer['amount']
        
        self._update_node(from_address, -amount, is_token, fee_source, txn_id)
        self._update_node(to_address, amount, is_token, fee_source, txn_id)
        
        self.handle_fee_source(fee_source, txn_id)
        self._handle_special_cases(from_address, to_address, amount, is_token, fee_source, txn_id)
        self.handle_edges(fee_source, from_address, to_address, is_token)

    def handle_edges(self, fee_source, from_address, to_address, is_token):
        relation_type = 'token' if is_token else 'denom'
        # If you want an edge from from_address -> to_address:
        # self.G.add_edge(from_address, to_address, type=relation_type)

        if not self.G.has_edge(fee_source, from_address):
            self.G.add_edge(fee_source, from_address, type='fee')
        if not self.G.has_edge(fee_source, to_address):
            self.G.add_edge(fee_source, to_address, type='fee')

    def handle_fee_source(self, fee_source, txn_id: tuple):
        if fee_source not in self.G:
            self.G.add_node(fee_source, data=UserTokenActivityTracker(
                address=fee_source,
                address_type=None,
                token_data=self.token_data,
                is_fee_source=True,
                entry_block=txn_id[0],
                entry_index=txn_id[1],
                entry_log_index=txn_id[2]
            ))

    def _update_node(self, address: str, amount: float, is_token: bool, fee_source: str, txn_id: tuple):
        if address not in self.G:
            self.G.add_node(address, data=UserTokenActivityTracker(
                address=address,
                address_type=None,
                token_data=self.token_data,
                is_fee_source=(address == fee_source),
                fee_source=fee_source,
                entry_block=txn_id[0],
                entry_index=txn_id[1],
                entry_log_index=txn_id[2]
            ))

        node_data = self.G.nodes[address]['data']
        
        if is_token:
            if amount > 0:
                node_data.token_in_dict[txn_id] = amount
            else:
                node_data.token_out_dict[txn_id] = -amount
        else:
            if amount > 0:
                node_data.denom_in_dict[txn_id] = amount
            else:
                node_data.denom_out_dict[txn_id] = -amount

    def _handle_special_cases(self, from_address, to_address, amount, is_token, fee_source, txn_id):
        if to_address == "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2":
            # ETH -> WETH
            node_data = self.G.nodes[from_address]['data']
            node_data.denom_out_dict.pop(txn_id, None)
        elif from_address == "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2":
            # WETH -> ETH
            node_data = self.G.nodes[to_address]['data']
            node_data.denom_in_dict.pop(txn_id, None)
        elif to_address in fee_recipients_set:
            self._update_bribe_amount(fee_source, amount, txn_id)

    def _update_bribe_amount(self, fee_source, amount, txn_id):
        if fee_source in self.G:
            self.G.nodes[fee_source]['data'].bribe_amount += amount
        else:
            self.G.add_node(fee_source, data=UserTokenActivityTracker(
                address=fee_source,
                address_type=None,
                token_data=self.token_data,
                is_fee_source=True,
                bribe_amount=amount,
                entry_block=txn_id[0],
                entry_index=txn_id[1],
                entry_log_index=txn_id[2]
            ))

    # ---------------------------------------------------------
    # 4) Utility: user activity queries
    # ---------------------------------------------------------
    def get_user_activity(self, address):
        if address not in self.G:
            return {}
        return self.G.nodes[address]['data'].get_user_features()
    
    def get_user_activity_df(self):
        all_users_activity = []
        for address in self:
            node = self.G.nodes[address]['data']
            user_activity = node.get_user_features()
            all_users_activity.append(user_activity)
        
        udf = pd.DataFrame(all_users_activity)
        udf.sort_values(["denom_balance", "denom_received_spent_ratio", "total_profit"],
                        ascending=[False, False, False], inplace=True)
        return udf