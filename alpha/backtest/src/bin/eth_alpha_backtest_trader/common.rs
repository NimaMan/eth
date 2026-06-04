use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use eth_alpha_backtest::execution::chain_sim::{run_chain_sim_backtest, ChainSimBacktestConfig};
use eth_alpha_backtest::replay::{
    common_nonempty_allowed_protocols, load_events_from_observations, load_events_from_risk_atlas,
    load_events_from_token_state, sort_events_by_block,
};
use eth_alpha_backtest::strategy_suites::{
    build_strategy_specs, strategy_suite_names, validate_historical_signal_replay_names,
    BacktestStrategySpec, StrategySuiteOptions,
};
use eth_alpha_core::amount::Amount;
use eth_alpha_store::PostgresTradingStore;
use eyre::{Result, WrapErr};
use rust_decimal::Decimal;

const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";
const RISK_ATLAS_DATABASE_CONFIG_KEY: &str = "databases.risk_atlas.url";
const TOKEN_STATE_DATABASE_CONFIG_KEY: &str = "databases.token_state.url";

#[derive(Debug, Parser)]
struct Args {
    /// Unique run identifier for this backtest.
    #[arg(long)]
    run_id: Option<String>,

    /// Strategy instance name to persist on orders, positions, and reports.
    /// Defaults to the canonical core-engine identifier; the legacy bare
    /// `snipe-all` deploy identity was removed in the strategies refactor.
    #[arg(long, default_value = eth_strategies::core::spec::CORE_STRATEGY_IMPL)]
    strategy_name: String,

    /// Strategy implementation to instantiate. Defaults to the canonical
    /// core-engine identifier the chain-sim engine resolves.
    #[arg(long, default_value = eth_strategies::core::spec::CORE_STRATEGY_IMPL)]
    strategy_impl: String,

    /// Named historical strategy suite to run in one replay pass.
    #[arg(long)]
    strategy_suite: Option<String>,

    /// Print every valid `--strategy-suite` name and exit. Discovery aid for a
    /// fresh agent driving the pipeline; runs before any config/DB access.
    #[arg(long, default_value_t = false)]
    list_strategy_suites: bool,

    /// Existing live chain-sim run_id to replay from `strategy_observations`
    /// (or `risk_atlas_observations` when prefixed `risk-atlas-`). Required for
    /// a backtest run; not required for `--list-strategy-suites`.
    #[arg(long)]
    replay_run_id: Option<String>,

    /// token_state.scope_id whose mined terminal/custody pool events should be overlaid.
    #[arg(long)]
    token_state_scope: Option<String>,

    /// Buy amount in wei (also used as sell amount for snipe-all).
    #[arg(long, default_value = "10000000000000000")]
    buy_amount_wei: String,

    /// Minimum ETH/WETH reserve for a pool to be eligible.
    #[arg(long, default_value = "0.5")]
    min_liquidity_eth: String,

    /// Minimum stable-denom reserve (USDC/USDT/DAI) for eligibility.
    #[arg(long, default_value = "1000")]
    min_liquidity_usd: String,

    /// Skip primed/warmup observations (suppress_events=true).
    /// Live trader skips these; enabling makes backtest apples-to-apples.
    #[arg(long, default_value_t = false)]
    skip_primed: bool,

    /// Start block (inclusive). If not set, starts from first observation.
    #[arg(long)]
    from_block: Option<u64>,

    /// End block (inclusive). If not set, runs to last observation.
    #[arg(long)]
    to_block: Option<u64>,

    /// Stop-loss ratio: sell if price drops to this fraction of entry price.
    /// E.g., 0.7 = sell at -30% loss. Disabled by default.
    #[arg(long)]
    stop_loss_ratio: Option<String>,

    /// Take-profit ratio: sell if price rises to this multiple of entry price.
    /// E.g., 3.0 = sell at +200% profit. Disabled by default.
    #[arg(long)]
    take_profit_ratio: Option<String>,

