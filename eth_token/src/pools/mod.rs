//! AMM pool state machines and liquidity/trading analysis.

pub mod balancer;
pub mod base;
pub mod curve;
pub mod data_models;
pub mod pancakeswap;
pub mod reserves;
pub mod scam_mechanism;
pub mod sushiswap;
pub mod tax;
pub(crate) mod trading_failure;
pub mod trading_simulation;
pub mod uniswap;

pub use balancer::{BalancerPool, BalancerPoolToken, BALANCER_V2_PROTOCOL};
pub use base::{BasePool, BasePoolConfig, PoolIdentity, TradingStatus, TradingStatusSnapshot};
pub use curve::{CurvePool, CurvePoolToken, CURVE_V1_PROTOCOL};
pub use data_models::{PoolLifecycle, PoolLiquiditySnapshot, PoolRuntimeState};
pub use pancakeswap::{
    PancakeSwapV2Pool, PancakeSwapV3Pool, PANCAKESWAP_V2_FACTORY, PANCAKESWAP_V2_PROTOCOL,
    PANCAKESWAP_V3_PROTOCOL,
};
pub use reserves::{PoolReserveTracker, ReserveSnapshot};
pub use scam_mechanism::{
    PoolScamMechanism, SCAM_DIRECT_LP_LIQUIDITY_REMOVAL, SCAM_PAIR_BALANCE_BACKDOOR_DRAIN,
    SCAM_PRIVILEGED_SELLER_RESERVE_DRAIN, SCAM_RESERVE_DUMP_DRAIN, SCAM_UNKNOWN_RESERVE_DRAIN,
};
pub use sushiswap::{
    SushiSwapV2Pool, SushiSwapV3Pool, SUSHISWAP_V2_FACTORY, SUSHISWAP_V2_PROTOCOL,
    SUSHISWAP_V3_PROTOCOL,
};
pub use tax::TaxBucket;
pub use trading_simulation::{PoolTradingSimulationConfig, PoolTradingSimulationOutcome};
pub use uniswap::{
    ApprovalInfo, LPApprovalEvent, LPApprovalSnapshot, LPHolderInfo, LPHolderSnapshot,
    LPTokenTracker, LPTransferEvent, UniswapV2BurnEvent, UniswapV2MintEvent, UniswapV2Pool,
    UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV2TransactionEvents, UniswapV2TxContext,
    UniswapV3Pool, UniswapV4Pool, UniswapV4PoolKey,
};
