from dataclasses import dataclass, field
from typing import Set


@dataclass
class TradingEnabledAlertData:
    block_number: int
    transaction_hash: str
    contract_address: str
    from_address: str
    alert_type: str = "Trading Enabled"
    erc20_contracts: Set[str] = field(default_factory=tuple)


@dataclass
class BribeAlertData:
    block_number: int
    bribe_amount: float
    alert_type: str = "Bribe"
    token_address: str


@dataclass
class ContractCreationAlertData:
    block_number: int
    transaction_hash: str
    creator_address: str    
    contract_address: str
    contract_type: str
    alert_type: str = "Contract Creation"
    input: str = ""


@dataclass
class UserInvolvedAlertData:
    block_number: int
    involved_addresses: Set[str]
    token_address: str
    alert_type: str
