use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use eth_alpha_lab::{
    connect,
    position_lab::{self, PositionSelector},
    strategy_lab,
};
use eyre::{eyre, Result};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let shared_config = load_shared_config()?;
    let database_url = required_shared_config_value(&shared_config, "ALPHA_DATABASE_URL")?;
    let pool = connect(&database_url).await?;

    match args.command {
        Command::Strategy {
            run_ids,
            limit,
            json,
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
    }

    Ok(())
}

fn shared_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.env")
}

fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path).map_err(|error| {
        eyre!(
            "failed to read shared config file {}: {error}",
            path.display()
        )
    })?;
    Ok(parse_shared_config(&contents))
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
                shared_config_path().display()
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
