use crate::function_detector::CreatorFunctionType;

use super::lane::RouteLane;

#[derive(Debug, Clone)]
pub enum TransactionCategory {
    ContractCreation {
        deployer: String,
        contract_address: String,
        is_token: bool,
        has_liquidity_in_calldata: bool,
    },
    CreatorTransaction {
        creator: String,
        target_address: String,
        target_token: Option<String>,
        function_type: CreatorFunctionType,
    },
    Regular {
        is_transfer: bool,
        is_approval: bool,
    },
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub category: TransactionCategory,
    pub lane: RouteLane,
    pub priority: SimulationPriority,
    pub requires_simulation: bool,
    pub requires_buy_sell_test: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

pub(crate) fn should_route_tracked_token_call(function_type: &CreatorFunctionType) -> bool {
    matches!(
        function_type,
        CreatorFunctionType::TradingControl
            | CreatorFunctionType::TaxModification
            | CreatorFunctionType::MaxWalletLimit
            | CreatorFunctionType::OwnershipChange
            | CreatorFunctionType::LiquidityPoolApproval
            | CreatorFunctionType::TokenSupplyModification
    )
}

pub(crate) fn priority_for_creator_function(
    function_type: &CreatorFunctionType,
) -> SimulationPriority {
    match function_type {
        CreatorFunctionType::TaxModification => SimulationPriority::Critical,
        CreatorFunctionType::TradingControl => SimulationPriority::Critical,
        CreatorFunctionType::OwnershipChange => SimulationPriority::High,
        CreatorFunctionType::LiquidityAddition => SimulationPriority::High,
        CreatorFunctionType::LiquidityRemoval => SimulationPriority::Critical,
        CreatorFunctionType::LiquidityPoolApproval => SimulationPriority::Critical,
        CreatorFunctionType::MaxWalletLimit => SimulationPriority::High,
        CreatorFunctionType::TokenSupplyModification => SimulationPriority::Critical,
        CreatorFunctionType::Other(_) => SimulationPriority::High,
    }
}
