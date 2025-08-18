"""
Live Trading Database Models

This module defines the database schema for live cryptocurrency trading operations.
All tables are designed for real-time position tracking, signal management, and
execution monitoring.

Database: live_trading_db
Schema: public (default)
"""

from sqlalchemy import (
    Column, Integer, BigInteger, String, Boolean, Float, DateTime, Text,
    ForeignKey, Index, UniqueConstraint, CheckConstraint, Numeric
)
from sqlalchemy.dialects.postgresql import UUID, JSONB, TIMESTAMP
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func
import uuid

Base = declarative_base()


# Note: Pool table moved to eth_db to avoid duplication
# Pools are referenced by pool_id from eth_db.pools


class Wallet(Base):
    """
    Trading wallet management.
    Tracks all wallets used for trading operations.
    """
    __tablename__ = 'wallets'
    
    # Primary key
    id = Column(Integer, primary_key=True)
    wallet_address = Column(String(42), unique=True, nullable=False, index=True)
    
    # Wallet info
    label = Column(String(100))  # Human-readable name
    is_active = Column(Boolean, default=True)
    
    # Strategy assignment
    primary_strategy = Column(String(100))
    
    # Risk limits
    max_position_size_eth = Column(Numeric(20, 10), default=0.1)
    max_positions = Column(Integer, default=10)
    max_daily_loss_eth = Column(Numeric(20, 10), default=1.0)
    
    # Performance tracking
    total_trades = Column(Integer, default=0)
    successful_trades = Column(Integer, default=0)
    total_volume_eth = Column(Numeric(30, 10), default=0)
    total_profit_eth = Column(Numeric(30, 10), default=0)
    
    # Timestamps
    created_at = Column(TIMESTAMP, server_default=func.now())
    last_trade_at = Column(TIMESTAMP)
    
    # Relationships
    positions = relationship("LivePosition", back_populates="wallet")
    signals = relationship("TradeSignal", back_populates="wallet")
    
    __table_args__ = (
        CheckConstraint('max_position_size_eth > 0', name='check_positive_position_size'),
        CheckConstraint('max_positions > 0', name='check_positive_max_positions'),
    )


class LivePosition(Base):
    """
    Real-time position tracking for each wallet-token-pool combination.
    Represents the current state of a trading position.
    """
    __tablename__ = 'live_positions'
    
    # Primary key
    id = Column(BigInteger, primary_key=True)
    
    # Position identity
    wallet_id = Column(Integer, ForeignKey('wallets.id'), nullable=False)
    token_address = Column(String(42), nullable=False)  # References eth_db.tokens
    pool_id = Column(Integer, nullable=False)  # References eth_db.pools.id
    
    # Position state
    position_state = Column(String(20), nullable=False, default='INIT')
    is_active = Column(Boolean, default=True, index=True)
    
    # Entry data
    entry_signal_id = Column(UUID(as_uuid=True), ForeignKey('trade_signals.signal_id'))
    entry_block = Column(BigInteger)
    entry_timestamp = Column(TIMESTAMP)
    entry_price = Column(Numeric(40, 18))  # Token price in ETH
    entry_tx_hash = Column(String(66))
    entry_gas_used = Column(Integer)
    entry_gas_price = Column(Numeric(20, 10))
    
    # Position size
    quantity_tokens = Column(Numeric(40, 18))  # Number of tokens
    quantity_eth = Column(Numeric(20, 10))     # ETH invested
    
    # Exit data
    exit_signal_id = Column(UUID(as_uuid=True), ForeignKey('trade_signals.signal_id'))
    exit_block = Column(BigInteger)
    exit_timestamp = Column(TIMESTAMP)
    exit_price = Column(Numeric(40, 18))
    exit_tx_hash = Column(String(66))
    exit_gas_used = Column(Integer)
    exit_gas_price = Column(Numeric(20, 10))
    
    # Performance metrics
    current_price = Column(Numeric(40, 18))
    current_value_eth = Column(Numeric(20, 10))
    realized_profit_eth = Column(Numeric(20, 10), default=0)
    unrealized_profit_eth = Column(Numeric(20, 10), default=0)
    roi_percent = Column(Numeric(10, 4), default=0)
    
    # Strategy info
    strategy_name = Column(String(100))
    
    # Risk metrics
    max_drawdown_percent = Column(Numeric(10, 4), default=0)
    time_in_position_hours = Column(Numeric(10, 2))
    
    # Timestamps
    created_at = Column(TIMESTAMP, server_default=func.now())
    updated_at = Column(TIMESTAMP, server_default=func.now(), onupdate=func.now())
    
    # Relationships
    wallet = relationship("Wallet", back_populates="positions")
    # Note: pool relationship would cross databases, handled in application
    entry_signal = relationship("TradeSignal", foreign_keys=[entry_signal_id])
    exit_signal = relationship("TradeSignal", foreign_keys=[exit_signal_id])
    snapshots = relationship("PositionSnapshot", back_populates="position")
    
    __table_args__ = (
        # Unique constraint: one active position per wallet+pool combination
        UniqueConstraint('wallet_id', 'pool_id', 'is_active',
                        name='uq_active_position_per_pool'),
        Index('idx_position_wallet_active', 'wallet_id', 'is_active'),
        Index('idx_position_state', 'position_state'),
        Index('idx_position_token', 'token_address'),
        Index('idx_position_pool', 'pool_id'),
        CheckConstraint("position_state IN ('INIT', 'BUY_SUBMITTED', 'BUY_CONFIRMED', "
                       "'SELL_SUBMITTED', 'SELL_CONFIRMED', 'FAILED', 'CANCELLED', 'SCAMMED')",
                       name='check_valid_position_state'),
    )


