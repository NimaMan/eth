use crate::read_models::pool::PoolView;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolSurfaceFilter {
    All,
    Active,
    Eligible,
    Ineligible,
    Scam,
}

impl Default for PoolSurfaceFilter {
    fn default() -> Self {
        Self::All
    }
}

impl PoolSurfaceFilter {
    pub fn matches(self, pool: &PoolView) -> bool {
        match self {
            Self::All => true,
            Self::Active => is_active_pool(pool),
            Self::Eligible => pool.pool_classification.eligible,
            Self::Ineligible => !pool.pool_classification.eligible,
            Self::Scam => pool.is_scam,
        }
    }
}

pub fn is_active_pool(pool: &PoolView) -> bool {
    is_active_pool_parts(pool.pool_classification.eligible, pool.is_scam)
}

pub(crate) fn is_active_pool_parts(eligible: bool, is_scam: bool) -> bool {
    eligible && !is_scam
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_requires_eligible_and_non_scam() {
        assert!(is_active_pool_parts(true, false));
        assert!(!is_active_pool_parts(true, true));
        assert!(!is_active_pool_parts(false, false));
        assert!(!is_active_pool_parts(false, true));
    }
}
