use crate::function_detector::CreatorFunctionType;

use super::types::{RouteLane, TransactionCategory};

pub(crate) fn lane_for_creator_transaction(category: &TransactionCategory) -> RouteLane {
    let TransactionCategory::CreatorTransaction {
        target_token,
        function_type,
        ..
    } = category
    else {
        return RouteLane::Irrelevant;
    };

    match (function_type, target_token.is_some()) {
        (CreatorFunctionType::LiquidityPoolApproval, _) => RouteLane::CurrentTrackedPool,
        (CreatorFunctionType::LiquidityRemoval, true) => RouteLane::CurrentTrackedPool,
        (CreatorFunctionType::LiquidityRemoval, false) => RouteLane::UnresolvedCacheMapping,
        (_, true) => RouteLane::CurrentTrackedToken,
        (_, false) => RouteLane::UnresolvedCacheMapping,
    }
}
