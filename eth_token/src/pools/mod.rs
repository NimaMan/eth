//! AMM pool state machines and liquidity/trading analysis.

pub mod balancer;
pub mod base;
pub mod curve;
pub mod data_models;
pub mod reserves;
pub mod sushiswap;
pub mod tax;
pub(crate) mod trading_failure;
pub mod trading_simulation;
pub mod uniswap;

pub use balancer::{BalancerPool, BalancerPoolToken, BALANCER_V2_PROTOCOL};
pub use base::{BasePool, BasePoolConfig, PoolIdentity, TradingStatus};
pub use curve::{CurvePool, CurvePoolToken, CURVE_V1_PROTOCOL};
pub use data_models::{PoolLifecycle, PoolLiquiditySnapshot, PoolRuntimeState};
pub use reserves::{PoolReserveTracker, ReserveSnapshot};
pub use sushiswap::{SushiSwapV2Pool, SUSHISWAP_V2_FACTORY, SUSHISWAP_V2_PROTOCOL};
pub use tax::TaxBucket;
pub use trading_simulation::{PoolTradingSimulationConfig, PoolTradingSimulationOutcome};
pub use uniswap::{
    ApprovalInfo, LPApprovalEvent, LPApprovalSnapshot, LPHolderInfo, LPHolderSnapshot,
    LPTokenTracker, LPTransferEvent, UniswapV2BurnEvent, UniswapV2MintEvent, UniswapV2Pool,
    UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV2TransactionEvents, UniswapV2TxContext,
    UniswapV3Pool, UniswapV4Pool, UniswapV4PoolKey, SUSHISWAP_V3_PROTOCOL,
};