    /// Max hold active pool-update blocks: force sell after this many distinct
    /// pool-update blocks while the position is open.
    /// Disabled by default.
    #[arg(long)]
    max_hold_blocks: Option<u64>,

    /// Blocks between signal observation/submission and simulated confirmation.
    /// Default 1 means observe N, submit at N, fill against post-block N+1 state.
    #[arg(long, default_value_t = 1)]
    execution_delay_blocks: u64,
}

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    // Discovery flag: print valid suite names and exit before touching config
    // files, databases, or the reth datadir, so an agent can enumerate options
    // with zero infrastructure available.
    if args.list_strategy_suites {
        print_strategy_suites();
        return Ok(());
    }

    // `replay_run_id` is clap-optional only so `--list-strategy-suites` can run
    // without it; a real backtest run requires it.
    let replay_run_id = args.replay_run_id.clone().ok_or_else(|| {
        eyre::eyre!(
            "--replay-run-id is required for a backtest run. Pass an existing live chain-sim run \
             id (or a `risk-atlas-` prefixed Risk Atlas run id). Use --list-strategy-suites to \
             enumerate strategy suites."
        )
    })?;

    // Pre-flight: validate the execution-delay semantic before any I/O.
    validate_execution_delay_blocks(args.execution_delay_blocks, args.from_block, args.to_block)?;

    let shared_config = load_shared_config()?;
    let database_url = required_shared_config_value(&shared_config, ALPHA_DATABASE_CONFIG_KEY)?;
    let risk_atlas_database_url =
        required_shared_config_value(&shared_config, RISK_ATLAS_DATABASE_CONFIG_KEY)?;
    let token_state_database_url = if args.token_state_scope.is_some() {
        Some(required_shared_config_value(
            &shared_config,
            TOKEN_STATE_DATABASE_CONFIG_KEY,
        )?)
    } else {
        None
    };
    let reth_datadir = required_shared_config_value(&shared_config, "RETH_DATADIR")?;

    // Pre-flight: fail fast with a single, named error listing exactly what is
    // missing — DB connectivity, the reth datadir, and the replay run id — before
    // the long-running pipeline starts. Happy-path behavior is unchanged: when
    // everything is present these checks are cheap and silent.
    let replay_uses_risk_atlas = replay_run_id.starts_with("risk-atlas-");
    preflight_validate(PreflightInputs {
        alpha_database_url: &database_url,
        risk_atlas_database_url: &risk_atlas_database_url,
        token_state_database_url: token_state_database_url.as_deref(),
        reth_datadir: &reth_datadir,
        replay_run_id: &replay_run_id,
        replay_uses_risk_atlas,
    })
    .await?;

    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let strategy_options = StrategySuiteOptions {
        strategy_name: args.strategy_name.clone(),
        strategy_impl: args.strategy_impl.clone(),
        strategy_suite: args.strategy_suite.clone(),
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
    };
    let strategy_specs = build_strategy_specs(&strategy_options)?;
    if !replay_uses_risk_atlas {
        validate_historical_signal_replay_names(&strategy_specs)?;
    }

    let buy_amount = Amount {
        raw: alloy_primitives::U256::from_str_radix(&args.buy_amount_wei, 10)
            .wrap_err("invalid --buy-amount-wei")?,
        decimals: 18,
    };
    let min_liquidity_eth =
        Decimal::from_str(&args.min_liquidity_eth).wrap_err("invalid --min-liquidity-eth")?;
    let min_liquidity_usd =
        Decimal::from_str(&args.min_liquidity_usd).wrap_err("invalid --min-liquidity-usd")?;

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to connect to Postgres trading store")?;
    let include_historical_signal_risk_events = strategy_specs
        .iter()
        .any(BacktestStrategySpec::uses_signal_risk_events);
    let risk_atlas_loader_allowed_protocols = common_nonempty_allowed_protocols(&strategy_specs);

    store
        .start_run(
            "backtest",
            serde_json::json!({
                "strategy_name": args.strategy_name.clone(),
                "strategy_impl": args.strategy_impl.clone(),
                "strategy_runtime": "historical",
                "strategy_input_timing": "confirmed-history",
                "strategy_suite": args.strategy_suite.clone(),
                "strategies": strategy_specs.iter().map(BacktestStrategySpec::config_json).collect::<Vec<_>>(),
                "replay_run_id": replay_run_id.clone(),
                "token_state_scope": args.token_state_scope.clone(),
                "token_state_risk_overlay": args.token_state_scope.is_some(),
                "from_block": args.from_block,
                "to_block": args.to_block,
                "skip_primed": args.skip_primed,
                "buy_amount_wei": args.buy_amount_wei.clone(),
                "min_liquidity_eth": min_liquidity_eth.to_string(),
                "min_liquidity_usd": min_liquidity_usd.to_string(),
                "stop_loss_ratio": args.stop_loss_ratio.clone(),
                "take_profit_ratio": args.take_profit_ratio.clone(),
                "max_hold_blocks": args.max_hold_blocks,
                "execution_delay_blocks": args.execution_delay_blocks,
                "historical_signal_risk_events": include_historical_signal_risk_events,
                "risk_atlas_loader_allowed_protocols": risk_atlas_loader_allowed_protocols.clone(),
            }),
        )
        .await
        .wrap_err("failed to start backtest run")?;

    let mut events = if replay_uses_risk_atlas {
        let risk_atlas_pool = sqlx::PgPool::connect(&risk_atlas_database_url)
            .await
            .wrap_err("failed to connect Risk Atlas database")?;
        load_events_from_risk_atlas(
            &risk_atlas_pool,
            &replay_run_id,
            args.from_block,
            args.to_block,
            &risk_atlas_loader_allowed_protocols,
        )
        .await
        .wrap_err_with(|| {
            format!("failed to load Risk Atlas observations for run {replay_run_id}")
        })?
    } else {
        load_events_from_observations(
            store.pool(),
            &replay_run_id,
            args.skip_primed,
            args.from_block,
            args.to_block,
            include_historical_signal_risk_events,
        )
        .await
        .wrap_err_with(|| format!("failed to load observations for run {replay_run_id}"))?
    };

    if let (Some(scope_id), Some(database_url)) = (
        args.token_state_scope.as_deref(),
        token_state_database_url.as_deref(),
    ) {
        let token_state_pool = sqlx::PgPool::connect(database_url)
            .await
            .wrap_err("failed to connect token_state database")?;
        let overlay_events = load_events_from_token_state(
            &token_state_pool,
            scope_id,
            args.from_block,
            args.to_block,
            &risk_atlas_loader_allowed_protocols,
        )
        .await
        .wrap_err_with(|| {
            format!("failed to load token_state terminal pool events for scope {scope_id}")
        })?;
        events.extend(overlay_events);
        events = sort_events_by_block(events);
    }

    if events.is_empty() {
        return Err(eyre::eyre!(
            "no valid events found for replay run {replay_run_id}"
        ));
    }

    run_chain_sim_backtest(
        ChainSimBacktestConfig {
            replay_run_id: replay_run_id.clone(),
            strategy_suite: args.strategy_suite.clone(),
            execution_delay_blocks: args.execution_delay_blocks,
        },
        store,
        events,
        run_id,
        strategy_specs,
        buy_amount,
        min_liquidity_eth,
        min_liquidity_usd,
        &reth_datadir,
    )
    .await?;

    Ok(())
}

