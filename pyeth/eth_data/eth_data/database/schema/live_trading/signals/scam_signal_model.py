"""
Scam Signal Model

SQLAlchemy model for the scam_signals table in the live_trading schema.
This model consolidates all scam-related signals into one place with clear, specific scam types.
"""

from datetime import datetime
from decimal import Decimal
from typing import Optional, Dict, Any
from sqlalchemy import (
    Column, Integer, String, Numeric, Boolean, 
    TIMESTAMP, JSON, CheckConstraint, 
    UniqueConstraint, Index, text
)
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.dialects.postgresql import JSONB
from pydantic import BaseModel, Field, validator

Base = declarative_base()


class ScamSignal(Base):
    """SQLAlchemy model for scam_signals table"""
    
    __tablename__ = 'scam_signals'
    __table_args__ = (
        UniqueConstraint('pool_address', 'detection_tx_hash', 'scam_type',
                        name='uq_scam_pool_tx_type'),
        CheckConstraint(
            "scam_type IN ('cant_sell', 'high_tax', 'liquidity_drain', 'lp_approval')",
            name='check_scam_type'
        ),
        CheckConstraint(
            "signal_source IN ('mempool', 'blockchain', 'simulation')",
            name='check_signal_source'
        ),
        Index('idx_scam_signals_token', 'token_address'),
        Index('idx_scam_signals_scammer', 'scammer_address'),
        Index('idx_scam_signals_type', 'scam_type'),
        Index('idx_scam_signals_timestamp', 'detection_timestamp'),
        {'schema': 'live_trading'}
    )
    
    # Primary key
    signal_id = Column(Integer, primary_key=True, autoincrement=True)
    
    # Scam type - clear and specific
    scam_type = Column(String(50), nullable=False)
    
    # Core fields
    token_address = Column(String(42), nullable=False)
    pool_address = Column(String(42))  # Optional for some scam types
    scammer_address = Column(String(42), nullable=False)
    detection_timestamp = Column(TIMESTAMP, nullable=False, default=datetime.utcnow)
    detection_tx_hash = Column(String(66), nullable=False)
    
    # Flexible details storage
    scam_details = Column(JSONB)
    """
    Examples:
    - cant_sell: {"buy_tax": 0, "sell_tax": null, "can_buy": true, "can_sell": false}
    - high_tax: {"buy_tax": 5.0, "sell_tax": 99.0}
    - liquidity_drain: {"eth_drained": 4.5, "drain_percentage": 95.0, "remaining_eth": 0.2}
    - lp_approval: {"approved_spender": "0x...", "pool_liquidity_eth": 10.5}
    """
    
    signal_source = Column(String(20), nullable=False, default='mempool')
    created_at = Column(TIMESTAMP, default=datetime.utcnow)


class ScamSignalRequest(BaseModel):
    """Pydantic model for creating scam signals via API"""
    
    scam_type: str = Field(..., description="Type of scam: cant_sell, high_tax, liquidity_drain, lp_approval")
    token_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    pool_address: Optional[str] = Field(None, regex="^0x[a-fA-F0-9]{40}$")
    scammer_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    detection_tx_hash: str = Field(..., regex="^0x[a-fA-F0-9]{64}$")
    detection_timestamp: Optional[datetime] = None
    scam_details: Optional[Dict[str, Any]] = None
    signal_source: str = Field('mempool', description="Source of signal detection")
    
    @validator('scam_type')
    def validate_scam_type(cls, v):
        valid_types = {'cant_sell', 'high_tax', 'liquidity_drain', 'lp_approval'}
        if v not in valid_types:
            raise ValueError(f"scam_type must be one of {valid_types}")
        return v
    
    @validator('signal_source')
    def validate_signal_source(cls, v):
        valid_sources = {'mempool', 'blockchain', 'simulation'}
        if v not in valid_sources:
            raise ValueError(f"signal_source must be one of {valid_sources}")
        return v
    
    class Config:
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }


class ScamSignalResponse(ScamSignalRequest):
    """Pydantic model for scam signal API responses"""
    
    signal_id: int
    created_at: datetime
    
    class Config:
        orm_mode = True
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }