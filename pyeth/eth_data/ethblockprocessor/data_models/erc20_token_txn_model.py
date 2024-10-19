
from dataclasses import dataclass


@dataclass
class ERC20TokenTxn:
    tx_hash: str
    block_number: int
    txn_index: int
    contract_address: str
    from_address: str
    value: int