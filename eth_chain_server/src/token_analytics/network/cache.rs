use std::collections::BTreeMap;
use std::sync::Arc;

use super::job::{TokenNetworkAnalysisJob, TokenNetworkAnalysisStatus};

pub const DEFAULT_MAX_ANALYSIS_JOBS: usize = 32;

pub async fn prune_finished_jobs(
    jobs: &mut BTreeMap<String, Arc<TokenNetworkAnalysisJob>>,
    max_jobs: usize,
) {
    if jobs.len() <= max_jobs {
        return;
    }

    let job_refs = jobs
        .iter()
        .map(|(id, job)| (id.clone(), job.clone()))
        .collect::<Vec<_>>();
    let mut finished = Vec::new();
    for (id, job) in job_refs {
        let state = job.state().await;
        if matches!(
            state.progress.status,
            TokenNetworkAnalysisStatus::Complete
                | TokenNetworkAnalysisStatus::Failed
                | TokenNetworkAnalysisStatus::Canceled
        ) {
            finished.push((
                state
                    .progress
                    .finished_at
                    .unwrap_or(state.progress.updated_at),
                id,
            ));
        }
    }

    finished.sort_by_key(|(finished_at, _)| *finished_at);
    for (_, id) in finished {
        if jobs.len() <= max_jobs {
            break;
        }
        jobs.remove(&id);
    }
}
