from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, String, Float, DateTime, Boolean, ForeignKey, Enum, JSON
from eth_portfolio_manager.core.data_models import TokenPositionState


Base = declarative_base()


class StrategyRun(Base):
    """Tracks strategy configuration and metadata"""
    __tablename__ = 'strategy_runs'
    
    id = Column(Integer, primary_key=True)
    name = Column(String)          # "BuyScamStrategy-v2"
    parameters = Column(JSON)      # {"position_size": 0.01, "scam_ratio": 0.6}
    start_block = Column(Integer)
    end_block = Column(Integer)
    created_at = Column(DateTime)


position_state_enum = Enum(
    *[e.value for e in TokenPositionState],
    name='tokenpositionstate'
)


class TokenPosition(Base):
    """Tracks token positions and their evolution"""
    __tablename__ = 'token_positions'
    
    id = Column(Integer, primary_key=True)
    strategy_run_id = Column(Integer, ForeignKey('strategy_runs.id'))
    block_number = Column(Integer)
    token_address = Column(String)
    symbol = Column(String)
    currency = Column(String)
    entry_Xprice = Column(Float)
    current_Xprice = Column(Float)
    Xprice = Column(Float)
    purchase_value = Column(Float)
    current_value = Column(Float)
    realized_profit = Column(Float)
    unrealized_profit = Column(Float)
    quantity = Column(Float)
    token_age_blocks = Column(Integer)
    token_age_hours = Column(Float)
    trading_enabled_block = Column(Integer)
    trading_enabled_timestamp = Column(Integer)
    last_updated_time = Column(Integer)
    entry_block = Column(Integer)
    exit_block = Column(Integer)
    has_active_position = Column(Boolean)
    position_state = Column(position_state_enum)
    scam_probability = Column(Float)
    scam_reason = Column(String)
    num_greys = Column(Integer)
    num_greens = Column(Integer)
