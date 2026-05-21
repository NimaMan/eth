use alloy_primitives::Address;
use tracing::{error, info};
use tx_processor::ProcessedTransaction;

use crate::tx_router::TransactionCategory;

use super::{mempool_tx_to_unsigned_tx, SimulationManager, SimulationResult, TxSimulationJob};

impl SimulationManager {
    pub(super) async fn simulate_liquidity_removal(
        &self,
        request: &TxSimulationJob,
        processed: &ProcessedTransaction,
        dependency_count: usize,
    ) -> Vec<SimulationResult> {
        info!(
            "💧 Starting dependency-aware liquidity removal analysis for TX {}",
            request.tx.hash
        );
        if dependency_count > 0 {
            info!(
                "💧 Liquidity removal TX {} includes {} same-sender nonce dependenc{} in replay",
                request.tx.hash,
                dependency_count,
                if dependency_count == 1 { "y" } else { "ies" }
            );
        }

        let unsigned_tx = match mempool_tx_to_unsigned_tx(&request.tx) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to convert transaction to call request: {}", e);
                let result = SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some(format!("Failed to convert transaction: {}", e)),
                    simulation_time_ms: 0.0,
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    liquidity_removal_result: None,
                };
                return vec![result];
            }
        };

        let sim_start = std::time::Instant::now();

        let removal_result = match self
            .liquidity_removal_simulator
            .result_from_processed_transaction(processed, Some(&unsigned_tx))
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!("Liquidity removal result extraction failed: {}", e);
                return vec![SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    liquidity_removal_result: None,
                    error: Some(format!("Liquidity removal result extraction failed: {}", e)),
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    simulation_time_ms: 0.0,
                }];
            }
        };

        let simulation_time = sim_start.elapsed().as_millis() as f64;

        info!(
            "  Simulation complete: success={}, is_scam={}, drain={}%",
            removal_result.success, removal_result.is_scam, removal_result.drain_percentage
        );

        let mut token_address = removal_result.token_address;

        if token_address.is_none() {
            token_address = match &request.category {
                TransactionCategory::CreatorTransaction {
                    target_token,
                    creator,
                    ..
                } => {
                    if let Some(token) = target_token {
                        match token.trim_start_matches("0x").parse::<Address>() {
                            Ok(addr) => Some(addr),
                            Err(e) => {
                                error!("Invalid token address: {}", e);
                                let result = SimulationResult {
                                    request: request.clone(),
                                    pool_viability_result: None,
                                    error: Some(format!("Invalid token address: {}", e)),
                                    simulation_time_ms: simulation_time,
                                    token_address: None,
                                    pool_address: None,
                                    pool_type: None,
                                    debug_info: None,
                                    liquidity_removal_result: None,
                                };
                                return vec![result];
                            }
                        }
                    } else if let Some(token_info) =
                        self.token_cache.get_token_for_creator(creator).await
                    {
                        match token_info
                            .address
                            .trim_start_matches("0x")
                            .parse::<Address>()
                        {
                            Ok(addr) => Some(addr),
                            Err(e) => {
                                let result = SimulationResult {
                                    request: request.clone(),
                                    pool_viability_result: None,
                                    error: Some(format!("Invalid token address: {}", e)),
                                    simulation_time_ms: simulation_time,
                                    token_address: None,
                                    pool_address: None,
                                    pool_type: None,
                                    debug_info: None,
                                    liquidity_removal_result: None,
                                };
                                return vec![result];
                            }
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
        }

        let Some(token_address) = token_address else {
            let result = SimulationResult {
                request: request.clone(),
                pool_viability_result: None,
                error: Some(
                    "unresolved_cache_context: liquidity removal has no mapped token/pool yet"
                        .to_string(),
                ),
                simulation_time_ms: simulation_time,
                token_address: None,
                pool_address: removal_result.pool_address,
                pool_type: removal_result.pool_type.clone(),
                debug_info: removal_result.debug_info.clone(),
                liquidity_removal_result: None,
            };
            return vec![result];
        };

        let result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,
            error: None,
            simulation_time_ms: simulation_time,
            token_address: Some(token_address),
            pool_address: removal_result.pool_address,
            pool_type: removal_result.pool_type.clone(),
            debug_info: None,
            liquidity_removal_result: Some(removal_result),
        };

        vec![result]
    }
}
