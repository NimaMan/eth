use eth_risk_atlas::config::RiskAtlasConfig;
use eth_risk_atlas::db::migrate::{apply, RISK_ATLAS_SCHEMA_SQL};
use eth_risk_atlas::db::RiskAtlasWriter;
use eth_risk_atlas::ingest::report::{
    import_distribution_report, DEFAULT_100K_DISTRIBUTION_REPORT,
};
use eyre::Result;
use sqlx::PgPool;

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
            let config = RiskAtlasConfig::default();
            let pool = PgPool::connect(&config.database_url).await?;
            apply(&pool).await?;
            println!("risk atlas migrations applied");
        }
        "import-report" | "import-100k" => {
            let report = std::env::args()
                .nth(2)
                .unwrap_or_else(|| DEFAULT_100K_DISTRIBUTION_REPORT.to_string());
            let config = RiskAtlasConfig::default();
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
        _ => {
            eprintln!(
                "usage: cargo run -p eth_risk_atlas -- <schema|migrate|import-report [report.md]>"
            );
        }
    }

    Ok(())
}
