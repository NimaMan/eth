from typing import Dict, Any, List, Tuple
from decimal import Decimal
from web3 import Web3
from web3.exceptions import BlockNotFound


class TransactionStateDiffAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.addresses: List[str] = []
        self.before_states: Dict[str, Dict[str, Any]] = {}
        self.after_states: Dict[str, Dict[str, Any]] = {}
        self.state_diffs: Dict[str, Dict[str, Any]] = {}

    def parse_state_diff(self, raw_state_diff: Dict[str, Any], transaction_block_number: int):
        # Collect addresses from raw_state_diff
        self.addresses = list(raw_state_diff.keys())

        # For each address, get the state before and after the transaction
        for address in self.addresses:
            self.before_states[address] = {}
            self.after_states[address] = {}
            self.state_diffs[address] = {}

            state = raw_state_diff[address]

            # Parse after-state balance
            if 'balance' in state:
                after_balance = int(state['balance'], 16) if isinstance(state['balance'], str) else int(state['balance'])
                self.after_states[address]['balance'] = Decimal(after_balance) / Decimal(10**18)
            else:
                self.after_states[address]['balance'] = Decimal(0)

            # Parse after-state nonce
            if 'nonce' in state:
                after_nonce = int(state['nonce'])
                self.after_states[address]['nonce'] = after_nonce
            else:
                self.after_states[address]['nonce'] = 0

            # Fetch before-state balance and nonce
            block_number = transaction_block_number - 1
            try:
                before_balance = self.w3.eth.get_balance(address, block_identifier=block_number)
                self.before_states[address]['balance'] = Decimal(before_balance) / Decimal(10**18)
            except Exception:
                # If the address didn't exist before, balance is zero
                self.before_states[address]['balance'] = Decimal(0)

            try:
                before_nonce = self.w3.eth.get_transaction_count(address, block_identifier=block_number)
                self.before_states[address]['nonce'] = before_nonce
            except Exception:
                # If the address didn't exist before, nonce is zero
                self.before_states[address]['nonce'] = 0

            # Calculate differences
            balance_diff = self.after_states[address]['balance'] - self.before_states[address]['balance']
            nonce_diff = self.after_states[address]['nonce'] - self.before_states[address]['nonce']

            self.state_diffs[address]['balance_diff'] = balance_diff
            self.state_diffs[address]['nonce_diff'] = nonce_diff

            # Handle storage changes if available
            if 'storage' in state:
                self.after_states[address]['storage'] = state['storage']
                # Note: Fetching previous storage requires an archive node
                self.before_states[address]['storage'] = {}  # Placeholder
                self.state_diffs[address]['storage_diff'] = state['storage']

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
        return 'storage_diff' in self.state_diffs.get(address, {})

    def get_storage_change(self, address: str) -> Dict[str, Any]:
        return self.state_diffs.get(address, {}).get('storage_diff', {})