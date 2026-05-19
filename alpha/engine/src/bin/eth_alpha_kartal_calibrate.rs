use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use clap::{Parser, ValueEnum};
use eth_live_trading::{
    run_calibration, CalibrationInputFile, CalibrationOverallVerdict, CalibrationRunConfig,
    ExpectedCalibrationOutcome,
};
use eyre::{eyre, Result, WrapErr};

const DEFAULT_REPORT_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/alpha/lab/reports/kartal_calibration";

#[derive(Debug, Parser)]
struct Args {
    /// JSON file containing either a single eth_direct_raw_v1 request or a
    /// calibration suite with `cases`.
    #[arg(long)]
    request: PathBuf,

    #[arg(long, default_value = "http://127.0.0.1:5004")]
    kartal_url: String,

    /// Env var used for Kartal auth. If empty, KARTAL_API_TOKEN is tried.
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let input = load_input(&args.request)?;
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