fn default_run_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("alpha-backtest-{secs}-{}", std::process::id())
}

/// Print every valid `--strategy-suite` value for the `--list-strategy-suites`
/// discovery flag, enumerated from the dispatch match's source-of-truth list.
fn print_strategy_suites() {
    println!("Valid --strategy-suite values:");
    for name in strategy_suite_names() {
        println!("  {name}");
    }
    println!(
        "\nOmit --strategy-suite to run a single strategy via --strategy-impl / --strategy-name."
    );
}

/// Validate the `--execution-delay-blocks` value against the observe-N / fill-N+delta
/// semantic before any I/O. A delay of 0 would let a decision exploit the same
/// block it was observed in (no realizable timing), and a delay larger than the
/// replayed block span would never fill, so both are rejected up front.
fn validate_execution_delay_blocks(
    execution_delay_blocks: u64,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Result<()> {
    if execution_delay_blocks < 1 {
        return Err(eyre::eyre!(
            "--execution-delay-blocks must be >= 1 (got {execution_delay_blocks}). The backtest \
             observes a signal at block N, submits at N, and fills against post-block N+delta \
             state; a delay of 0 would let the decision exploit its own observation block."
        ));
    }
    if let (Some(from_block), Some(to_block)) = (from_block, to_block) {
        if to_block >= from_block {
            let span = to_block - from_block;
            if execution_delay_blocks > span {
                return Err(eyre::eyre!(
                    "--execution-delay-blocks ({execution_delay_blocks}) exceeds the replay block \
                     span ({span} = --to-block {to_block} - --from-block {from_block}). A signal \
                     observed at N fills against post-block N+delta state, so no fill can land \
                     inside the range. Lower --execution-delay-blocks or widen the block range."
                ));
            }
        }
    }
    Ok(())
}

/// Inputs for the fail-fast pre-flight validation pass.
struct PreflightInputs<'a> {
    alpha_database_url: &'a str,
    risk_atlas_database_url: &'a str,
    token_state_database_url: Option<&'a str>,
    reth_datadir: &'a str,
    replay_run_id: &'a str,
    replay_uses_risk_atlas: bool,
}

