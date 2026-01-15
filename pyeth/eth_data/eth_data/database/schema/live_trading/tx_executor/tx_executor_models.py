"""
Transaction Executor Models for live_trading_db

This module defines SQLAlchemy models based on data actually available
in ETH Kartal, avoiding fields we don't track.
"""

from datetime import datetime
from enum import Enum
import uuid

from sqlalchemy import (
    Column, Integer, String, Boolean, Numeric, DateTime, 
    ForeignKey, Index, Text, BigInteger, func, Enum as SQLEnum
)
from sqlalchemy.dialects.postgresql import UUID, JSONB
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship

Base = declarative_base()


class ExecutionPath(str, Enum):
    PUBLIC_MEMPOOL = "PUBLIC_MEMPOOL"
    FLASHBOTS_BUNDLE = "FLASHBOTS_BUNDLE"
    MULTI_PATH = "MULTI_PATH"


class ExecutionWallet(Base):
    """
    Configuration for execution-enabled wallets.
    Most fields are configuration, not tracked data.
    """
    __tablename__ = 'execution_wallets'
    
    id = Column(Integer, primary_key=True)
    wallet_id = Column(Integer, ForeignKey('wallets.id'), nullable=False, unique=True)
    
    # Configuration
    keystore_path = Column(String(255))
    execution_enabled = Column(Boolean, default=True)
    max_concurrent_executions = Column(Integer, default=10)
    max_gas_per_tx_eth = Column(Numeric(20, 8), default=0.01)
    max_slippage_override = Column(Numeric(5, 4))
    require_flashbots_above_eth = Column(Numeric(20, 8), default=1.0)
    
    # Simple counters (calculated from executions table)
    total_executions = Column(Integer, default=0)
    successful_executions = Column(Integer, default=0)
    failed_executions = Column(Integer, default=0)
    
    # Timestamps
    last_execution_at = Column(DateTime(timezone=True))
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    updated_at = Column(DateTime(timezone=True), server_default=func.now(), onupdate=func.now())
    
    # Relationships
    execution_details = relationship("ExecutionDetail", back_populates="execution_wallet")


class ExecutionDetail(Base):
    """
    Simplified execution details based on data actually available in ETH Kartal.
    Only includes metrics we actually track.
    """
    __tablename__ = 'execution_details'
    
    id = Column(Integer, primary_key=True)
    execution_id = Column(BigInteger, ForeignKey('executions.id'), nullable=False, unique=True)
    wallet_id = Column(Integer, ForeignKey('execution_wallets.id'), nullable=False, index=True)
    
    # Transaction details we actually have
    nonce = Column(BigInteger)
    gas_price = Column(String(78))  # Total gas price in wei
    execution_path = Column(SQLEnum(ExecutionPath))
    
    # The 6 timing metrics we actually track
    alert_to_start_ms = Column(Integer)
    position_check_ms = Column(Integer)
    gas_ranking_ms = Column(Integer)
    price_quote_ms = Column(Integer)
    tx_build_ms = Column(Integer)
    tx_submit_ms = Column(Integer)
    total_execution_ms = Column(Integer)
    
    # Simple MEV tracking
    mev_protected = Column(Boolean, default=False)
    
    # Ranking system output
    expected_position = Column(Integer)  # Position in block
    ranking_confidence = Column(Numeric(5, 4))  # 0.0-1.0
    
    created_at = Column(DateTime(timezone=True), server_default=func.now())
    
    # Relationships
    execution_wallet = relationship("ExecutionWallet", back_populates="execution_details")
    errors = relationship("ExecutionError", back_populates="execution_detail")


class ExecutionError(Base):
    """
    Simplified error tracking based on available data.
    """
    __tablename__ = 'execution_errors'
    
    id = Column(Integer, primary_key=True)
    execution_detail_id = Column(Integer, ForeignKey('execution_details.id'))
    signal_id = Column(UUID(as_uuid=True), ForeignKey('trade_signals.signal_id'), index=True)
    
    # What we actually have
    error_message = Column(Text, nullable=False)
    attempt_number = Column(Integer, default=1)
    
    occurred_at = Column(DateTime(timezone=True), server_default=func.now(), index=True)
    
    # Relationships
    execution_detail = relationship("ExecutionDetail", back_populates="errors")


# Views for analytics
VIEWS_SQL = """
-- Performance summary by wallet
CREATE OR REPLACE VIEW wallet_execution_performance AS
SELECT 
    ew.wallet_id,
    w.wallet_address,
    ew.total_executions,
    ew.successful_executions,
    ew.failed_executions,
    ROUND(ew.successful_executions::numeric / NULLIF(ew.total_executions, 0) * 100, 2) as success_rate,
    AVG(ed.total_execution_ms) as avg_execution_ms,
    COUNT(CASE WHEN ed.mev_protected THEN 1 END) as mev_protected_count
FROM execution_wallets ew
JOIN wallets w ON ew.wallet_id = w.id
LEFT JOIN execution_details ed ON ed.wallet_id = ew.id
WHERE ew.execution_enabled = true
GROUP BY ew.wallet_id, w.wallet_address, ew.total_executions, ew.successful_executions, ew.failed_executions;

-- Recent execution performance
CREATE OR REPLACE VIEW recent_execution_metrics AS
SELECT 
    DATE_TRUNC('hour', ed.created_at) as hour,
    COUNT(*) as executions,
    AVG(ed.total_execution_ms) as avg_latency,
    AVG(ed.gas_ranking_ms) as avg_gas_optimization_ms,
    COUNT(CASE WHEN ed.mev_protected THEN 1 END) as mev_protected
FROM execution_details ed
WHERE ed.created_at > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour DESC;
"""