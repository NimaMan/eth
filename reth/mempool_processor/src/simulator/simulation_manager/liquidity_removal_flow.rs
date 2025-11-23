use alloy_primitives::Address;
use tracing::{error, info};

use crate::common::convert::ipc_to_call_request;
use crate::tx_router::TransactionCategory;

use super::{SimulationManager, SimulationResult, TxSimulationJob};

impl SimulationManager {
    pub(super) async fn simulate_liquidity_removal(
        &self,
        request: &TxSimulationJob,
    ) -> Vec<SimulationResult> {
        info!(
            "💧 Starting liquidity removal simulation for TX {}",
            request.tx.hash
        );

        let call_request = match ipc_to_call_request(&request.tx.data) {
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

        let block_number = match self.mempool_simulator.latest_simulation_block().await {
            Ok(number) => Some(number),
            Err(err) => {
                error!(
                    "Failed to resolve target block for liquidity removal simulation: {}",
                    err
                );
                None
            }
        };

        let sim_start = std::time::Instant::now();

        let removal_result = match self
            .liquidity_removal_simulator
            .simulate_removal_with_retry(
                call_request,
                block_number,
                true,
                Some(request.tx.hash.as_str()),
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!("Liquidity removal simulation failed: {}", e);
                return vec![SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    liquidity_removal_result: None,
                    error: Some(format!("Simulation failed: {}", e)),
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

        let token_address = match &request.category {
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
                    let result = SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some("No token found for creator".to_string()),
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
            _ => {
                let result = SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some("Not a creator transaction".to_string()),
                    simulation_time_ms: simulation_time,
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    liquidity_removal_result: None,
                };
                return vec![result];
            }
        };

        let result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,
            error: None,
            simulation_time_ms: simulation_time,
            token_address,
            pool_address: removal_result.pool_address,
            pool_type: Some("V2".to_string()),
            debug_info: None,
            liquidity_removal_result: Some(removal_result),
        };

        vec![result]
    }
}
