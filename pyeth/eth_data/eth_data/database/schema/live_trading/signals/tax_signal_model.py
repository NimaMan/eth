"""
Tax Signal Model

SQLAlchemy model for the tax_signals table in the signals schema.
This model represents signals detected when taxes exceed thresholds or tokens are honeypots.
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

from eth_data.chain_utils.common_addresses import DEX_POOL_TYPES, DEX_POOL_TYPE_SET

Base = declarative_base()


class TaxSignal(Base):
    """SQLAlchemy model for tax_signals table"""
    
    __tablename__ = 'tax_signals'
    __table_args__ = (
        UniqueConstraint('pool_address', 'detection_tx_hash', 
                        name='uq_tax_pool_tx'),
        CheckConstraint(
            "pool_type IN ('UNISWAP-V2', 'UNISWAP-V3', 'UNISWAP-V4', 'SUSHI-SWAP', 'CURVE', 'BALANCER')",
            name='check_tax_pool_type'
        ),
        CheckConstraint("signal_type IN ('HighTaxOrHoneypot', 'TaxChange', 'SuspiciousPattern')",
                       name='check_tax_signal_type'),
        Index('idx_tax_signals_token_address', 'token_address'),
        Index('idx_tax_signals_pool_address', 'pool_address'),
        Index('idx_tax_signals_detection_timestamp', 'detection_timestamp', postgresql_using='btree'),
        Index('idx_tax_signals_honeypot', 'cant_sell',
              postgresql_where=text('cant_sell = true')),
        Index('idx_tax_signals_high_taxes', 'buy_tax_exceeds_threshold', 'sell_tax_exceeds_threshold',
              postgresql_where=text('buy_tax_exceeds_threshold = true OR sell_tax_exceeds_threshold = true')),
        Index('idx_tax_signals_token_pool_time', 'token_address', 'pool_address', 'detection_timestamp'),
        {'schema': 'live_trading'}
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
    
    # Signal type and details
    signal_type = Column(String(30), nullable=False)  # HighTaxOrHoneypot, TaxChange, SuspiciousPattern
    signal_details = Column(String(500))  # Human-readable description
    confidence = Column(Numeric(3, 2))  # 0.00 to 1.00
    
    # Tax information
    buy_tax_at_signal = Column(Numeric(5, 2))  # Percentage (0-100)
    sell_tax_at_signal = Column(Numeric(5, 2))  # Percentage (0-100)
    
    # HighTaxOrHoneypot specific flags
    buy_tax_exceeds_threshold = Column(Boolean, default=False)
    sell_tax_exceeds_threshold = Column(Boolean, default=False)
    cant_sell = Column(Boolean, default=False)  # Honeypot indicator
    
    # Creator information
    creator_address = Column(String(42), nullable=False)  # Token creator
    
    # Signal source and processing
    signal_source = Column(String(20), default='mempool')  # mempool, confirmed, manual


class TaxSignalRequest(BaseModel):
    """Pydantic model for tax signal creation requests"""
    
    token_address: str = Field(..., description="Token contract address")
    pool_address: str = Field(..., description="Pool contract address") 
    pool_type: str = Field(default=DEX_POOL_TYPES[0], description="Pool type (canonical DEX identifiers)")
    denom_address: str = Field(..., description="Denomination token address (WETH, etc)")
    denom_currency: Optional[str] = Field(None, description="Denomination currency symbol")
    detection_tx_hash: str = Field(..., description="Transaction that triggered the signal")
    signal_type: str = Field(..., description="Type of tax signal")
    signal_details: Optional[str] = Field(None, description="Signal description")
    confidence: Optional[float] = Field(None, ge=0.0, le=1.0, description="Signal confidence")
    buy_tax_at_signal: Optional[float] = Field(None, description="Buy tax percentage")
    sell_tax_at_signal: Optional[float] = Field(None, description="Sell tax percentage")
    buy_tax_exceeds_threshold: bool = Field(default=False)
    sell_tax_exceeds_threshold: bool = Field(default=False)
    cant_sell: bool = Field(default=False)
    creator_address: str = Field(..., description="Token creator address")
    signal_source: str = Field(default="mempool")

    @validator('token_address', 'pool_address', 'denom_address', 'creator_address')
    def validate_address(cls, v):
        """Validate Ethereum addresses"""
        if not v.startswith('0x') or len(v) != 42:
            raise ValueError('Invalid Ethereum address format')
        return v.lower()

    @validator('detection_tx_hash')
    def validate_tx_hash(cls, v):
        """Validate transaction hash"""
        if not v.startswith('0x') or len(v) != 66:
            raise ValueError('Invalid transaction hash format')
        return v.lower()

    @validator('pool_type')
    def validate_pool_type(cls, v):
        """Validate pool type"""
        if v not in DEX_POOL_TYPE_SET:
            raise ValueError(f"Pool type must be one of {DEX_POOL_TYPES}")
        return v

    @validator('signal_type')
    def validate_signal_type(cls, v):
        """Validate signal type"""
        if v not in ['HighTaxOrHoneypot', 'TaxChange', 'SuspiciousPattern']:
            raise ValueError('Invalid signal type')
        return v


class TaxSignalResponse(TaxSignalRequest):
    """Pydantic model for tax signal responses"""
    
    signal_id: int = Field(..., description="Database record ID")
    detection_timestamp: datetime = Field(..., description="When signal was detected")
    
    class Config:
        from_attributes = True
