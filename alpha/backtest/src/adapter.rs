use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use eth_alpha_core::ids::PoolAddress;
use eth_alpha_core::market::PoolSnapshot;

/// Trait for adapters that can be used by the backtest runner.
///
/// The EVM-backed adapter implements this so the runner can update pool
/// snapshots and current block number before each event.
pub trait BacktestAdapter {
    /// Shared handle to the live pool map.
    fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>;

    /// Shared handle to the current block number.
    fn current_block(&self) -> Arc<AtomicU64>;
}
