use super::*;

#[derive(Debug)]
pub(super) struct Args {
    pub(super) poll_interval_ms: u64,
    pub(super) mempool_since_days: i64,
    pub(super) signal_limit: i64,
    pub(super) buy_wei: String,
    pub(super) min_liquidity_eth: String,
    pub(super) min_liquidity_usd: String,
    pub(super) run_id: Option<String>,
    pub(super) disable_entry: bool,
    pub(super) replay_current: bool,
    pub(super) once: bool,
    pub(super) max_entry_pools: Option<usize>,
    pub(super) entry_bankroll_eth: Option<String>,
    pub(super) max_hold_blocks: Option<u64>,
    pub(super) stop_loss_ratio: Option<String>,
    pub(super) take_profit_ratio: Option<String>,
    pub(super) strategy_set: Option<String>,
}

#[derive(Debug)]
pub(super) struct RealExecutionArgs {
    pub(super) kartal_url: String,
    pub(super) kartal_token_env: String,
    pub(super) live_real_from: String,
    pub(super) live_real_vault_address: String,
    pub(super) live_real_shadow_priority_fee_gwei: String,
    pub(super) live_real_shadow_max_fee_gwei: String,
    pub(super) live_real_shadow_predicted_base_fee_gwei: String,
}

#[derive(Debug, Parser)]
struct LiveCommonCli {
    #[arg(long, default_value_t = 2_000)]
    poll_interval_ms: u64,

    #[arg(long, default_value_t = 14)]
    mempool_since_days: i64,

    #[arg(long, default_value_t = 200)]
    signal_limit: i64,

    #[arg(long = "buy-wei", default_value = "10000000000000000")]
    buy_wei: String,

    #[arg(long, default_value = "0.5")]
    min_liquidity_eth: String,

    #[arg(long, default_value = "1000")]
    min_liquidity_usd: String,

    #[arg(long)]
    run_id: Option<String>,

    /// Disable new entries while still allowing existing live positions to exit.
    #[arg(long, default_value_t = false)]
    disable_entry: bool,

    /// Process the current token-server snapshot immediately instead of only priming watermarks.
    #[arg(long, default_value_t = false)]
    replay_current: bool,

    #[arg(long, default_value_t = false)]
    once: bool,

    /// Hard cap on distinct pools each live strategy may buy in this run.
    /// Restored bought pools count toward the cap.
    #[arg(long)]
    max_entry_pools: Option<usize>,

    /// Starting ETH bankroll each live strategy may deploy into entries.
    /// Buys consume it; confirmed sells replenish it; profits can be redeployed.
    #[arg(long = "entry-bankroll-eth")]
    entry_bankroll_eth: Option<String>,

    /// Max hold active pool-update blocks: force sell after this many distinct
    /// pool-update blocks while the position is open.
    /// Disabled by default.
    #[arg(long)]
    max_hold_blocks: Option<u64>,

    /// Stop-loss ratio: sell if price drops to this fraction of entry price.
    /// E.g., 0.7 = sell at -30% loss. Disabled by default.
    #[arg(long)]
    stop_loss_ratio: Option<String>,

    /// Take-profit ratio: sell if price rises to this multiple of entry price.
    /// E.g., 3.0 = sell at +200% profit. Disabled by default.
    #[arg(long)]
    take_profit_ratio: Option<String>,

    /// Register a named live strategy set instead of the default single strategy.
    /// Examples: `alpha11-live-univ2-lp30-price-to-initial-lte1p5-pool-update-block-hold-sweep`,
    /// `mempool-live-exits`.
    #[arg(long = "strategy-set", alias = "strategy-suite")]
    strategy_set: Option<String>,
}

#[derive(Debug, Parser)]
struct LiveRealCli {
    #[command(flatten)]
    common: LiveCommonCli,

    #[command(flatten)]
    real: LiveRealOnlyCli,
}

#[derive(Debug, Parser)]
struct LiveRealOnlyCli {
    /// Kartal base URL used only by real live execution.
    #[arg(long, default_value = DEFAULT_KARTAL_URL)]
    kartal_url: String,

    /// Env var containing the Kartal bearer token. KARTAL_API_TOKEN is also
    /// tried as a fallback.
    #[arg(long, default_value = DEFAULT_KARTAL_TOKEN_ENV)]
    kartal_token_env: String,

    /// EOA/from address that Kartal policy and the vault owner must allow.
    #[arg(long, default_value = DEFAULT_LIVE_REAL_FROM)]
    live_real_from: String,

    /// Deployed Uniswap V2 trading vault used by the real priority-sell route.
    #[arg(long, default_value = DEFAULT_UNISWAP_V2_TRADING_VAULT)]
    live_real_vault_address: String,

    /// Temporary dry-run gas rank candidate until the production gas-rank
    /// provider is wired.
    #[arg(long, default_value = "40")]
    live_real_shadow_priority_fee_gwei: String,

    /// Temporary dry-run max fee candidate until the production gas-rank
    /// provider is wired.
    #[arg(long, default_value = "50")]
    live_real_shadow_max_fee_gwei: String,

    /// Temporary dry-run predicted base fee until the production gas-rank
    /// provider is wired.
    #[arg(long, default_value = "10")]
    live_real_shadow_predicted_base_fee_gwei: String,
}

pub(super) fn parse_live_backtest_args() -> Args {
    Args::from(LiveCommonCli::parse())
}

pub(super) fn parse_live_real_args() -> (Args, RealExecutionArgs) {
    let cli = LiveRealCli::parse();
    (Args::from(cli.common), RealExecutionArgs::from(cli.real))
}

impl From<LiveCommonCli> for Args {
    fn from(common: LiveCommonCli) -> Self {
        Self {
            poll_interval_ms: common.poll_interval_ms,
            mempool_since_days: common.mempool_since_days,
            signal_limit: common.signal_limit,
            buy_wei: common.buy_wei,
            min_liquidity_eth: common.min_liquidity_eth,
            min_liquidity_usd: common.min_liquidity_usd,
            run_id: common.run_id,
            disable_entry: common.disable_entry,
            replay_current: common.replay_current,
            once: common.once,
            max_entry_pools: common.max_entry_pools,
            entry_bankroll_eth: common.entry_bankroll_eth,
            max_hold_blocks: common.max_hold_blocks,
            stop_loss_ratio: common.stop_loss_ratio,
            take_profit_ratio: common.take_profit_ratio,
            strategy_set: common.strategy_set,
        }
    }
}

impl From<LiveRealOnlyCli> for RealExecutionArgs {
    fn from(real: LiveRealOnlyCli) -> Self {
        Self {
            kartal_url: real.kartal_url,
            kartal_token_env: real.kartal_token_env,
            live_real_from: real.live_real_from,
            live_real_vault_address: real.live_real_vault_address,
            live_real_shadow_priority_fee_gwei: real.live_real_shadow_priority_fee_gwei,
            live_real_shadow_max_fee_gwei: real.live_real_shadow_max_fee_gwei,
            live_real_shadow_predicted_base_fee_gwei: real.live_real_shadow_predicted_base_fee_gwei,
        }
    }
}
