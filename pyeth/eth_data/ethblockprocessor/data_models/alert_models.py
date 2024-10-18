from dataclasses import dataclass, field
from typing import Dict, Any

@dataclass
class BaseAlertData:
    block_number: int
    transaction_hash: str
    alert_type: str
    details: Dict[str, Any] = field(default_factory=dict)

@dataclass
class TradingEnabledAlertData:
    block_number: int
    transaction_hash: str
    contract_address: str
    alert_type: str = "Trading Enabled"
    details: Dict[str, Any] = field(default_factory=dict)

@dataclass
class BribeAlertData:
    block_number: int
    transaction_hash: str
    bribe_amount: float
    from_address: str
    token_address: str
    alert_type: str = "Bribe"
    details: Dict[str, Any] = field(default_factory=dict)

@dataclass
class ContractCreationAlertData:
    block_number: int
    transaction_hash: str
    alert_type: str = "Contract Creation"
    details: Dict[str, Any] = field(default_factory=dict)