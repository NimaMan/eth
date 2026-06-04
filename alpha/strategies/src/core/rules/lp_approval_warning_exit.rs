//! Shared identity and defaults for the LP-approval warning exit strategy.
//!
//! The strategy implementation is still `snipe-all`. Live uses current
//! mempool/chain evidence directly. Historical runs that replay stored
//! `mempool_signal` rows must use the mempool-aware name so they are not
//! mistaken for pure chain-history backtests.

pub const STRATEGY_NAME: &str = "snipe-all-lp-approval-warning-exit";
pub const STRATEGY_LABEL: &str = "Snipe All LP approval warning exit";
pub const SUITE_NAME: &str = "lp-approval-warning-exit";

pub const MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME: &str =
    "snipe-all-mempool-aware-lp-approval-warning-exit";
pub const MEMPOOL_AWARE_HISTORICAL_STRATEGY_LABEL: &str =
    "Snipe All mempool-aware LP approval warning exit";
pub const MEMPOOL_AWARE_HISTORICAL_SUITE_NAME: &str = "mempool-aware-lp-approval-warning-exit";

/// Backtest signal-replay declaration for this diagnostic strategy: with the
/// fundamental always-on liquidity-removal exit, the historical runner replays
/// stored mempool liquidity-removal signals for it. Not a strategy on/off toggle.
pub const EXIT_LIQUIDITY_REMOVAL: bool = true;
pub const EXIT_TAX: bool = false;
pub const EXIT_LP_APPROVAL: bool = true;
pub const EXIT_LP_APPROVAL_CRITICAL_ONLY: bool = false;
pub const EXIT_SCAM: bool = false;
