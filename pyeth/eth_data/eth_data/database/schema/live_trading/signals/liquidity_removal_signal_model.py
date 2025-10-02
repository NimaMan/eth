"""
Liquidity Removal Signal Model

SQLAlchemy model for the liquidity_removal_signals table in the live_trading schema.
This model represents signals detected when liquidity is removed from pools.
"""

from datetime import datetime
from decimal import Decimal
from typing import Optional
from sqlalchemy import (
    Column, BigInteger, String, Numeric, Boolean,
    TIMESTAMP, JSON, CheckConstraint,
    UniqueConstraint, Index, text
)
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.dialects.postgresql import JSONB
from pydantic import BaseModel, Field, validator

from eth_data.chain_utils.common_addresses import DEX_POOL_TYPES, DEX_POOL_TYPE_SET

Base = declarative_base()


class LiquidityRemovalSignal(Base):
    """SQLAlchemy model for liquidity_removal_signals table"""
    
    __tablename__ = 'liquidity_removal_signals'
    __table_args__ = (
        UniqueConstraint('pool_address', 'detection_tx_hash', 
                        name='uq_liquidity_removal_pool_tx'),
        CheckConstraint(
            "pool_type IN ('UNISWAP-V2', 'UNISWAP-V3', 'UNISWAP-V4', 'SUSHI-SWAP', 'CURVE', 'BALANCER')",
            name='check_liquidity_pool_type'
        ),
        CheckConstraint("removal_type IN ('LIQUIDITY_REMOVAL', 'RUG_PULL', 'PARTIAL_REMOVAL')",
                       name='check_removal_type'),
        Index('idx_liquidity_removal_token', 'token_address'),
        Index('idx_liquidity_removal_pool', 'pool_address'),
        Index('idx_liquidity_removal_remover', 'remover_address'),
        Index('idx_liquidity_removal_timestamp', 'detection_timestamp'),
        Index('idx_liquidity_removal_drain_pct', 'drain_percentage',
              postgresql_where=text('drain_percentage >= 60')),
        {'schema': 'live_trading'}
    )
    
    # Primary key
    signal_id = Column(BigInteger, primary_key=True, autoincrement=True)
    
    # Token and Pool identification
    token_address = Column(String(42), nullable=False)
    pool_address = Column(String(42), nullable=False)
    pool_type = Column(String(10), nullable=False)
    
    # Removal details
    remover_address = Column(String(42), nullable=False)
    removal_type = Column(String(50), nullable=False)
    function_name = Column(String(100))  # removeLiquidityETH, etc.
    
    # Amounts
    liquidity_removed_denom = Column(Numeric(78, 18))
    remaining_liquidity_denom = Column(Numeric(78, 18))
    
    # Detection metadata
    detection_timestamp = Column(TIMESTAMP, nullable=False)
    detection_tx_hash = Column(String(66), nullable=False)
    
    # Additional details in JSON
    removal_details = Column(JSONB)
    """
    Examples:
    - {"lp_tokens_burned": "1000000", "recipient": "0x..."}
    - {"permit_used": true, "deadline": 1234567890}
    """
    
    # Status tracking
    is_rug_pull = Column(Boolean, default=False)
    signal_source = Column(String(20), default='mempool')
    created_at = Column(TIMESTAMP, default=datetime.utcnow)


class LiquidityRemovalSignalRequest(BaseModel):
    """Pydantic model for creating liquidity removal signals via API"""
    
    token_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    pool_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    pool_type: str = Field(DEX_POOL_TYPES[0], description="Pool type: canonical DEX identifiers")
    remover_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    removal_type: str = Field('LIQUIDITY_REMOVAL')
    function_name: Optional[str] = None
    liquidity_removed_denom: Optional[float] = None
    remaining_liquidity_denom: Optional[float] = None
    detection_tx_hash: str = Field(..., regex="^0x[a-fA-F0-9]{64}$")
    detection_timestamp: Optional[datetime] = None
    removal_details: Optional[dict] = None
    is_rug_pull: bool = False
    signal_source: str = Field('mempool')
    
    @validator('pool_type')
    def validate_pool_type(cls, v):
        if v not in DEX_POOL_TYPE_SET:
            raise ValueError(f"pool_type must be one of {DEX_POOL_TYPES}")
        return v
    
    @validator('removal_type')
    def validate_removal_type(cls, v):
        valid_types = {'LIQUIDITY_REMOVAL', 'RUG_PULL', 'PARTIAL_REMOVAL'}
        if v not in valid_types:
            raise ValueError(f"removal_type must be one of {valid_types}")
        return v
    
    
    class Config:
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }


class LiquidityRemovalSignalResponse(LiquidityRemovalSignalRequest):
    """Pydantic model for liquidity removal signal API responses"""
    
    signal_id: int
    created_at: datetime
    
    class Config:
        orm_mode = True
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }
