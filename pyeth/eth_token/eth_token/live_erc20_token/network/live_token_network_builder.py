import networkx as nx
from collections import defaultdict, OrderedDict

from eth_block_processor.utils.common_addresses import fee_recipients_set
from eth_token.live_erc20_token.network.user_activity_tracker import UserTokenActivityTracker


class LiveTokenTxnStateDiffCalculator:
    """
    LiveTokenTxnStateDiffCalculator: Tracks and analyzes state changes in Ethereum token transactions.

    This class calculates net state changes for addresses involved in token transactions by:
    1. Tracking token and denomination (ETH/WETH) movements for each address
    2. Handling special cases like WETH conversions and bribes
    3. Filtering out insignificant state changes based on thresholds

    Key Features:
    - Maintains chronological order of transfers using OrderedDict
    - Tracks both incoming and outgoing movements for tokens and denominations
    - Handles special addresses (WETH, null, dead addresses)
    - Identifies significant state changes based on configurable thresholds
    - Excludes WETH conversions from denomination movements
    - Tracks bribe payments to known fee recipients

    Movement Structure:
    {
        'token': {
            'address1': {
                'in': OrderedDict{(block, txn_index, log_index): amount, ...},
                'out': OrderedDict{(block, txn_index, log_index): amount, ...}
            },
            ...
        },
        'denom': {
            'address1': {
                'in': OrderedDict{(block, txn_index, log_index): amount, ...},
                'out': OrderedDict{(block, txn_index, log_index): amount, ...}
            },
            ...
        }
    }

    Net Changes Output Structure:
    {
        'address1': {
            'token_net': float,  # Net token balance change
            'denom_net': float,  # Net denomination balance change
            'movements': {
                'token': {'in': {...}, 'out': {...}},
                'denom': {'in': {...}, 'out': {...}}
            }
        },
        ...
    }

    Special Cases:
    - WETH conversions (deposit/withdraw) are not counted as denomination movements
    - Transfers to fee recipients are tracked as bribes
    - Only addresses with significant state changes (above threshold) are included in output
    """
    def __init__(self, 
                 live_token,
                 denom_state_change_threshold=0.0005, 
                 token_state_change_threshold=0.1,
                 logger=None):
        self.logger = logger
        self.live_token = live_token
        self.WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        self.null_address = "0x0000000000000000000000000000000000000000"
        self.dead_address = "0x000000000000000000000000000000000000dEaD"
        self.denom_state_change_threshold = denom_state_change_threshold
        self.token_state_change_threshold = token_state_change_threshold
        self.movements = {
            'token': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()}),
            'denom': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()})
        }

    def _track_movement(self, movement_type: str, from_addr: str, to_addr: str, amount: float, transfer_id: tuple):
        """Track a movement between addresses and handle special cases"""
        # Handle WETH conversions
        if to_addr == self.WETH_ADDRESS:
            # ETH to WETH: Don't count as denom movement
            return
        elif from_addr == self.WETH_ADDRESS:
            # WETH to ETH: Don't count as denom movement
            return
        elif transfer_id[2] == '0':
            # This is a deployer transaction, skip it
            return
        
        # Track from address out movement
        self.movements[movement_type][from_addr]['out'][transfer_id] = amount
        # Track bribe amount
        if to_addr in fee_recipients_set and movement_type == 'denom':
            return
        else:
            # Track to address in movement if not bribe recipient
            self.movements[movement_type][to_addr]['in'][transfer_id] = amount

    def get_net_changes(self) -> dict:
        """Get net changes for all addresses in this transaction"""
        net_changes = {}
        
        change_addresses = set(self.movements['token'].keys()) | set(self.movements['denom'].keys())
        for address in change_addresses:
            # Calculate token changes
            token_in = sum(self.movements['token'][address]['in'].values())
            token_out = sum(self.movements['token'][address]['out'].values())
            token_net = token_in - token_out
            
            # Calculate denomination changes
            denom_in = sum(self.movements['denom'][address]['in'].values())
            denom_out = sum(self.movements['denom'][address]['out'].values())
            denom_net = denom_in - denom_out
            
            # Check if state change is significant or if address is a fee source
            if abs(token_net) > self.token_state_change_threshold or abs(denom_net) > self.denom_state_change_threshold or address in self.live_token.token_data.txn_hashes_to_makers.values():
                net_changes[address] = {
                    'token_net': token_net,
                    'denom_net': denom_net,
                    'movements': {
                        'token': self.movements['token'][address],
                        'denom': self.movements['denom'][address]
                    }
                }
        return net_changes

    def validate_eth_movements(self, txn_hash: str) -> bool:
        """
        Validate that total ETH movement in transaction nets to zero
        Returns True if movements are valid (net zero), False otherwise
        """
        net_eth = 0
        
        # Sum all denomination movements
        for address in self.movements['denom']:
            eth_in = sum(self.movements['denom'][address]['in'].values())
            eth_out = sum(self.movements['denom'][address]['out'].values())
            net_eth += (eth_in - eth_out)
        
        # If net is not zero, we likely have a deployer or missing trace
        net_eth = net_eth + self.bribe_amount
        valid = abs(net_eth) < 1e-10
        # Only log if it's actually invalid (net_eth >= 1e-10)
        if not valid:
            self.logger.warning(f"Invalid ETH movements net_eth: {net_eth} for {txn_hash} for {self.live_token.contract_address}")
            
        return valid

    def calculate_state_changes(self, txn_hash: str, block_number: int, txn_index: int) -> dict:
        """Calculate state changes including special cases"""
        self.movements = {  
            'token': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()}),
            'denom': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()})
        }          
        
        # Process all transfers with special case handling
        erc20_transfers = self.live_token.token_data.erc20_transfers.get(txn_hash, [])
        for transfer in erc20_transfers:
            log_index = transfer['log_index']
            transfer_id = (block_number, txn_index, log_index)
            is_wet_transfer = transfer["token_address"] == self.WETH_ADDRESS
            if is_wet_transfer:
                movement_type = 'denom'
            else:
                movement_type = 'token'
            self._track_movement(movement_type, 
                                 transfer['from_address'], 
                                 transfer['to_address'], 
                                 transfer['amount'], 
                                 transfer_id
                                 )
        
        eth_transfers = self.live_token.token_data.eth_transfers.get(txn_hash, [])
        for transfer in eth_transfers:
            if "log_index" in transfer:
                log_index = transfer['log_index']
                transfer_id = (block_number, txn_index, log_index)
            else:
                transfer_id = (block_number, txn_index, f"depth_{transfer['depth']}")
            self._track_movement('denom', 
                                 transfer['from_address'], 
                                 transfer['to_address'], 
                                 transfer['amount'], 
                                 transfer_id
                                 )
        try:
            state_changes = self.get_net_changes()
        except Exception as e:
            print(self.movements)
            return {}
        return state_changes


