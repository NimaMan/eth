use std::fs;
use std::path::PathBuf;

use alloy_primitives::Address;
use chrono::Utc;
use clap::{Parser, ValueEnum};
use eth_live_trading::{
    build_planner_calibration_request, run_calibration, CalibrationInputFile,
    CalibrationOverallVerdict, CalibrationRunConfig, ExpectedCalibrationOutcome,
    PlannerCalibrationFixtureConfig, PlannerCalibrationRoute,
};
use eyre::{eyre, Result, WrapErr};
use serde_json::json;

const DEFAULT_REPORT_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/alpha/lab/reports/kartal_calibration";
const DEFAULT_PLANNER_FIXTURE_STRATEGY: &str =
    "snipe-all-risk-atlas-lp-gate-hold15-buy-confirm-lp-maxhold";
const DEFAULT_PLANNER_FIXTURE_RUN_ID: &str = "kartal-calibration-planner-fixture";
const DEFAULT_PLANNER_FIXTURE_VAULT: &str = "0x0000000000000000000000000000000000000002";

#[derive(Debug, Parser)]
struct Args {
    /// JSON file containing either a single eth_unsigned_tx request or a
    /// calibration suite with `cases`.
    #[arg(long)]
    request: Option<PathBuf>,

    /// Build the calibration request through the live priority-sell planner
    /// instead of reading a JSON request from disk.
    #[arg(long, default_value_t = false)]
    planner_fixture: bool,

    /// Signer/from address for --planner-fixture. Defaults to the single
    /// ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES address when available.
    #[arg(long)]
    planner_fixture_from: Option<String>,

    #[arg(long, value_enum, default_value_t = PlannerFixtureRouteArg::TradingVaultUniswapV2)]
    planner_fixture_route: PlannerFixtureRouteArg,

    #[arg(long, default_value = DEFAULT_PLANNER_FIXTURE_VAULT)]
    planner_fixture_vault_address: String,

    #[arg(long, default_value = DEFAULT_PLANNER_FIXTURE_STRATEGY)]
    planner_fixture_strategy_name: String,

    #[arg(long, default_value = DEFAULT_PLANNER_FIXTURE_RUN_ID)]
    planner_fixture_run_id: String,

    /// Optional suffix for the generated planner-fixture attempt id.
    /// Defaults to a UTC timestamp so repeated dry-runs remain auditable.
    #[arg(long)]
    planner_fixture_attempt_suffix: Option<String>,

    /// Keep the planner-produced attempt id deterministic. Prefer leaving this
    /// off for repeated dry-run signing checks.
    #[arg(long, default_value_t = false)]
    stable_planner_fixture_attempt_id: bool,

    /// Write the planner-produced request JSON before submission.
    #[arg(long)]
    write_request_path: Option<PathBuf>,

    /// Generate and write the planner-produced request, then exit without
    /// contacting Kartal. Requires --planner-fixture.
    #[arg(long, default_value_t = false)]
    write_request_only: bool,

    #[arg(long, default_value = "http://127.0.0.1:5006")]
    kartal_url: String,

    /// Env var used for ETH tx executor auth. If empty, KARTAL_API_TOKEN is tried.
    #[arg(long, default_value = "ETH_TX_EXECUTOR_API_TOKEN")]
    token_env: String,

    /// Default expected outcome for single-request files.
    #[arg(long, value_enum, default_value_t = ExpectArg::PolicyRejected)]
    expect: ExpectArg,

    /// Allow submitting calibration requests when Kartal is not in dry_run.
    /// Leave this off for repeatable safety checks.
    #[arg(long, default_value_t = false)]
    allow_non_dry_run: bool,

    #[arg(long, default_value_t = false)]
    skip_policy_journal: bool,

    /// Replace each request simulation.block_number with Kartal's current RPC
    /// eth_blockNumber before submission.
    #[arg(long, default_value_t = false)]
    refresh_simulation_block: bool,

    #[arg(long)]
    report_path: Option<PathBuf>,

    #[arg(long, default_value = DEFAULT_REPORT_DIR)]
    report_dir: PathBuf,