/// Fail fast with a single, named error before the long-running pipeline starts.
///
/// Validates, in order: (1) the reth datadir is set and is a readable directory;
/// (2) each required Postgres connection actually succeeds; (3) the
/// `--replay-run-id` has at least one observation row in the target DB
/// (`risk_atlas_observations` when the id is `risk-atlas-` prefixed, else
/// `alpha_trading.strategy_observations`). Each failure names exactly what is
/// missing instead of surfacing an opaque error deep in the pipeline.
async fn preflight_validate(inputs: PreflightInputs<'_>) -> Result<()> {
    // (a) reth datadir must be a readable directory.
    let datadir = PathBuf::from(inputs.reth_datadir);
    let metadata = fs::metadata(&datadir).wrap_err_with(|| {
        format!(
            "RETH_DATADIR points at {} which cannot be read (set RETH_DATADIR in {} to the synced \
             reth datadir)",
            datadir.display(),
            shared_config_path().display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(eyre::eyre!(
            "RETH_DATADIR ({}) is not a directory; it must be the reth datadir the chain-server / \
             tx-simulator reads sim state from",
            datadir.display()
        ));
    }

    // (b) Alpha DB connection (always required — results are written here).
    let alpha_pool = connect_preflight(inputs.alpha_database_url, ALPHA_DATABASE_CONFIG_KEY).await?;

    // The Risk Atlas / token_state pools are only opened when actually needed,
    // so probe them only in those modes to keep the happy path's I/O identical.
    let risk_atlas_pool = if inputs.replay_uses_risk_atlas {
        Some(
            connect_preflight(
                inputs.risk_atlas_database_url,
                RISK_ATLAS_DATABASE_CONFIG_KEY,
            )
            .await?,
        )
    } else {
        None
    };

    if let Some(token_state_url) = inputs.token_state_database_url {
        let _token_state_pool =
            connect_preflight(token_state_url, TOKEN_STATE_DATABASE_CONFIG_KEY).await?;
    }

    // (c) The replay run id must actually exist in the target observation table.
    if inputs.replay_uses_risk_atlas {
        let pool = risk_atlas_pool.as_ref().expect("risk atlas pool probed above");
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM risk_atlas_observations WHERE run_id = $1)")
                .bind(inputs.replay_run_id)
                .fetch_one(pool)
                .await
                .wrap_err("failed to check --replay-run-id in risk_atlas_observations")?;
        if !exists {
            return Err(eyre::eyre!(
                "--replay-run-id '{}' has no rows in risk_atlas_observations (Risk Atlas database \
                 {}). Confirm the run id, or drop the `risk-atlas-` prefix to replay from \
                 strategy_observations instead.",
                inputs.replay_run_id,
                RISK_ATLAS_DATABASE_CONFIG_KEY
            ));
        }
    } else {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM alpha_trading.strategy_observations WHERE run_id = $1)",
        )
        .bind(inputs.replay_run_id)
        .fetch_one(&alpha_pool)
        .await
        .wrap_err("failed to check --replay-run-id in alpha_trading.strategy_observations")?;
        if !exists {
            return Err(eyre::eyre!(
                "--replay-run-id '{}' has no rows in alpha_trading.strategy_observations (Alpha \
                 database {}). Confirm the run id, or use a `risk-atlas-` prefixed id to replay \
                 from risk_atlas_observations instead.",
                inputs.replay_run_id,
                ALPHA_DATABASE_CONFIG_KEY
            ));
        }
    }

    alpha_pool.close().await;
    if let Some(pool) = risk_atlas_pool {
        pool.close().await;
    }
    Ok(())
}

