use serde::{Deserialize, Serialize};

use crate::atlas::{RiskAtlasPageStory, RiskAtlasStorySection};
use crate::db::schema::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, ModelReadinessItem, NumericStat,
    ReviewExample, RiskAtlasRun,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasPageView {
    pub run: RiskAtlasRun,
    pub page_story: RiskAtlasPageStory,
    pub sections: Vec<RiskAtlasStorySection>,
    pub distributions: Vec<DistributionBucket>,
    pub numeric_stats: Vec<NumericStat>,
    pub active_targets: Vec<ActiveTargetSummary>,
    pub decision_questions: Vec<DecisionQuestion>,
    pub review_examples: Vec<ReviewExample>,
    pub model_readiness: Vec<ModelReadinessItem>,
}
