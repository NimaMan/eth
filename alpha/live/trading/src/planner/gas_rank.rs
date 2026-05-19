use async_trait::async_trait;
use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};

use crate::{PreparedSellRoute, RankedFeeCandidate};

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasRankPlan {
    pub predicted_base_fee_gwei: DecimalAmount,
    pub candidates: Vec<RankedFeeCandidate>,
}

#[async_trait]
pub trait GasRankProvider: Send + Sync {
    async fn ranked_fee_candidates(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug)]
pub struct FixedGasRankProvider {
    plan: GasRankPlan,
}

impl FixedGasRankProvider {
    pub fn new(plan: GasRankPlan) -> Self {
        Self { plan }
    }
}

#[async_trait]
impl GasRankProvider for FixedGasRankProvider {
    async fn ranked_fee_candidates(
        &self,
        _input: &LivePrioritySellPlannerInput,
        _route: &PreparedSellRoute,
    ) -> Result<GasRankPlan, LivePrioritySellPlannerError> {
        Ok(self.plan.clone())
    }
}
