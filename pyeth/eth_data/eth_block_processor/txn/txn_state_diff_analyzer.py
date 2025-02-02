from typing import Dict, Any, List, Tuple
from decimal import Decimal
from web3 import Web3
from collections import defaultdict, OrderedDict
from eth_data.addresses.common_addresses import fee_recipients_set
from eth_token_analyzer.utils.logger import get_logger


class TxnStateDiffCalculator:
    """
    TxnStateDiffCalculator: Tracks and analyzes state changes in Ethereum token transactions.

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
                 denom_state_change_threshold=0.0005, 
                 token_state_change_threshold=0.1):
        self.logger = get_logger(name="sanity_check", log_folder="tokens_live")
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
            
            # Check if state change is significant
            if abs(token_net) > self.token_state_change_threshold or abs(denom_net) > self.denom_state_change_threshold:
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
            self.logger.warning(f"Invalid ETH movements net_eth: {net_eth} for {txn_hash} for {self.token_data.contract_address}")
            
        return valid

    def calculate_state_changes(self, txn_dict: dict, block_number: int, txn_index: int) -> dict:
        """Calculate state changes including special cases"""
        self.movements = {  
            'token': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()}),
            'denom': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()})
        }          
        
        # Process all transfers with special case handling
        for transfer in txn_dict.get('erc20_transfers', []):
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
                
        for transfer in txn_dict.get('eth_transfers', []):
            log_index = transfer['log_index']
            transfer_id = (block_number, txn_index, log_index)
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



class TransactionStateDiffAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.addresses: List[str] = []
        self.before_states: Dict[str, Dict[str, Any]] = {}
        self.after_states: Dict[str, Dict[str, Any]] = {}
        self.state_diffs: Dict[str, Dict[str, Any]] = {}

    def parse_state_diff(self, raw_state_diff: Dict[str, Any]):
        for address, state in raw_state_diff.items():
            self.addresses.append(address)
            self.before_states[address] = {}
            self.after_states[address] = {}
            self.state_diffs[address] = {}

            if isinstance(state, dict):
                for key, value in state.items():
                    if key == 'balance':
                        if isinstance(value, dict) and '*' in value:
                            # Balance changed; extract 'from' and 'to' values
                            before_balance = int(value['*']['from'], 16)
                            after_balance = int(value['*']['to'], 16)
                        else:
                            # Balance did not change; 'value' is the current balance
                            before_balance = int(value, 16)
                            after_balance = before_balance

                        self.before_states[address]['balance'] = Decimal(before_balance) / Decimal(10**18)
                        self.after_states[address]['balance'] = Decimal(after_balance) / Decimal(10**18)
                        self.state_diffs[address]['balance_diff'] = self.after_states[address]['balance'] - self.before_states[address]['balance']
                    elif key == 'nonce':
                        if isinstance(value, dict) and '*' in value:
                            before_nonce = int(value['*']['from'], 16)
                            after_nonce = int(value['*']['to'], 16)
                        else:
                            before_nonce = int(value)
                            after_nonce = before_nonce

                        self.before_states[address]['nonce'] = before_nonce
                        self.after_states[address]['nonce'] = after_nonce
                        self.state_diffs[address]['nonce_diff'] = after_nonce - before_nonce
                    elif key == 'storage':
                        self.before_states[address]['storage'] = {}
                        self.after_states[address]['storage'] = {}
                        self.state_diffs[address]['storage_diff'] = {}
                        for storage_key, storage_value in value.items():
                            if isinstance(storage_value, dict) and '*' in storage_value:
                                before_storage = storage_value['*']['from']
                                after_storage = storage_value['*']['to']
                            else:
                                before_storage = storage_value
                                after_storage = storage_value

                            self.before_states[address]['storage'][storage_key] = before_storage
                            self.after_states[address]['storage'][storage_key] = after_storage
                            if before_storage != after_storage:
                                self.state_diffs[address]['storage_diff'][storage_key] = {
                                    'from': before_storage,
                                    'to': after_storage
                                }

        return self.state_diffs, self.after_states

    def get_involved_addresses(self) -> List[str]:
        return self.addresses

    def get_state_diff(self, address: str) -> Dict[str, Any]:
        return self.state_diffs.get(address, {})

    def get_latest_state(self, address: str) -> Dict[str, Any]:
        return self.after_states.get(address, {})

    def get_before_state(self, address: str) -> Dict[str, Any]:
        return self.before_states.get(address, {})

    def get_balance_change(self, address: str) -> Tuple[Decimal, Decimal, Decimal]:
        before_balance = self.before_states.get(address, {}).get('balance', Decimal(0))
        after_balance = self.after_states.get(address, {}).get('balance', Decimal(0))
        diff = after_balance - before_balance
        return before_balance, after_balance, diff

    def get_nonce_change(self, address: str) -> Tuple[int, int, int]:
        before_nonce = self.before_states.get(address, {}).get('nonce', 0)
        after_nonce = self.after_states.get(address, {}).get('nonce', 0)
        diff = after_nonce - before_nonce
        return before_nonce, after_nonce, diff

    def has_storage_change(self, address: str) -> bool:
        return 'storage_diff' in self.state_diffs.get(address, {}) and bool(self.state_diffs[address]['storage_diff'])

    def get_storage_change(self, address: str) -> Dict[str, Any]:
        return self.state_diffs.get(address, {}).get('storage_diff', {})