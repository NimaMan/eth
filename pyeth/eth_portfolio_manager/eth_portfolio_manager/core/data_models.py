from enum import Enum
from dataclasses import dataclass, asdict
from typing import Optional, List, Tuple, Dict, Any


class TradingDecision(Enum):
    SUBMIT_BUY = "SUBMIT_BUY"
    SUBMIT_SELL = "SUBMIT_SELL"
    CONFIRM_BUY = "CONFIRM_BUY"
    CONFIRM_SELL = "CONFIRM_SELL"


class TokenPositionState(Enum):
    INIT = "Init"  # Initial state - token created but not active for trading
    BUY_SUBMITTED = "Buy Submitted"  # Submitted to the chain but not yet confirmed
    BUY_CONFIRMED = "Buy Confirmed"  # We have a position in the token
    SELL_SUBMITTED = "Sell Submitted"  # Submitted to the chain but not yet confirmed
    SELL_CONFIRMED = "Sell Confirmed"  # We have sold the token
    SCAMMED = "Scammed"  # Token is scammed and not active for trading


@dataclass
class TradeSignal:
    token_address: str
    decision: TradingDecision
    quantity: float
    strategy_name: str


@dataclass
class TokenPositionStaticData:
    token_address: str
    symbol: str
    currency: str
    pool_address: str
    pool_type: str
    creation_block: int
    creation_timestamp: int
    trading_enabled_block: int
    trading_enabled_timestamp: int
    purchase_value: float
    entry_price_ratio: float  # relative price of the token at entry to its intial price 
    exit_price_ratio: float  # relative price of the token at exit to its intial price 
    entry_block: int
    exit_block: int
    entry_timestamp: int
    exit_timestamp: int
    entry_txn_fee: float
    exit_txn_fee: float

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class TokenPositionDynamicSnapshot:
    '''
    Defalt values represent the initial state of the token position
    '''
    current_price_ratio: float = 0.0  # relative price of the token at current time
    reserve: float = 0.0
    roi: float = 0.0  # ratio current_price_ratio to entry_price_ratio -1 
    current_value: float = 0.0
    realized_profit: float = 0.0
    unrealized_profit: float = 0.0
    quantity: float = 0.0
    token_age_blocks: int = 0
    token_age_hours: int = 0
    block_number: int = 0
    timestamp: int = 0 
    has_active_position: bool = False
    num_greys: Optional[int] = None 
    num_greens: Optional[int] = None
    token_bribe_amount: Optional[float] = None
    num_bribers: Optional[str] = None 
    scam_probability: Optional[float] = None 
    scam_reason: Optional[str] = None 
    comment: Optional[str] = None 
    position_state: TokenPositionState = TokenPositionState.INIT
    
    def to_dict(self) -> Dict[str, Any]:
        attr = asdict(self)
        attr['position_state'] = self.position_state.value
        return attr

