use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::classification::classify_pool;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EligibilityTrack {
    pub eligible: bool,
    pub tradable_now: bool,
    pub cohort: String,
    pub category: String,
    pub eligible_outcome: Option<String>,
    pub reason_key: Option<String>,
    pub reason_label: Option<String>,
    pub quote_symbol: Option<String>,
    pub liquidity_class: String,
    pub evidence: Vec<EvidenceRef>,
}

impl EligibilityTrack {
    pub fn from_base_pool(base: &BasePool) -> Self {
        let input = base.classification_input();
        let classification = classify_pool(&input);
        Self {
            eligible: classification.eligible,
            tradable_now: classification.tradable_now,
            cohort: classification.cohort.key().to_string(),
            category: classification.category.key().to_string(),
            eligible_outcome: classification
                .eligible_outcome
                .map(|outcome| outcome.key().to_string()),
            reason_key: classification.reason_key.map(str::to_string),
            reason_label: classification.reason_label.map(str::to_string),
            quote_symbol: classification.quote_symbol,
            liquidity_class: format!("{:?}", classification.liquidity_class).to_ascii_lowercase(),
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::Classification,
                EvidenceConfidence::Inferred,
            )
            .at_block(base.latest_block_number)],
        }
    }
}
