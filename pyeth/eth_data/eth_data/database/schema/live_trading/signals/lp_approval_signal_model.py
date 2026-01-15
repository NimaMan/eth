"""
LP Approval Signal Model

SQLAlchemy model for the lp_approval_signals table in the live_trading schema.
This model represents signals detected when LP token holders approve routers to spend their tokens,
which is typically the precursor to a rug pull.
"""

from datetime import datetime
from decimal import Decimal
from typing import Optional
from sqlalchemy import (
    Column, BigInteger, String, Numeric, Boolean,
    TIMESTAMP, CheckConstraint,
    UniqueConstraint, Index, text
)
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.dialects.postgresql import JSONB
from pydantic import BaseModel, Field, validator

from eth_data.chain_utils.common_addresses import DEX_POOL_TYPES, DEX_POOL_TYPE_SET

Base = declarative_base()


class LpApprovalSignal(Base):
    """SQLAlchemy model for lp_approval_signals table"""
    
    __tablename__ = 'lp_approval_signals'
    __table_args__ = (
        UniqueConstraint('pool_address', 'detection_tx_hash', 
                        name='uq_lp_approval_pool_tx'),
        CheckConstraint(
            "pool_type IN ('UNISWAP-V2', 'UNISWAP-V3', 'UNISWAP-V4', 'SUSHI-SWAP', 'CURVE', 'BALANCER')",
            name='check_lp_pool_type'
        ),
        CheckConstraint("approval_type IN ('LP_TOKEN', 'NFT_POSITION', 'OTHER')",
                       name='check_approval_type'),
        Index('idx_lp_approval_token', 'token_address'),
        Index('idx_lp_approval_pool', 'pool_address'),
        Index('idx_lp_approval_creator', 'creator_address'),
        Index('idx_lp_approval_spender', 'approved_spender'),
        Index('idx_lp_approval_timestamp', 'detection_timestamp'),
        Index('idx_lp_approval_unlimited', 'is_unlimited_approval',
              postgresql_where=text('is_unlimited_approval = true')),
        {'schema': 'live_trading'}
    )
    
    # Primary key
    signal_id = Column(BigInteger, primary_key=True, autoincrement=True)
    
    # Token and Pool identification
    token_address = Column(String(42), nullable=False)  # The actual token in the pool
    pool_address = Column(String(42), nullable=False)   # LP token address (pool contract)
    pool_type = Column(String(10), nullable=False, default=DEX_POOL_TYPES[0])
    
    # Denominated asset (usually WETH)
    denom_address = Column(String(42), default='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2')
    denom_currency = Column(String(20), default='WETH')
    
    # Approval details
    creator_address = Column(String(42), nullable=False)  # LP token owner (potential rugger)
    approved_spender = Column(String(42), nullable=False)  # Router or other contract
    approval_amount = Column(Numeric(78, 18))
    is_unlimited_approval = Column(Boolean, default=False)
    approval_type = Column(String(20), default='LP_TOKEN')
    previous_allowance = Column(Numeric(78, 18))
    
    # Pool liquidity at time of approval
    pool_liquidity_denom = Column(Numeric(78, 18))
    creator_lp_balance = Column(Numeric(78, 18))
    creator_lp_percentage = Column(Numeric(5, 2))  # % of pool owned by creator
    
    # Detection metadata
    detection_timestamp = Column(TIMESTAMP, nullable=False)
    detection_tx_hash = Column(String(66), nullable=False)
    
    # Risk assessment
    risk_score = Column(Numeric(3, 2))  # 0-1 score
    risk_factors = Column(JSONB)
    """
    Examples:
    - {"known_router": true, "creator_is_deployer": true, "time_since_creation": 3600}
    - {"previous_rugs": 2, "suspicious_pattern": true}
    """
    
    # Status tracking
    signal_source = Column(String(20), default='mempool')
    created_at = Column(TIMESTAMP, default=datetime.utcnow)


class LpApprovalSignalRequest(BaseModel):
    """Pydantic model for creating LP approval signals via API"""
    
    token_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    pool_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    pool_type: str = Field(DEX_POOL_TYPES[0], description="Pool type: canonical DEX identifiers")
    denom_address: str = Field('0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2', regex="^0x[a-fA-F0-9]{40}$")
    denom_currency: str = Field('WETH')
    creator_address: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    approved_spender: str = Field(..., regex="^0x[a-fA-F0-9]{40}$")
    approval_amount: Optional[float] = None
    is_unlimited_approval: bool = False
    approval_type: str = Field('LP_TOKEN')
    previous_allowance: Optional[float] = None
    pool_liquidity_denom: Optional[float] = None
    creator_lp_balance: Optional[float] = None
    creator_lp_percentage: Optional[float] = None
    detection_tx_hash: str = Field(..., regex="^0x[a-fA-F0-9]{64}$")
    detection_timestamp: Optional[datetime] = None
    risk_score: Optional[float] = None
    risk_factors: Optional[dict] = None
    signal_source: str = Field('mempool')
    
    @validator('pool_type')
    def validate_pool_type(cls, v):
        if v not in DEX_POOL_TYPE_SET:
            raise ValueError(f"pool_type must be one of {DEX_POOL_TYPES}")
        return v
    
    @validator('approval_type')
    def validate_approval_type(cls, v):
        valid_types = {'LP_TOKEN', 'NFT_POSITION', 'OTHER'}
        if v not in valid_types:
            raise ValueError(f"approval_type must be one of {valid_types}")
        return v
    
    @validator('creator_lp_percentage')
    def validate_lp_percentage(cls, v):
        if v is not None and (v < 0 or v > 100):
            raise ValueError("creator_lp_percentage must be between 0 and 100")
        return v
    
    @validator('risk_score')
    def validate_risk_score(cls, v):
        if v is not None and (v < 0 or v > 1):
            raise ValueError("risk_score must be between 0 and 1")
        return v
    
    class Config:
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }


class LpApprovalSignalResponse(LpApprovalSignalRequest):
    """Pydantic model for LP approval signal API responses"""
    
    signal_id: int
    created_at: datetime
    
    class Config:
        orm_mode = True
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            Decimal: lambda v: float(v)
        }