class TradeSignal(Base):
    """
    Trading signals sent to eth_kartal for execution.
    Complete audit trail of all trading decisions.
    """
    __tablename__ = 'trade_signals'
    
    # Primary key
    signal_id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    
    # Signal identity
    wallet_id = Column(Integer, ForeignKey('wallets.id'), nullable=False)
    token_address = Column(String(42), nullable=False)  # References eth_db.tokens
    pool_id = Column(Integer, nullable=False)  # References eth_db.pools.id
    
    # Trade parameters
    action = Column(String(10), nullable=False)  # BUY, SELL
    amount_eth = Column(Numeric(20, 10))        # For buys
    amount_tokens = Column(Numeric(40, 18))     # For sells
    max_slippage_percent = Column(Numeric(5, 2), default=3.0)
    deadline_timestamp = Column(TIMESTAMP)
    
    # Strategy info
    strategy_name = Column(String(100), nullable=False)
    strategy_version = Column(String(20))
    
    # Execution tracking
    status = Column(String(20), default='PENDING')
    submission_timestamp = Column(TIMESTAMP, server_default=func.now())
    confirmation_timestamp = Column(TIMESTAMP)
    
    # Execution results
    tx_hash = Column(String(66), index=True)
    actual_amount_eth = Column(Numeric(20, 10))
    actual_amount_tokens = Column(Numeric(40, 18))
    gas_used = Column(Integer)
    gas_price = Column(Numeric(20, 10))
    slippage_percent = Column(Numeric(5, 2))
    
    # Error handling
    error_code = Column(String(50))
    error_message = Column(Text)
    retry_count = Column(Integer, default=0)
    
    # Relationships
    wallet = relationship("Wallet", back_populates="signals")
    # Note: pool relationship would cross databases, handled in application
    executions = relationship("Execution", back_populates="signal")
    
    __table_args__ = (
        Index('idx_signal_wallet_status', 'wallet_id', 'status'),
        Index('idx_signal_timestamp', 'submission_timestamp'),
        Index('idx_signal_tx_hash', 'tx_hash'),
        CheckConstraint("action IN ('BUY', 'SELL')", name='check_valid_action'),
        CheckConstraint("status IN ('PENDING', 'SUBMITTED', 'CONFIRMED', 'FAILED', "
                       "'CANCELLED', 'TIMEOUT')", name='check_valid_signal_status'),
    )


