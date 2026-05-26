use serde::Serialize;

use crate::ranges::{RangeIndexError, RangeIndexErrorKind, RangeIndexJob};

#[derive(Clone, Debug, Serialize)]
pub struct ErrorListResponse {
    pub run_id: String,
    pub count: usize,
    pub transaction_failure_count: usize,
    pub pool_simulation_failure_count: usize,
    pub run_failure_count: usize,
    pub errors: Vec<RangeIndexError>,
}

pub async fn error_list(run: &RangeIndexJob) -> ErrorListResponse {
    let state = run.state.read().await;
    ErrorListResponse {
        run_id: run.id.clone(),
        count: state.errors.len(),
        transaction_failure_count: state
            .errors
            .iter()
            .filter(|error| error.kind == RangeIndexErrorKind::Transaction)
            .count(),
        pool_simulation_failure_count: state
            .errors
            .iter()
            .filter(|error| error.kind == RangeIndexErrorKind::PoolSimulation)
            .count(),
        run_failure_count: state
            .errors
            .iter()
            .filter(|error| error.kind == RangeIndexErrorKind::Run)
            .count(),
        errors: state.errors.clone(),
    }
}
