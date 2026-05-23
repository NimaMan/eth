use super::*;

#[derive(Debug)]
pub(super) struct Args {
    pub(super) poll_interval_ms: Option<u64>,
    pub(super) mempool_since_days: Option<i64>,
    pub(super) signal_limit: Option<i64>,
    pub(super) run_id: Option<String>,
    pub(super) disable_entry: bool,
    pub(super) replay_current: bool,
    pub(super) once: bool,
    pub(super) strategy_set: Option<String>,
}

#[derive(Debug)]
pub(super) struct RealExecutionArgs {
    pub(super) kartal_url: String,
    pub(super) kartal_token_env: String,
    pub(super) live_real_from: String,
    pub(super) live_real_vault_address: String,
    pub(super) allow_public_mempool_live_validation: bool,
}

#[derive(Debug, Parser)]
struct LiveCommonCli {
    /// Optional override for ALPHA_LIVE_TRADER_POLL_INTERVAL_MS in config.env.
    #[arg(long)]
    poll_interval_ms: Option<u64>,

    /// Optional override for ALPHA_LIVE_MEMPOOL_SINCE_DAYS in config.env.
    #[arg(long)]
    mempool_since_days: Option<i64>,

    /// Optional override for ALPHA_LIVE_SIGNAL_LIMIT in config.env.
    #[arg(long)]
    signal_limit: Option<i64>,

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

    /// Register a named live strategy set instead of the default single strategy.
    /// Examples: `alpha11-univ2-lp30-pool-update-block-hold-sweep`,
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

    /// Env var containing the Kartal bearer token.
    #[arg(long, default_value = DEFAULT_KARTAL_TOKEN_ENV)]
    kartal_token_env: String,

    /// EOA/from address that Kartal policy and the vault owner must allow.
    #[arg(long, default_value = DEFAULT_LIVE_REAL_FROM)]
    live_real_from: String,

    /// Deployed Uniswap V2 trading vault used by the real priority-sell route.
    #[arg(long, default_value = DEFAULT_UNISWAP_V2_TRADING_VAULT)]
    live_real_vault_address: String,

    /// Allow Kartal public_mempool only for the explicit Alpha11 hold16
    /// deploy strategy. Without this flag the live trader refuses
    /// any non-dry-run Kartal status.
    #[arg(long, default_value_t = false)]
    allow_public_mempool_live_validation: bool,
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
            run_id: common.run_id,
            disable_entry: common.disable_entry,
            replay_current: common.replay_current,
            once: common.once,
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
            allow_public_mempool_live_validation: real.allow_public_mempool_live_validation,
        }
    }
}
