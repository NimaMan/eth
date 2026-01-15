"""
Backtest Database Models

Objective:
----------
Store token positions with support for multiple pools per token. The token_positions table
now includes a pool_address column to uniquely identify positions across different pools.

Algorithmic Steps:
------------------
1. A StrategyRun is defined to track high-level strategy metadata.
2. A TokenPosition table references a run via strategy_run_id. 
3. The token_address and pool_address together provide a unique lookup key per position.
4. The 'token_position' (JSONB) holds the complete TokenPosition data.
"""

from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, String, JSON, DateTime
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy import ForeignKey, UniqueConstraint

Base = declarative_base()


class StrategyRun(Base):
    """Tracks strategy configuration and metadata"""
    __tablename__ = 'strategy_runs'
    
    id = Column(Integer, primary_key=True)
    name = Column(String)
    parameters = Column(JSON)
    start_block = Column(Integer)
    end_block = Column(Integer)
    created_at = Column(DateTime) 


class TokenPosition(Base):
    """Tracks each token position as a JSONB object with support for multiple pools"""
    __tablename__ = 'token_positions'
    
    id = Column(Integer, primary_key=True)
    strategy_run_id = Column(Integer, ForeignKey('strategy_runs.id'))
    token_address = Column(String, nullable=False)
    pool_address = Column(String, nullable=True)  # Added pool_address column
    currency = Column(String, nullable=True)      # Added currency column for easier querying
    token_position = Column(JSONB, nullable=True)
    
    # Create a unique constraint on the combination of strategy_run_id, token_address, and pool_address
    __table_args__ = (
        UniqueConstraint('strategy_run_id', 'token_address', 'pool_address', name='uix_token_pool_strategy'),
    )