class Execution(Base):
    """
    Detailed execution records from eth_kartal.
    Tracks each execution attempt for a signal.
    """
    __tablename__ = 'executions'
    
    # Primary key
    id = Column(BigInteger, primary_key=True)
    signal_id = Column(UUID(as_uuid=True), ForeignKey('trade_signals.signal_id'), nullable=False)
    
    # Execution attempt
    attempt_number = Column(Integer, nullable=False, default=1)
    status = Column(String(20), nullable=False)
    timestamp = Column(TIMESTAMP, server_default=func.now())
    
    # Transaction details
    tx_hash = Column(String(66), unique=True, index=True)
    block_number = Column(BigInteger, index=True)
    block_timestamp = Column(TIMESTAMP)
    gas_limit = Column(Integer)
    gas_price = Column(Numeric(20, 10))
    gas_used = Column(Integer)
    nonce = Column(Integer)
    
    # Execution path
    router_address = Column(String(42))
    route_path = Column(JSONB)  # Array of pool addresses in route
    
    # Results
    success = Column(Boolean)
    input_amount = Column(Numeric(40, 18))
    output_amount = Column(Numeric(40, 18))
    price_impact_percent = Column(Numeric(10, 4))
    
    # MEV protection
    mev_protected = Column(Boolean, default=False)
    flashbots_bundle_hash = Column(String(66))
    
    # Error details
    revert_reason = Column(Text)
    error_details = Column(JSONB)
    
    # Relationships
    signal = relationship("TradeSignal", back_populates="executions")
    
    __table_args__ = (
        UniqueConstraint('signal_id', 'attempt_number', name='uq_signal_attempt'),
        Index('idx_execution_block', 'block_number'),
        CheckConstraint("status IN ('PENDING', 'SUBMITTED', 'CONFIRMED', 'FAILED', "
                       "'REVERTED', 'TIMEOUT')", name='check_valid_exec_status'),
    )


class PositionSnapshot(Base):
    """
    Periodic snapshots of position states for performance tracking.
    """
    __tablename__ = 'position_snapshots'
    
    # Primary key
    id = Column(BigInteger, primary_key=True)
    position_id = Column(BigInteger, ForeignKey('live_positions.id'), nullable=False)
    
    # Snapshot timing
    block_number = Column(BigInteger, nullable=False, index=True)
    timestamp = Column(TIMESTAMP, nullable=False, index=True)
    
    # Position metrics at snapshot time
    token_price = Column(Numeric(40, 18))
    position_value_eth = Column(Numeric(20, 10))
    unrealized_profit_eth = Column(Numeric(20, 10))
    roi_percent = Column(Numeric(10, 4))
    
    # Pool state
    pool_eth_reserve = Column(Numeric(30, 10))
    pool_token_reserve = Column(Numeric(40, 18))
    pool_fee_tier = Column(Integer)  # For V3/V4
    
    # Market conditions
    gas_price_gwei = Column(Numeric(10, 2))
    eth_price_usd = Column(Numeric(10, 2))
    
    # Relationships
    position = relationship("LivePosition", back_populates="snapshots")
    
    __table_args__ = (
        Index('idx_snapshot_position_time', 'position_id', 'timestamp'),
        Index('idx_snapshot_block', 'block_number'),
    )