/// Open a probe connection for pre-flight, naming the config key on failure so a
/// fresh agent knows which `databases.*.url` to fix.
async fn connect_preflight(url: &str, config_key: &str) -> Result<sqlx::PgPool> {
    sqlx::PgPool::connect(url).await.wrap_err_with(|| {
        format!(
            "failed to connect to the database for {config_key} (set it in {})",
            shared_toml_config_path().display()
        )
    })
}

fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read shared config file {}", path.display()))?;
    let mut values = parse_shared_config(&contents);
    merge_toml_database_config(&mut values)?;
    Ok(values)
}

fn required_shared_config_value(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eyre::eyre!(
                "{key} must be set in shared config file {}",
                if key.starts_with("databases.") {
                    shared_toml_config_path()
                } else {
                    shared_config_path()
                }
                .display()
            )
        })
}

fn shared_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.env")
}

fn shared_toml_config_path() -> PathBuf {
    shared_config_path().with_file_name("config.toml")
}

fn merge_toml_database_config(values: &mut HashMap<String, String>) -> Result<()> {
    let path = shared_toml_config_path();
    let contents = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read shared TOML config file {}", path.display()))?;
    let root = contents
        .parse::<toml::Value>()
        .wrap_err_with(|| format!("failed to parse shared TOML config file {}", path.display()))?;
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get("alpha"))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(ALPHA_DATABASE_CONFIG_KEY.to_string(), url.to_string());
    }
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get("risk_atlas"))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(RISK_ATLAS_DATABASE_CONFIG_KEY.to_string(), url.to_string());
    }
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get("token_state"))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(TOKEN_STATE_DATABASE_CONFIG_KEY.to_string(), url.to_string());
    }
    Ok(())
}

fn parse_shared_config(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(
            key.to_string(),
            unquote_config_value(value.trim()).to_string(),
        );
    }
    values
}

fn unquote_config_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::validate_execution_delay_blocks;

    #[test]
    fn execution_delay_zero_is_rejected() {
        let err = validate_execution_delay_blocks(0, None, None).expect_err("0 must be rejected");
        assert!(format!("{err}").contains(">= 1"), "{err}");
    }

    #[test]
    fn execution_delay_one_is_accepted_without_bounds() {
        assert!(validate_execution_delay_blocks(1, None, None).is_ok());
    }

    #[test]
    fn execution_delay_within_span_is_accepted() {
        assert!(validate_execution_delay_blocks(3, Some(100), Some(110)).is_ok());
    }

    #[test]
    fn execution_delay_exceeding_span_is_rejected() {
        let err = validate_execution_delay_blocks(11, Some(100), Some(110))
            .expect_err("delay > span must be rejected");
        assert!(format!("{err}").contains("exceeds the replay block span"), "{err}");
    }

    #[test]
    fn execution_delay_equal_to_span_is_accepted() {
        assert!(validate_execution_delay_blocks(10, Some(100), Some(110)).is_ok());
    }
}
