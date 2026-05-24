//! Pure Ethereum alpha/trading domain contracts.
//!
//! This crate intentionally contains no external stores, ZMQ, RPC, signing, or
//! runtime orchestration code. It defines the shared language used by the engine,
//! strategies, backtests, risk processors, stores, and execution adapters.

pub mod amount;
pub mod decision_rationale;
pub mod error;
pub mod execution;
pub mod ids;
pub mod market;
pub mod mempool_entry;
pub mod order;
pub mod portfolio;
pub mod position;
pub mod risk;
pub mod store;
pub mod strategy;
pub mod time;

pub use amount::{Amount, DecimalAmount};
pub use decision_rationale::{DecisionReason, ReasonCategory};
pub use error::{AlphaCoreError, Result};
pub use execution::{ExecutionAdapter, ExecutionReport, ExecutionStatus, MinedExecutionEvidence};
pub use ids::{
    BlockHash, BlockNumber, ChainId, OrderId, PortfolioId, PositionId, StrategyName, TokenPoolId,
    TxHash, WalletId,
};
pub use market::{MarketEvent, PoolProtocol, PoolSnapshot, TokenSnapshot};
pub use mempool_entry::{
    MempoolEntryEvidence, MempoolEntryViability, MempoolProjectedPool, MempoolVaultBuySimulation,
    MEMPOOL_ENTRY_EVIDENCE_KEY, MEMPOOL_ENTRY_EVIDENCE_VERSION,
};
pub use order::{OrderIntent, OrderSide, OrderStatus};
pub use portfolio::{PortfolioLimits, PortfolioState};
pub use position::{Position, PositionKey, PositionSnapshot, PositionState};
pub use risk::{RiskDecision, RiskEvent, RiskKind, RiskPolicy, RiskSeverity};
pub use store::TradingStore;
pub use strategy::{Strategy, StrategyContext, StrategyDecision};
