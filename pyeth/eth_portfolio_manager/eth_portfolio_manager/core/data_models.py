"""
Shared types and enums for portfolio management

Objective:
---------
Define common data structures and types used across the portfolio management system
"""

from enum import Enum
from dataclasses import dataclass
from typing import Optional
from datetime import datetime


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
class TokenPositionData:
    symbol: str
    entry_Xprice: float # relative_price of the token at entry 
    current_Xprice: float # relative_price of the token at current time
    Xprice: float # our current_Xprice to entry_Xprice
    purchase_value: float
    current_value: float
    realized_profit: float
    unrealized_profit: float
    quantity: float
    token_age_blocks: int
    token_age_hours: int
    last_updated_block: int
    last_updated_time: datetime
    entry_block: int
    has_active_position: bool
    position_state: TokenPositionState
    token_address: str
    scam_probability: float = None 
    scam_reason: str = None 
    num_greys: int = None 
    num_greens: int = None
    strategy_name: str = None 
    
    

@dataclass
class TradeSignal:
    token_address: str
    decision: TradingDecision
    quantity: float
    strategy_name: str

