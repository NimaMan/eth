use serde::Serialize;

use crate::runs::{RunError, TrackingRun};

#[derive(Clone, Debug, Serialize)]
pub struct ErrorListResponse {
    pub run_id: String,
    pub count: usize,
    pub errors: Vec<RunError>,
}

pub async fn error_list(run: &TrackingRun) -> ErrorListResponse {
    let state = run.state.read().await;
    ErrorListResponse {
        run_id: run.id.clone(),
        count: state.errors.len(),
        errors: state.errors.clone(),
    }
}
