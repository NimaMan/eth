use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eth_alpha_lab::{
    connect,
    position_lab::{self, PositionSelector},
    strategy_assessment::{
        self, report::print_strategy_assessment_report, StrategyAssessmentOptions,
    },
    strategy_event_trace::{self, LossScanOptions, TraceSelector},
    strategy_lab,
    strategy_validation::{
        self, persistence::persist_strategy_validation_report,
        report::print_strategy_validation_report, ValidationOptions,
    },
};
use eyre::{eyre, Result};

const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";

#[derive(Debug, Parser)]
#[command(name = "eth_alpha_lab")]
#[command(about = "Offline diagnostics for ETH alpha strategy and position sanity checks")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run-level diagnostics: PnL, concentration, failures, and invariant flags.
    Strategy {
        #[arg(long = "run-id", required = true)]
        run_ids: Vec<String>,

        #[arg(long, default_value_t = 10)]
        limit: i64,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        persist: bool,
    },

    /// Single position/token diagnostics with entry, snapshots, and observation joins.
    Position {
        #[arg(long)]
        run_id: String,

        #[arg(long, conflicts_with = "position_id")]
        token: Option<String>,

        #[arg(long = "position-id", conflicts_with = "token")]
        position_id: Option<String>,

        /// Override replay run if run metadata does not record it.
        #[arg(long = "replay-run-id")]
        replay_run_id: Option<String>,

        /// Number of first and last trajectory rows to print.
        #[arg(long, default_value_t = 8)]
        samples: i64,

        #[arg(long)]
        json: bool,
    },

    /// Strategy validation for lifecycle, timing, accounting, and API trust.
    #[command(name = "strategy-validation", alias = "backtest-validation")]
    StrategyValidation {
        #[arg(long = "result-set")]
        result_set_id: String,

        #[arg(long)]
        strategy: Option<String>,

        /// Deprecated; validation is always comprehensive.
        #[arg(long, hide = true)]
        profile: Option<String>,

        /// Deprecated; comprehensive validation owns its sample policy.
        #[arg(long = "sample-limit", hide = true)]
        sample_limit: Option<i64>,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        persist: bool,

        /// Promotion gate: exit non-zero when any promotion-blocking check
        /// fails. ON by default so CI/operators gate on the exit code. Pass
        /// --no-gate to always exit 0 (still prints/persists the verdict) for
        /// non-gating diagnostic runs. A `Blocked` ("could not evaluate")
        /// verdict never counts as a blocking failure.
        #[arg(long = "no-gate", action = clap::ArgAction::SetFalse, default_value_t = true)]
        gate: bool,
    },

    /// Strategy-quality assessment questions such as PnL concentration and exposure.
    StrategyAssessment {
        #[arg(long = "result-set")]
        result_set_id: String,

        #[arg(long)]
        strategy: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// One trade's ordered event timeline: trade events, risks, decisions, snapshots.
    TradeEvents {
        #[arg(long = "result-set")]
        result_set_id: String,

        #[arg(long)]
        strategy: String,

        #[arg(long = "trade-id")]
        trade_id: String,

        #[arg(long = "run-id")]
        run_id: Option<String>,

        #[arg(long)]
        json: bool,
    },

    /// Scan losing trades for repeated event-timing signal candidates.
    LosingTrades {
        #[arg(long = "result-set")]
        result_set_id: String,

        #[arg(long)]
        strategy: String,

        #[arg(long = "run-id")]
        run_id: Option<String>,

        #[arg(long, default_value_t = 20)]
        limit: usize,

        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let shared_config = load_shared_config()?;
    let database_url = required_shared_config_value(&shared_config, ALPHA_DATABASE_CONFIG_KEY)?;
    let pool = connect(&database_url).await?;

    match args.command {
        Command::Strategy {
            run_ids,
            limit,
            json,
            persist: _,
        } => {
            let mut reports = Vec::with_capacity(run_ids.len());
            for run_id in &run_ids {
                reports.push(strategy_lab::analyze_strategy(&pool, run_id, limit).await?);
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&reports)?);
            } else {
                for (idx, report) in reports.iter().enumerate() {
                    if idx > 0 {
                        println!();
                        println!("---");
                        println!();
                    }
                    strategy_lab::print_strategy_report(report);
                }
            }
        }
        Command::Position {
            run_id,
            token,
            position_id,
            replay_run_id,
            samples,
            json,
        } => {
            let selector = match (position_id, token) {
                (Some(position_id), None) => PositionSelector::PositionId(position_id),
                (None, Some(token)) => PositionSelector::Token(token),
                _ => {
                    return Err(eyre!(
                        "provide exactly one selector: --token or --position-id"
                    ))
                }
            };
            let report = position_lab::analyze_position(
                &pool,
                &run_id,
                selector,
                replay_run_id.as_deref(),
                samples,
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                position_lab::print_position_report(&report);
            }
        }
        Command::StrategyValidation {
            result_set_id,
            strategy,
            profile: _deprecated_profile,
            sample_limit: _deprecated_sample_limit,
            json,
            persist,
            gate,
        } => {
            let report = strategy_validation::validate_strategy(
                &pool,
                ValidationOptions {
                    result_set_id,
                    strategy,
                },
            )
            .await?;
            let validation_id = if persist {
                Some(persist_strategy_validation_report(&pool, &report).await?)
            } else {
                None
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_strategy_validation_report(&report);
                if let Some(validation_id) = validation_id {
                    println!();
                    println!("Persisted validation report: {validation_id}");
                }
            }
            // Gate LAST so printing/persisting always happens first. Exit
            // non-zero only on a promotion-blocking failure when gating is on
            // (the default); --no-gate keeps diagnostic runs at exit 0.
            if gate && report.summary.has_blocking_failures() {
                if !json {
                    eprintln!(
                        "strategy-validation: {} promotion-blocking check(s) failed; refusing promotion (exit 1). Pass --no-gate for a non-gating run.",
                        report.summary.blocking_failures
                    );
                }
                std::process::exit(1);
            }
        }
        Command::StrategyAssessment {
            result_set_id,
            strategy,
            json,
        } => {
            let report = strategy_assessment::assess_strategy(
                &pool,
                StrategyAssessmentOptions {
                    result_set_id,
                    strategy,
                },
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_strategy_assessment_report(&report);
            }
        }
        Command::TradeEvents {
            result_set_id,
            strategy,
            trade_id,
            run_id,
            json,
        } => {
            let trace = strategy_event_trace::trace_trade(
                &pool,
                TraceSelector {
                    result_set_id,
                    strategy_name: strategy,
                    trade_id,
                    run_id,
                },
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&trace)?);
            } else {
                strategy_event_trace::print_trade_trace(&trace);
            }
        }
        Command::LosingTrades {
            result_set_id,
            strategy,
            run_id,
            limit,
            json,
        } => {
            let report = strategy_event_trace::scan_losing_trades(
                &pool,
                LossScanOptions {
                    result_set_id,
                    strategy_name: strategy,
                    run_id,
                    detail_limit: limit,
                },
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                strategy_event_trace::print_loss_scan(&report);
            }
        }
    }

    Ok(())
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

fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path).map_err(|error| {
        eyre!(
            "failed to read shared config file {}: {error}",
            path.display()
        )
    })?;
    let mut values = parse_shared_config(&contents);
    merge_toml_database_config(&mut values)?;
    Ok(values)
}

fn merge_toml_database_config(values: &mut HashMap<String, String>) -> Result<()> {
    let path = shared_toml_config_path();
    let contents = fs::read_to_string(&path).map_err(|error| {
        eyre!(
            "failed to read shared TOML config file {}: {error}",
            path.display()
        )
    })?;
    let root = contents.parse::<toml::Value>().map_err(|error| {
        eyre!(
            "failed to parse shared TOML config file {}: {error}",
            path.display()
        )
    })?;
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

fn required_shared_config_value(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eyre!(
                "{key} must be set in shared config file {}",
                if key == ALPHA_DATABASE_CONFIG_KEY {
                    shared_toml_config_path()
                } else {
                    shared_config_path()
                }
                .display()
            )
        })
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
