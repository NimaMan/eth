"""
Trading Enabled Signal Model

SQLAlchemy model for the trading_enabled_signals table in the signals schema.
This model represents signals detected when trading becomes enabled for a token pool.
"""

from datetime import datetime
from decimal import Decimal
from typing import Optional, Dict, Any
from sqlalchemy import (
    Column, BigInteger, String, Numeric, Boolean, 
    TIMESTAMP, JSON, ForeignKey, CheckConstraint, 
    UniqueConstraint, Index, text
)
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship
from pydantic import BaseModel, Field, validator

Base = declarative_base()


class TradingEnabledSignal(Base):
    """SQLAlchemy model for trading_enabled_signals table"""
    
    __tablename__ = 'trading_enabled_signals'
    __table_args__ = (
        UniqueConstraint('pool_address', 'detection_tx_hash', 
                        name='uq_pool_tx'),
        CheckConstraint("pool_type IN ('V2', 'V3', 'V4')", 
                       name='check_pool_type'),
        Index('idx_signals_token_address', 'token_address'),
        Index('idx_signals_pool_address', 'pool_address'),
        Index('idx_signals_detection_timestamp', 'detection_timestamp', postgresql_using='btree'),
        Index('idx_signals_price_ratio', 'price_ratio',
              postgresql_where=text('price_ratio IS NOT NULL')),
        Index('idx_signals_liquidity', 'denom_reserve_at_signal',
              postgresql_where=text('denom_reserve_at_signal > 0')),
        Index('idx_signals_token_pool_time', 'token_address', 'pool_address', 'detection_timestamp'),
        {'schema': 'signals'}
    )
    
    # Primary key
    signal_id = Column(BigInteger, primary_key=True, autoincrement=True)
    
    # Token and Pool identification
    token_address = Column(String(42), ForeignKey('eth_db.tokens.contract_address', ondelete='CASCADE'), nullable=False)
    pool_address = Column(String(42), ForeignKey('eth_db.pools.pool_address', ondelete='CASCADE'), nullable=False)
    pool_type = Column(String(10), nullable=False)
    denom_address = Column(String(42), nullable=False)
    denom_currency = Column(String(10))  # WETH, USDC, USDT, etc.
    
    # Signal detection info
    detection_timestamp = Column(TIMESTAMP, nullable=False, server_default=text('CURRENT_TIMESTAMP'))
    detection_tx_hash = Column(String(66), ForeignKey('eth_db.transactions.tx_hash', ondelete='CASCADE'), nullable=False)
    
    # Price and liquidity at time of signal
    price_ratio = Column(Numeric(10, 4))  # Ratio to initial price
    denom_reserve_at_signal = Column(Numeric(30, 18))
    token_reserve_at_signal = Column(Numeric(30, 18))
    
    # Tax information at signal time
    buy_tax_at_signal = Column(Numeric(5, 2))  # Percentage (0-100)
    sell_tax_at_signal = Column(Numeric(5, 2))  # Percentage (0-100)
    
    # Token metadata at signal time
    total_supply = Column(Numeric(78, 0))
    owner_address = Column(String(42))
    creator_address = Column(String(42), nullable=False)  # Who enabled trading
    
    # Signal source and processing
    signal_source = Column(String(20), default='mempool')  # mempool, confirmed, manual
