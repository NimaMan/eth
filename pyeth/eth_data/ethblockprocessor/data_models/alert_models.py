from dataclasses import dataclass, field
from typing import Dict, Any, Set


@dataclass
class TradingEnabledAlertData:
    block_number: int
    transaction_hash: str
    contract_address: str
    from_address: str
    alert_type: str = "Trading Enabled"
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class BribeAlertData:
    block_number: int
    transaction_hash: str
    from_address: str
    value: float
    alert_type: str = "Bribe"
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ContractCreationAlertData:
    block_number: int
    transaction_hash: str
    creator_address: str    
    contract_address: str
    alert_type: str = "Contract Creation"
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class UserInvolvedAlertData:
    block_number: int
    transaction_hash: str
    from_address: str
    involved_addresses: Set[str]
    txn_type: str
    alert_type: str
    details: Dict[str, Any] = field(default_factory=dict)