    #[arg(long, default_value_t = false)]
    print_json: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ExpectArg {
    AnyDecision,
    PolicyRejected,
    DryRunSigned,
}

impl From<ExpectArg> for ExpectedCalibrationOutcome {
    fn from(value: ExpectArg) -> Self {
        match value {
            ExpectArg::AnyDecision => Self::AnyDecision,
            ExpectArg::PolicyRejected => Self::PolicyRejected,
            ExpectArg::DryRunSigned => Self::DryRunSigned,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum PlannerFixtureRouteArg {
    DirectUniswapV2,
    TradingVaultUniswapV2,
}

impl From<PlannerFixtureRouteArg> for PlannerCalibrationRoute {
    fn from(value: PlannerFixtureRouteArg) -> Self {
        match value {
            PlannerFixtureRouteArg::DirectUniswapV2 => Self::DirectUniswapV2,
            PlannerFixtureRouteArg::TradingVaultUniswapV2 => Self::TradingVaultUniswapV2,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let input = load_or_build_input(&args).await?;
    if args.write_request_only {
        return Ok(());
    }
    let token = load_token(&args.token_env)?;
    let config = CalibrationRunConfig {
        kartal_base_url: args.kartal_url.clone(),
        bearer_token: token,
        require_dry_run: !args.allow_non_dry_run,
        fetch_policy_journal: !args.skip_policy_journal,
        refresh_simulation_block: args.refresh_simulation_block,
    };

    let report = run_calibration(config, input, args.expect.into())
        .await
        .wrap_err("Kartal calibration failed before report generation")?;
    let report_path = write_report(&args, &report)?;

    if args.print_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Kartal calibration {:?}: {}/{} passed, {} failed, {} unsafe",
            report.summary.verdict,
            report.summary.passed,
            report.summary.total,
            report.summary.failed,
            report.summary.unsafe_cases
        );
        println!("report: {}", report_path.display());
        for case in &report.cases {
            println!(
                "- {} [{}]: {:?} - {}",
                case.name, case.attempt_id, case.verdict.kind, case.verdict.reason
            );
        }
    }

    match report.summary.verdict {
        CalibrationOverallVerdict::Passed => Ok(()),
        CalibrationOverallVerdict::Failed => Err(eyre!("Kartal calibration failed")),
        CalibrationOverallVerdict::Unsafe => Err(eyre!("Kartal calibration was unsafe to submit")),
    }
}

async fn load_or_build_input(args: &Args) -> Result<CalibrationInputFile> {
    match (&args.request, args.planner_fixture) {
        (Some(_), true) => Err(eyre!(
            "--request and --planner-fixture are mutually exclusive input sources"
        )),
        (Some(path), false) => load_input(path),
        (None, true) => build_planner_fixture_input(args).await,
        (None, false) => Err(eyre!("provide --request or --planner-fixture")),
    }
}

async fn build_planner_fixture_input(args: &Args) -> Result<CalibrationInputFile> {
    let from = planner_fixture_from(args)?;
    let vault_address = parse_address(
        &args.planner_fixture_vault_address,
        "--planner-fixture-vault-address",
    )?;
    let mut config = PlannerCalibrationFixtureConfig::new(from);
    config.route = args.planner_fixture_route.into();
    config.vault_address = vault_address;
    config.strategy_name = args.planner_fixture_strategy_name.clone();
    config.strategy_run_id = Some(args.planner_fixture_run_id.clone());

    let mut request = build_planner_calibration_request(config)
        .await
        .wrap_err("failed to build planner-produced calibration request")?;
    if let Some(suffix) = planner_fixture_attempt_suffix(args) {
        apply_attempt_suffix(&mut request, &suffix);
    }
    if let Some(path) = &args.write_request_path {
        write_request(path, &request)?;
        println!("request: {}", path.display());
    }
    Ok(CalibrationInputFile::Single(request))
}

fn planner_fixture_from(args: &Args) -> Result<Address> {
    if let Some(value) = &args.planner_fixture_from {
        return parse_address(value, "--planner-fixture-from");
    }

    let value = std::env::var("ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES")
        .wrap_err("missing --planner-fixture-from and ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES")?;
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    match values.as_slice() {
        [single] => parse_address(single, "ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES"),
        [] => Err(eyre!(
            "ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES is empty; pass --planner-fixture-from"
        )),
        _ => Err(eyre!(
            "ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES has multiple values; pass --planner-fixture-from"
        )),
    }
}

fn planner_fixture_attempt_suffix(args: &Args) -> Option<String> {
    if let Some(suffix) = &args.planner_fixture_attempt_suffix {
        let suffix = suffix.trim();
        return (!suffix.is_empty()).then(|| suffix.to_string());
    }
    if args.stable_planner_fixture_attempt_id {
        return None;
    }
    Some(Utc::now().format("%Y%m%d%H%M%S").to_string())
}

fn apply_attempt_suffix(
    request: &mut eth_live_trading::LiveDirectRawTransactionRequest,
    suffix: &str,
) {
    let base = request
        .attempt_id
        .clone()
        .unwrap_or_else(|| "planner-fixture".to_string());
    request.attempt_id = Some(format!("{base}-{suffix}"));
    if let serde_json::Value::Object(metadata) = &mut request.metadata {
        metadata.insert("calibration_attempt_suffix".to_string(), json!(suffix));
    }
}

fn parse_address(value: &str, label: &str) -> Result<Address> {
    value
        .parse::<Address>()
        .wrap_err_with(|| format!("invalid {label} address {value:?}"))
}

fn load_input(path: &PathBuf) -> Result<CalibrationInputFile> {
    let bytes = fs::read(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .wrap_err_with(|| format!("failed to decode calibration JSON {}", path.display()))
}

fn load_token(primary_env: &str) -> Result<String> {
    for key in [primary_env, "KARTAL_API_TOKEN"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }
    }
    Err(eyre!(
        "missing Kartal bearer token; set {} or KARTAL_API_TOKEN",
        primary_env
    ))
}

fn write_request(
    path: &PathBuf,
    request: &eth_live_trading::LiveDirectRawTransactionRequest,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create request dir {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(request)?;
    fs::write(path, bytes).wrap_err_with(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_report(args: &Args, report: &eth_live_trading::CalibrationReport) -> Result<PathBuf> {
    let path = match &args.report_path {
        Some(path) => path.clone(),
        None => {
            fs::create_dir_all(&args.report_dir).wrap_err_with(|| {
                format!("failed to create report dir {}", args.report_dir.display())
            })?;
            let timestamp = Utc::now().format("%Y%m%d-%H%M%SZ");
            args.report_dir
                .join(format!("kartal-calibration-{timestamp}.json"))
        }
    };
    let bytes = serde_json::to_vec_pretty(report)?;
    fs::write(&path, bytes).wrap_err_with(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}
