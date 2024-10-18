from typing import Dict, Any, List, Tuple
from decimal import Decimal
from web3 import Web3

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