from collections import defaultdict, OrderedDict
from eth_block_processor.utils.common_addresses import fee_recipients_set


class ProcessedTxStateDiffCalculator:
    """
    ProcessedTxStateDiffCalculator: Tracks and analyzes state changes in Ethereum transactions.

    This class calculates net state changes for addresses involved in transactions by:
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
                 token_state_change_threshold=0.1,
                 logger=None):
        self.logger = logger
        self.WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
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

    def get_net_changes(self, from_address: str) -> dict:
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
            if abs(token_net) > self.token_state_change_threshold or abs(denom_net) > self.denom_state_change_threshold or address == from_address:
                net_changes[address] = {
                    'token_net': token_net,
                    'denom_net': denom_net,
                    'movements': {
                        'token': self.movements['token'][address],
                        'denom': self.movements['denom'][address]
                    }
                }
        return net_changes

    def validate_eth_movements(self) -> bool:
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
        return valid

    def calculate_state_changes(
            self, 
            txn_hash: str, 
            from_address: str,
            block_number: int, 
            txn_index: int, 
            eth_transfers: list, 
            erc20_transfers: list) -> dict:
        """Calculate state changes including special cases"""
        self.movements = {  
            'token': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()}),
            'denom': defaultdict(lambda: {'in': OrderedDict(), 'out': OrderedDict()})
        }          
        
        # Process all transfers with special case handling
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
            state_changes = self.get_net_changes(from_address)
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error calculating state changes for {txn_hash}: {str(e)}")
            return {}
        return state_changes
