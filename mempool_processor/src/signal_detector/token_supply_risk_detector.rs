use std::collections::HashSet;

use crate::simulator::SimulationResult;
use crate::tx_router::{CreatorFunctionType, TransactionCategory};

use super::{Signal, TokenSupplyRiskSignal};

pub struct TokenSupplyRiskDetector {
    emitted: HashSet<(String, String, String)>,
}

impl TokenSupplyRiskDetector {
    pub fn new() -> Self {
        Self {
            emitted: HashSet::new(),
        }
    }

    pub fn detect(&mut self, result: &SimulationResult) -> Option<Signal> {
        let TransactionCategory::CreatorTransaction {
            creator,
            target_address,
            target_token: Some(token_address),
            function_type: CreatorFunctionType::TokenSupplyModification,
        } = &result.request.category
        else {
            return None;
        };

        // Public supply-risk signals are only emitted for calls sent directly
        // to the tracked token contract. Creator calls to unrelated helper
        // contracts can share the same selector and should stay simulation-only.
        if !target_address.eq_ignore_ascii_case(token_address) {
            return None;
        }

        let selector = result
            .request
            .tx
            .input
            .get(0..4)
            .map(hex::encode)
            .unwrap_or_else(|| "unknown".to_string());
        let (risk_type, risk_details) = token_supply_risk_for_selector(&selector);
        let tx_hash = result.request.tx.hash.clone();
        if !self.emitted.insert((
            token_address.clone(),
            tx_hash.clone(),
            risk_type.to_string(),
        )) {
            return None;
        }

        Some(Signal::TokenSupplyRisk(TokenSupplyRiskSignal {
            tx_hash: Some(tx_hash),
            token_address: token_address.clone(),
            risk_type: risk_type.to_string(),
            risk_details: risk_details.to_string(),
            actor_address: Some(creator.clone()),
            block_number: result
                .pool_viability_result
                .as_ref()
                .map(|pool_result| pool_result.block_number),
            confidence: 0.75,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }))
    }
}

fn token_supply_risk_for_selector(selector: &str) -> (&'static str, &'static str) {
    match selector {
        "40c10f19" => (
            "mint_function_pending",
            "pending mint(address,uint256)-style call on tracked token contract",
        ),
        "e58306f9" => (
            "admin_mint_function_pending",
            "pending admin mint call on tracked token contract",
        ),
        "484b973c" => (
            "owner_mint_function_pending",
            "pending owner mint call on tracked token contract",
        ),
        "627804af" => (
            "dev_mint_function_pending",
            "pending developer mint call on tracked token contract",
        ),
        "68573107" => (
            "batch_mint_function_pending",
            "pending batch mint call on tracked token contract",
        ),
        "8ba4cc3c" => (
            "airdrop_function_pending",
            "pending airdrop-style supply distribution call on tracked token contract",
        ),
        _ => (
            "supply_mutator_pending",
            "pending supply-mutating call on tracked token contract",
        ),
    }
}
