use eyre::Result;
use sqlx::PgPool;

use crate::load_run_metadata;

mod checks;
mod model;
mod output;
mod query;

pub use model::{
    ExecutionReportRecord, PoolObservation, PositionCheck, PositionRecord, PositionReport,
    PositionSelector, SnapshotRecord, TrajectoryPoint,
};
pub use output::print_position_report;

pub async fn analyze_position(
    pool: &PgPool,
    run_id: &str,
    selector: PositionSelector,
    replay_override: Option<&str>,
    samples: i64,
) -> Result<PositionReport> {
    let mut run = load_run_metadata(pool, run_id).await?;
    if let Some(replay_run_id) = replay_override {
        run.replay_run_id = Some(replay_run_id.to_string());
    }

    let position = query::load_position(pool, run_id, selector).await?;
    let entry_report = query::load_entry_report(pool, &position).await?;
    let latest_snapshot = query::load_latest_snapshot(pool, &position).await?;

    let replay_run_id = run.replay_run_id.as_deref();
    let entry_observation = match (replay_run_id, position.entry_block) {
        (Some(replay), Some(block)) => {
            query::load_pool_observation(pool, replay, block, &position.pool_address).await?
        }
        _ => None,
    };
    let latest_observation = match (
        replay_run_id,
        latest_snapshot.as_ref().and_then(|s| s.block_number),
    ) {
        (Some(replay), Some(block)) => {
            query::load_pool_observation(pool, replay, block, &position.pool_address).await?
        }
        _ => None,
    };
    let trajectory =
        query::load_trajectory(pool, &position, replay_run_id.unwrap_or(""), samples.max(1))
            .await?;
    let checks = checks::build_checks(
        &run,
        &position,
        entry_report.as_ref(),
        latest_snapshot.as_ref(),
        entry_observation.as_ref(),
        latest_observation.as_ref(),
    );

    Ok(PositionReport {
        run,
        position,
        entry_report,
        latest_snapshot,
        entry_observation,
        latest_observation,
        trajectory,
        checks,
    })
}
