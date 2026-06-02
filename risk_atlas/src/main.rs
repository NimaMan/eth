use eth_risk_atlas::config::RiskAtlasConfig;
use eth_risk_atlas::db::migrate::{apply, RISK_ATLAS_SCHEMA_SQL};
use eth_risk_atlas::db::RiskAtlasWriter;
use eth_risk_atlas::ingest::report::{
    import_distribution_report, DEFAULT_100K_DISTRIBUTION_REPORT,
};
use eth_risk_atlas::scammer_analytics::ScammerCaseAnalyzer;
use eyre::Result;
use sqlx::PgPool;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "schema" => {
            print!("{RISK_ATLAS_SCHEMA_SQL}");
        }
        "migrate" => {
            let config = RiskAtlasConfig::from_shared_config()?;
            let pool = PgPool::connect(&config.database_url).await?;
            apply(&pool).await?;
            println!("risk atlas migrations applied");
        }
        "import-report" | "import-100k" => {
            let report = std::env::args()
                .nth(2)
                .unwrap_or_else(|| DEFAULT_100K_DISTRIBUTION_REPORT.to_string());
            let config = RiskAtlasConfig::from_shared_config()?;
            let pool = PgPool::connect(&config.database_url).await?;
            apply(&pool).await?;
            let import = import_distribution_report(&report, None)?;
            let writer = RiskAtlasWriter::new(pool);
            writer.replace_report_import(&import).await?;
            println!(
                "risk atlas imported run={} distributions={} stats={} targets={}",
                import.run.run_id,
                import.distributions.len(),
                import.numeric_stats.len(),
                import.active_targets.len()
            );
        }
        "scammer-case" => {
            let case_file = std::env::args().nth(2).ok_or_else(|| {
                eyre::eyre!(
                    "usage: cargo run -p eth_risk_atlas -- scammer-case <case.toml> [reth_datadir]"
                )
            })?;
            let reth_datadir = std::env::args().nth(3).unwrap_or_else(default_reth_datadir);
            let report = ScammerCaseAnalyzer::analyze_case_file(&case_file, &reth_datadir).await?;
            let case_dir = Path::new(&case_file)
                .parent()
                .unwrap_or_else(|| Path::new("."));
            report.write_artifacts(case_dir)?;
            println!(
                "scammer analytics wrote case={} txs={} forwarder_traces={} total_forwarded_eth={:.9}",
                report.config.case_id,
                report.summary.transaction_count,
                report.summary.forwarder_trace_count,
                report.summary.total_forwarded_eth
            );
        }
        _ => {
            eprintln!(
                "usage: cargo run -p eth_risk_atlas -- <schema|migrate|import-report [report.md]|scammer-case <case.toml> [reth_datadir]>"
            );
        }
    }

    Ok(())
}

fn default_reth_datadir() -> String {
    std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/storage/samsung8tb/ethereum/reth".to_string())
}
