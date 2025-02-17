"""
Shared types and enums for portfolio management

Objective:
---------
Define common data structures and types used across the portfolio management system
"""

from enum import Enum
from dataclasses import dataclass, asdict
from typing import Optional, List, Tuple
from datetime import datetime


# Database column definitions
TOKEN_POSITION_COLUMNS: List[str] = [
    'strategy_run_id',
    'token_address',
    'symbol',
    'entry_Xprice',
    'current_Xprice',
    'Xprice',
    'purchase_value',
    'current_value',
    'realized_profit',
    'unrealized_profit',
    'quantity',
    'token_age_blocks',
    'token_age_hours',
    'block_number',
    'last_updated_time',
    'entry_block',
    'has_active_position',
    'position_state',
    'exit_block',
    'scam_probability',
    'scam_reason',
    'num_greys',
    'num_greens'
]


# For SQL generation
def get_token_position_columns_sql() -> Tuple[str, str]:
    """Returns properly quoted column names"""
    quoted_columns = [f'"{col}"' for col in TOKEN_POSITION_COLUMNS]  # Add quotes
    columns = ', '.join(quoted_columns)
    placeholders = ', '.join(['%s'] * len(TOKEN_POSITION_COLUMNS))
    return columns, placeholders


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
    entry_Xprice: float  # relative price of the token at entry 
    current_Xprice: float  # relative price of the token at current time
    Xprice: float  # ratio current_Xprice to entry_Xprice
    purchase_value: float
    current_value: float
    realized_profit: float
    unrealized_profit: float
    quantity: float
    token_age_blocks: int
    token_age_hours: int
    block_number: int
    last_updated_time: datetime
    entry_block: int
    has_active_position: bool
    position_state: TokenPositionState
    token_address: str
    exit_block: int = None
    scam_probability: float = None 
    scam_reason: str = None 
    num_greys: int = None 
    num_greens: int = None

    def to_dict(self):
        # Return a dictionary representation of the TokenPositionData
        attributes = asdict(self)
        attributes['position_state'] = attributes['position_state'].value
        return attributes

    @classmethod
    def from_dict(cls, data: dict) -> 'TokenPositionData':
        """
        Create a TokenPositionData from a dictionary.
        This method handles empty strings in numeric fields or booleans by converting them 
        to appropriate default types so that the instance can be created without errors.
        """
        # Helper functions for conversion
        def to_float(value):
            if value in [None, '']:
                return 0.0
            try:
                return float(value)
            except (ValueError, TypeError):
                return 0.0

        def to_int(value):
            if value in [None, '']:
                return 0
            try:
                return int(value)
            except (ValueError, TypeError):
                return 0

        def to_bool(value):
            if value in [None, '', 0, 0.0]:
                return False
            try:
                return bool(int(value))
            except (ValueError, TypeError):
                return False

        def to_datetime(value):
            if value in [None, '']:
                return datetime.fromtimestamp(0)
            if isinstance(value, datetime):
                return value
            try:
                return datetime.fromtimestamp(float(value))
            except Exception:
                return datetime.fromtimestamp(0)

        processed = {}
        processed['symbol'] = data.get('symbol', '') or 'UNKNOWN'
        processed['entry_Xprice'] = to_float(data.get('entry_Xprice'))
        processed['current_Xprice'] = to_float(data.get('current_Xprice'))
        processed['Xprice'] = to_float(data.get('Xprice'))
        processed['purchase_value'] = to_float(data.get('purchase_value'))
        processed['current_value'] = to_float(data.get('current_value'))
        processed['realized_profit'] = to_float(data.get('realized_profit'))
        processed['unrealized_profit'] = to_float(data.get('unrealized_profit'))
        processed['quantity'] = to_float(data.get('quantity'))
        processed['token_age_blocks'] = to_int(data.get('token_age_blocks'))
        processed['token_age_hours'] = to_int(data.get('token_age_hours'))
        processed['block_number'] = to_int(data.get('block_number'))
        processed['last_updated_time'] = to_datetime(data.get('last_updated_time'))
        processed['entry_block'] = to_int(data.get('entry_block'))
        processed['has_active_position'] = to_bool(data.get('has_active_position'))

        # Convert position state string to TokenPositionState enum
        raw_state = data.get('position_state', 'Init') or 'Init'
        try:
            processed['position_state'] = TokenPositionState(raw_state)
        except ValueError:
            processed['position_state'] = TokenPositionState.INIT

        processed['token_address'] = data.get('token_address', '')

        # Handle optional fields, providing defaults if necessary
        exit_block_val = data.get('exit_block')
        processed['exit_block'] = to_int(exit_block_val) if exit_block_val not in [None, ''] else None
        processed['scam_probability'] = to_float(data.get('scam_probability'))
        processed['scam_reason'] = data.get('scam_reason') if data.get('scam_reason') not in ['', None] else None
        processed['num_greys'] = to_int(data.get('num_greys'))
        processed['num_greens'] = to_int(data.get('num_greens'))

        return cls(**processed)
    
    
@dataclass
class TradeSignal:
    token_address: str
    decision: TradingDecision
    quantity: float
    strategy_name: str

