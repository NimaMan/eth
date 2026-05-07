use serde::Serialize;

use crate::range_indexer::{RangeIndexError, RangeIndexJob};

#[derive(Clone, Debug, Serialize)]
pub struct ErrorListResponse {
    pub run_id: String,
    pub count: usize,
    pub errors: Vec<RangeIndexError>,
}

pub async fn error_list(run: &RangeIndexJob) -> ErrorListResponse {
    let state = run.state.read().await;
    ErrorListResponse {
        run_id: run.id.clone(),
        count: state.errors.len(),
        errors: state.errors.clone(),
    }
}
