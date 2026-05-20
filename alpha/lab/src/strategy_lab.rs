use eyre::Result;
use sqlx::PgPool;

use crate::load_run_metadata;

mod model;
mod output;
mod query;

pub use model::{
    BuyFailedEntry, FailureBucket, IssueFlag, OpenFailedExit, PnlConcentration, PositionRank,
    ProtocolBucket, RunSummary, StrategyReport,
};
pub use output::print_strategy_report;

pub async fn analyze_strategy(pool: &PgPool, run_id: &str, limit: i64) -> Result<StrategyReport> {
    let run = load_run_metadata(pool, run_id).await?;
    let summary = query::load_summary(pool, run_id).await?;
    let concentration = query::load_concentration(pool, run_id).await?;
    let failures = query::load_failures(pool, run_id).await?;
    let buy_failed_entries =
        query::load_buy_failed_entries(pool, run_id, run.replay_run_id.as_deref(), limit).await?;
    let open_failed_exits = query::load_open_failed_exits(pool, run_id, limit).await?;
    let protocols = query::load_protocols(pool, run_id, run.replay_run_id.as_deref()).await?;
    let top_winners = query::load_ranked_positions(pool, run_id, limit, "DESC").await?;
    let worst_losers = query::load_ranked_positions(pool, run_id, limit, "ASC").await?;
    let issue_flags = query::issue_flags(&summary, &concentration, &run);

    Ok(StrategyReport {
        run,
        summary,
        concentration,
        issue_flags,
        failures,
        buy_failed_entries,
        open_failed_exits,
        protocols,
        top_winners,
        worst_losers,
    })
}
