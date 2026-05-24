use std::time::Instant;

use crate::tx_router::{CreatorFunctionType, TransactionCategory};

use super::{
    logging::{log_signal_dispatch_result, log_signal_dispatch_start},
    nonce_dependency_replay::merge_nonce_dependencies_with_replay_sequence,
    pending_sequences::sequence_key_from_request,
    SimulationManager, SimulationResult, TxSimulationJob,
};

impl SimulationManager {
    pub(in crate::simulator::simulation_manager) async fn handle_creator_transaction(
        &self,
        request: &TxSimulationJob,
        now: Instant,
    ) -> SimulationResult {
        let mut aggregate_result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            pool_type: None,
            debug_info: None,
            exact_vault_buy_result: None,
            liquidity_removal_result: None,
        };

        let processed_with_dependencies = match self
            .build_processed_transaction_with_nonce_dependencies(request, true)
            .await
        {
            Ok(processed) => processed,
            Err(err) => {
                aggregate_result.error =
                    Some(format!("Failed to process creator transaction: {}", err));
                return aggregate_result;
            }
        };
        let processed = processed_with_dependencies.transaction.clone();

        if let TransactionCategory::CreatorTransaction { function_type, .. } = &request.category {
            if matches!(function_type, CreatorFunctionType::LiquidityRemoval) {
                let dependency_count = processed_with_dependencies.dependencies.len();
                let removal_results = self
                    .simulate_liquidity_removal(request, &processed, dependency_count)
                    .await;
                if removal_results.is_empty() {
                    aggregate_result.error = Some("No pools found for token".to_string());
                    aggregate_result.debug_info =
                        Some("Liquidity removal simulation produced no pools".to_string());
                    return aggregate_result;
                }

                let mut last_success = None;
                let mut first_error = None;
                for (pool_idx, pool_specific_result) in removal_results.into_iter().enumerate() {
                    log_signal_dispatch_start(
                        pool_specific_result.request.tx.hash.as_str(),
                        Some(pool_idx),
                    );
                    let mut signal_manager = self.signal_manager.lock().await;
                    let signals = signal_manager
                        .process_simulation_result(&pool_specific_result)
                        .await;
                    log_signal_dispatch_result(signals.len(), Some(pool_idx));

                    if pool_specific_result.error.is_none() {
                        last_success = Some(pool_specific_result);
                    } else {
                        if aggregate_result.error.is_none() {
                            aggregate_result.error = pool_specific_result.error.clone();
                        }
                        if first_error.is_none() {
                            first_error = Some(pool_specific_result);
                        }
                    }
                }

                if let Some(success) = last_success {
                    return success;
                }
                if let Some(error_result) = first_error {
                    return error_result;
                }

                return aggregate_result;
            }
        }

        let replay_sequence =
            if let Some(key) = sequence_key_from_request(request, Some(&processed)) {
                let stored_sequence = self
                    .record_pending_transaction(key, request.tx_hash, processed.clone(), now)
                    .await;
                merge_nonce_dependencies_with_replay_sequence(
                    processed_with_dependencies.dependencies,
                    stored_sequence,
                )
            } else {
                merge_nonce_dependencies_with_replay_sequence(
                    processed_with_dependencies.dependencies,
                    vec![processed.clone()],
                )
            };

        let all_results = self
            .simulate_tx_with_buy_sell_all_pools(request, &replay_sequence)
            .await;

        if all_results.is_empty() {
            aggregate_result.error = Some("No pools found for token".to_string());
            aggregate_result.debug_info =
                Some("Token cache reported no pools; nothing to simulate".to_string());

            log_signal_dispatch_start(aggregate_result.request.tx.hash.as_str(), None);
            let mut signal_manager = self.signal_manager.lock().await;
            let signals = signal_manager
                .process_simulation_result(&aggregate_result)
                .await;
            log_signal_dispatch_result(signals.len(), None);

            return aggregate_result;
        }

        let mut last_success = None;
        let mut first_error = None;
        for (pool_idx, pool_specific_result) in all_results.into_iter().enumerate() {
            log_signal_dispatch_start(
                pool_specific_result.request.tx.hash.as_str(),
                Some(pool_idx),
            );
            let mut signal_manager = self.signal_manager.lock().await;
            let signals = signal_manager
                .process_simulation_result(&pool_specific_result)
                .await;
            log_signal_dispatch_result(signals.len(), Some(pool_idx));

            if pool_specific_result.error.is_none() {
                last_success = Some(pool_specific_result);
            } else {
                if aggregate_result.error.is_none() {
                    aggregate_result.error = pool_specific_result.error.clone();
                }
                if first_error.is_none() {
                    first_error = Some(pool_specific_result);
                }
            }
        }

        if let Some(success) = last_success {
            success
        } else if let Some(error_result) = first_error {
            error_result
        } else {
            aggregate_result
        }
    }
}