class MempoolScamPrediction(Base):
    """
    Scam predictions from mempool analysis.
    Tracks potential scams before they execute.
    Moved from eth_db to live_trading_db for consolidation.
    """
    __tablename__ = 'mempool_scam_predictions'
    
    # Primary key
    id = Column(BigInteger, primary_key=True)
    
    # Prediction target
    token_address = Column(String(42), nullable=False)  # References eth_db.tokens
    pool_id = Column(Integer, nullable=False)  # References eth_db.pools.id
    
    # Prediction details
    prediction_block = Column(BigInteger, nullable=False, index=True)
    prediction_timestamp = Column(TIMESTAMP, server_default=func.now())
    pending_tx_hash = Column(String(66))
    
    # Liquidity analysis
    current_eth_level = Column(Numeric(30, 10), nullable=False)
    simulated_eth_level = Column(Numeric(30, 10), nullable=False)
    eth_threshold = Column(Numeric(30, 10), nullable=False)
    liquidity_removal_percent = Column(Numeric(5, 2))
    
    # Prediction confidence
    scam_probability = Column(Numeric(5, 4), nullable=False)  # 0.0000 to 1.0000
    scam_type = Column(String(50))  # RUG_PULL, HONEYPOT, etc.
    
    # Outcome tracking
    prediction_correct = Column(Boolean)
    actual_outcome = Column(String(50))
    
    # Note: pool relationship would cross databases, handled in application
    
    __table_args__ = (
        Index('idx_scam_pred_token_pool', 'token_address', 'pool_id'),
        Index('idx_scam_pred_block', 'prediction_block'),
        Index('idx_scam_pred_probability', 'scam_probability'),
    )


class StrategyConfig(Base):
    """
    Trading strategy configurations.
    """
    __tablename__ = 'strategy_configs'
    
    # Primary key
    id = Column(Integer, primary_key=True)
    strategy_name = Column(String(100), unique=True, nullable=False)
    
    # Configuration
    is_active = Column(Boolean, default=True)
    version = Column(String(20))
    parameters = Column(JSONB)  # Strategy-specific parameters
    
    # Risk limits
    max_position_size_eth = Column(Numeric(20, 10))
    max_positions = Column(Integer)
    stop_loss_percent = Column(Numeric(5, 2))
    take_profit_percent = Column(Numeric(5, 2))
    
    # Token filters
    min_liquidity_eth = Column(Numeric(20, 10), default=1.0)
    max_token_age_hours = Column(Integer)
    allowed_pool_types = Column(JSONB)  # ['V2', 'V3']
    
    # Timestamps
    created_at = Column(TIMESTAMP, server_default=func.now())
    updated_at = Column(TIMESTAMP, server_default=func.now(), onupdate=func.now())


class PerformanceMetric(Base):
    """
    Aggregated performance metrics for analysis.
    """
    __tablename__ = 'performance_metrics'
    
    # Primary key
    id = Column(BigInteger, primary_key=True)
    
    # Metric identity
    wallet_id = Column(Integer, ForeignKey('wallets.id'), nullable=False)
    strategy_name = Column(String(100))
    
    # Time period
    period_type = Column(String(10), nullable=False)  # HOUR, DAY, WEEK
    period_start = Column(TIMESTAMP, nullable=False)
    period_end = Column(TIMESTAMP, nullable=False)
    
    # Trading activity
    total_trades = Column(Integer, default=0)
    successful_trades = Column(Integer, default=0)
    failed_trades = Column(Integer, default=0)
    
    # Volume metrics
    buy_volume_eth = Column(Numeric(30, 10), default=0)
    sell_volume_eth = Column(Numeric(30, 10), default=0)
    
    # Performance metrics
    gross_profit_eth = Column(Numeric(30, 10), default=0)
    gross_loss_eth = Column(Numeric(30, 10), default=0)
    net_profit_eth = Column(Numeric(30, 10), default=0)
    gas_costs_eth = Column(Numeric(20, 10), default=0)
    
    # Ratios
    win_rate = Column(Numeric(5, 4))  # 0.0000 to 1.0000
    profit_factor = Column(Numeric(10, 4))  # gross_profit / gross_loss
    sharpe_ratio = Column(Numeric(10, 4))
    
    # Risk metrics
    max_drawdown_eth = Column(Numeric(20, 10))
    max_drawdown_percent = Column(Numeric(5, 2))
    
    __table_args__ = (
        UniqueConstraint('wallet_id', 'strategy_name', 'period_type', 'period_start',
                        name='uq_metric_period'),
        Index('idx_metric_wallet_period', 'wallet_id', 'period_type', 'period_start'),
        CheckConstraint("period_type IN ('HOUR', 'DAY', 'WEEK', 'MONTH')",
                       name='check_valid_period_type'),
    )