class LiveTokenNetworkBuilder:
    def __init__(self, live_token, logger=None):
        self.logger = logger
        self.live_token = live_token
        self.graph = nx.MultiDiGraph()
        self.state_calculator = LiveTokenTxnStateDiffCalculator(live_token, logger=self.logger)

    def __iter__(self):
        return iter(self.graph.nodes)
    
    def __getitem__(self, address):
        return self.graph.nodes[address]
    
    @property
    def fee_sources(self):
        return self.live_token.token_data.fee_sources
    
    @property
    def txn_hashes(self):
        return self.live_token.token_data.txn_hashes

    def _add_fee_source_edges(self, fee_source: str, addresses: list):
        """Add edges from fee source to all addresses involved in its transaction"""
        if not fee_source or fee_source not in self.graph:
            return
            
        for address in addresses:
            if address in self.graph and address != fee_source:
                # Add directed edge from fee source to address if it does not exist
                if not self.graph.has_edge(fee_source, address):
                    self.graph.add_edge(fee_source, address, type='txn owner')

    def graph_add_or_update_address(self, 
                               address: str, 
                               state_changes: dict, 
                               block_number: int, 
                               txn_index: int, 
                               fee_source: str, 
                               bribe_amount: float):
        """Add new address or update existing one with movement data"""
        is_fee_source = address == fee_source
        if address not in self.graph:
            # Create new node if it doesn't exist
            self.graph.add_node(
                address, 
                data=UserTokenActivityTracker(
                    address=address,
                    address_type=None,
                    token_data=self.live_token.token_data,
                    entry_block=block_number,
                    entry_index=txn_index,
                    entry_log_index=None,
                    is_fee_source=is_fee_source,
                    fee_source=fee_source,
                )
            )
    
        # Get the user activity tracker
        user_activity = self.graph.nodes[address]['data']

        if is_fee_source:
            # Add bribe amount
            user_activity.bribe_amount += bribe_amount
        
        # Update movements
        movements = state_changes['movements']
        # Add token movements
        for transfer_id, amount in movements['token']['in'].items():
            user_activity.token_in_dict[transfer_id] = amount
        for transfer_id, amount in movements['token']['out'].items():
            user_activity.token_out_dict[transfer_id] = amount
            
        # Add denomination movements
        for transfer_id, amount in movements['denom']['in'].items():
            user_activity.denom_in_dict[transfer_id] = amount
        for transfer_id, amount in movements['denom']['out'].items():
            user_activity.denom_out_dict[transfer_id] = amount
    
    def update_from_transaction(self, txn_dict: dict):
        """Process transaction and update network with significant changes"""
        block_number = txn_dict['block_number']
        txn_hash = txn_dict['hash']
        txn_index = txn_dict['txn_index']
        fee_source = txn_dict['from_address']
        bribe_amount = txn_dict['bribe_amount']
        
        # Get state changes (only significant ones are returned by calculator)
        state_changes = self.state_calculator.calculate_state_changes(txn_hash, block_number, txn_index)
        
        # Add or update addresses with their movements
        for address, addr_state_changes in state_changes.items():
            self.graph_add_or_update_address(address, 
                                             addr_state_changes, 
                                             block_number, 
                                             txn_index, 
                                             fee_source, 
                                             bribe_amount
                                             )
        
        self._add_fee_source_edges(fee_source, state_changes.keys())
