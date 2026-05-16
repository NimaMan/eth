use serde::Serialize;

use crate::ranges::RangeIndexJob;
use crate::read_models::surface::PoolSurfaceFilter;

use super::view::{sort_pools_by_liquidity, PoolView};

#[derive(Clone, Debug, Serialize)]
pub struct PoolListResponse {
    pub run_id: String,
    pub count: usize,
    pub pools: Vec<PoolView>,
}

pub async fn pool_list(run: &RangeIndexJob) -> PoolListResponse {
    pool_list_filtered(run, PoolSurfaceFilter::All).await
}

pub async fn pool_list_filtered(
    run: &RangeIndexJob,
    filter: PoolSurfaceFilter,
) -> PoolListResponse {
    let state = run.state.read().await;
    let mut pools = Vec::new();

    for token in state.processor.registry.tokens.values() {
        pools.extend(PoolView::from_token_pool_summaries(token));
    }
    pools.retain(|pool| filter.matches(pool));

    sort_pools_by_liquidity(&mut pools);

    PoolListResponse {
        run_id: run.id.clone(),
        count: pools.len(),
        pools,
    }
}
