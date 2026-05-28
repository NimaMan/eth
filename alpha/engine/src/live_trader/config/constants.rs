pub(super) const POOL_UPDATE_SOURCE: &str = "pool_update";
pub(super) const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";
pub(super) const MINED_POOL_RISK_SOURCE: &str = POOL_UPDATE_SOURCE;
pub(super) const LEGACY_MINED_POOL_RISK_SOURCE: &str = "mined_pool_update";
pub(super) const LEGACY_RETH_MINED_POOL_RISK_SOURCE: &str = "reth_mined_pool_update";
pub(super) const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";
pub(super) const ALPHA_TRADER_LOG_DIR_CONFIG: &str = "ALPHA_TRADER_LOG_DIR";
pub(super) const ALPHA_LIVE_MEMPOOL_SINCE_DAYS_CONFIG: &str = "ALPHA_LIVE_MEMPOOL_SINCE_DAYS";
pub(super) const ALPHA_LIVE_SIGNAL_LIMIT_CONFIG: &str = "ALPHA_LIVE_SIGNAL_LIMIT";
pub(super) const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
pub(super) const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
pub(super) const RETH_HTTP_RPC_CONFIG: &str = "RETH_HTTP_RPC";
pub(super) const DEFAULT_ALPHA_TRADER_LOG_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/logs/alpha_trader";
pub(super) const DEFAULT_KARTAL_URL: &str = "http://127.0.0.1:5004";
pub(super) const DEFAULT_KARTAL_TOKEN_ENV: &str = "ETH_TX_EXECUTOR_API_TOKEN";
pub(super) const DEFAULT_LIVE_REAL_FROM: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
pub(super) const DEFAULT_UNISWAP_V2_TRADING_VAULT: &str =
    "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
pub(super) const LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH: &str = "0.555";
pub(super) const LIVE_REAL_MEMPOOL_SIGNAL_POLL_INTERVAL_MS: u64 = 250;
pub(super) const LIVE_UPDATE_WAIT_TIMEOUT_MS: u64 = 30_000;
pub(super) const LIVE_POLL_ERROR_RETRY_MS: u64 = 1_000;
pub(super) const CHAIN_SERVER_PREFLIGHT_TIMEOUT_SECS: u64 = 240;
pub(super) const CHAIN_SERVER_PREFLIGHT_POLL_INTERVAL_MS: u64 = 2_000;
pub(super) const CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON_CODE: &str =
    "chain_sim.live_backtest.skip_mempool_trading_enabled";
