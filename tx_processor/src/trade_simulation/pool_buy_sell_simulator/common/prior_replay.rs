use std::collections::HashMap;
use std::sync::Arc;

use alloy_primitives::Address;
use eyre::{eyre, Result};
use tx_simulator::UnsignedTxChainSimulation;

use crate::processed_tx_builder::UnsignedTxBuilder;
use crate::trade_simulation::types::{PoolBuySellParameters, PoolBuySellSimulationResult};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;

use super::failure::format_failure_with_full_trace;
use super::fees::{apply_fee_policy, normalize_prior_fees_with_header};
use super::replay_funding::ensure_replay_sender_can_pay;
use super::results::create_failed_result;

pub(in crate::trade_simulation::pool_buy_sell_simulator) struct PriorReplayOutcome {
    pub transactions: Vec<ProcessedTransaction>,
    pub failure: Option<PoolBuySellSimulationResult>,
}

struct ReplayCandidate<'a> {
    source_index: usize,
    tx: &'a ProcessedTransaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PriorReplayDecision {
    Replay,
    SkipAlreadyApplied { state_nonce: u64 },
    SkipNonceGap { expected_nonce: u64 },
}

pub(in crate::trade_simulation::pool_buy_sell_simulator) async fn replay_prior_transactions(
    tx_processor: Arc<TxProcessor>,
    config: &PoolBuySellParameters,
    block_number: u64,
    base_fee: Option<u128>,
    chain: &mut UnsignedTxChainSimulation,
) -> Result<PriorReplayOutcome> {
    let replay_candidates = select_replay_candidates(&config.prior_txs, chain)?;
    let mut prior_tx_results = Vec::with_capacity(replay_candidates.len());

    for candidate in replay_candidates {
        let prior_tx = candidate.tx;
        let mut setup_call = UnsignedTxBuilder::build_unsigned_from_processed_tx(prior_tx);
        setup_call.nonce = Some(prior_tx.nonce);
        if setup_call.gas.is_none() {
            setup_call.gas = if prior_tx.fees.gas_limit > 0 {
                Some(prior_tx.fees.gas_limit)
            } else if prior_tx.fees.gas_used > 0 {
                Some(prior_tx.fees.gas_used)
            } else {
                None
            };
        }
        normalize_prior_fees_with_header(base_fee, prior_tx, &mut setup_call);

        let has_explicit_fee = setup_call.gas_price.is_some()
            || setup_call.max_fee_per_gas.is_some()
            || setup_call.max_priority_fee_per_gas.is_some();
        if !has_explicit_fee {
            apply_fee_policy(&mut setup_call, config, base_fee);
        }

        let prior_hash = format!("{:#x}", prior_tx.hash);
        let prior_nonce = prior_tx.nonce;
        if let Some(adjustment) = ensure_replay_sender_can_pay(chain, &setup_call)? {
            tracing::debug!(
                target: "pool_buy_sell_sim",
                step = "prior_replay_sender_funding",
                tx_hash = %prior_hash,
                sender = %adjustment.sender,
                previous_balance = %adjustment.previous_balance,
                replay_balance = %adjustment.replay_balance,
                "funding selected prior transaction sender for replay validation"
            );
        }

        let setup_sim_result = chain
            .step_with_trace(setup_call.clone())
            .await
            .map_err(|err| {
                let context = format!(
                    "while replaying prior tx {prior_hash} (index {}, nonce {prior_nonce}) with gas_limit {:?}, gas_price {:?}, max_fee {:?}, max_priority {:?}",
                    candidate.source_index,
                    setup_call.gas,
                    setup_call.gas_price,
                    setup_call.max_fee_per_gas,
                    setup_call.max_priority_fee_per_gas
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "prior_replay",
                    %context,
                    block = block_number,
                    error = %err
                );
                eyre!("{}: {}", context, err)
            })?;
        let setup_processed = tx_processor
            .process_transaction_from_simulation_result(
                &setup_call,
                &setup_sim_result,
                block_number,
                candidate.source_index as u64,
            )
            .await?;
        let succeeded = setup_sim_result.success;
        prior_tx_results.push(setup_processed);

        if !succeeded {
            let prior_to = prior_tx
                .to_address
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "contract creation".to_string());
            let base_message = format!(
                "Setup transaction replay failed tx={} mined_block={} tx_index={} nonce={} from={:#x} to={} mined_status={} simulation_base_block={}",
                prior_hash,
                prior_tx.block_number,
                prior_tx.tx_index,
                prior_nonce,
                prior_tx.from_address,
                prior_to,
                prior_tx.status,
                block_number
            );
            let failure_message = format!(
                "{}{}",
                format_failure_with_full_trace(&base_message, &setup_sim_result),
                if prior_tx.status {
                    "; mined receipt succeeded, so this is a setup replay mismatch rather than an on-chain transaction failure"
                } else {
                    ""
                }
            );
            tracing::warn!(
                target: "pool_buy_sell_sim",
                step = "setup_replay_failed",
                tx_hash = %prior_hash,
                mined_block = prior_tx.block_number,
                tx_index = prior_tx.tx_index,
                nonce = prior_nonce,
                from = %prior_tx.from_address,
                to = %prior_to,
                mined_status = prior_tx.status,
                simulation_base_block = block_number,
                failure = %failure_message
            );
            return Ok(PriorReplayOutcome {
                transactions: prior_tx_results.clone(),
                failure: Some(create_failed_result(
                    config.clone(),
                    block_number,
                    prior_tx_results,
                    None,
                    None,
                    None,
                    failure_message,
                    false,
                    false,
                    false,
                )),
            });
        }
    }

    Ok(PriorReplayOutcome {
        transactions: prior_tx_results,
        failure: None,
    })
}

fn select_replay_candidates<'a>(
    prior_txs: &'a [ProcessedTransaction],
    chain: &mut UnsignedTxChainSimulation,
) -> Result<Vec<ReplayCandidate<'a>>> {
    let mut candidates = Vec::with_capacity(prior_txs.len());
    let mut expected_nonces: HashMap<Address, u64> = HashMap::new();

    for (source_index, prior_tx) in prior_txs.iter().enumerate() {
        let sender = prior_tx.from_address;
        let expected_nonce = match expected_nonces.get(&sender) {
            Some(nonce) => *nonce,
            None => {
                let state_nonce = chain.account_nonce(sender)?;
                expected_nonces.insert(sender, state_nonce);
                state_nonce
            }
        };

        match classify_prior_replay_nonce(prior_tx.nonce, expected_nonce) {
            PriorReplayDecision::Replay => {
                expected_nonces.insert(sender, prior_tx.nonce.saturating_add(1));
                candidates.push(ReplayCandidate {
                    source_index,
                    tx: prior_tx,
                });
            }
            PriorReplayDecision::SkipAlreadyApplied { state_nonce } => {
                tracing::debug!(
                    target: "pool_buy_sell_sim",
                    step = "prior_replay_skipped_already_applied",
                    tx_hash = %format!("{:#x}", prior_tx.hash),
                    sender = %sender,
                    tx_nonce = prior_tx.nonce,
                    state_nonce,
                    simulated_block = prior_tx.block_number,
                    "skipping prior transaction already included in the selected base state"
                );
            }
            PriorReplayDecision::SkipNonceGap { expected_nonce } => {
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    step = "prior_replay_skipped_nonce_gap",
                    tx_hash = %format!("{:#x}", prior_tx.hash),
                    sender = %sender,
                    tx_nonce = prior_tx.nonce,
                    expected_nonce,
                    simulated_block = prior_tx.block_number,
                    "skipping prior transaction because an earlier sender nonce is missing"
                );
            }
        }
    }

    Ok(candidates)
}

fn classify_prior_replay_nonce(prior_nonce: u64, expected_nonce: u64) -> PriorReplayDecision {
    if prior_nonce < expected_nonce {
        return PriorReplayDecision::SkipAlreadyApplied {
            state_nonce: expected_nonce,
        };
    }

    if prior_nonce > expected_nonce {
        return PriorReplayDecision::SkipNonceGap { expected_nonce };
    }

    PriorReplayDecision::Replay
}

#[cfg(test)]
mod tests {
    use super::{classify_prior_replay_nonce, PriorReplayDecision};

    #[test]
    fn replays_next_nonce_only() {
        assert_eq!(
            classify_prior_replay_nonce(7, 7),
            PriorReplayDecision::Replay
        );
    }

    #[test]
    fn skips_nonce_already_consumed_by_base_state() {
        assert_eq!(
            classify_prior_replay_nonce(7, 8),
            PriorReplayDecision::SkipAlreadyApplied { state_nonce: 8 }
        );
    }

    #[test]
    fn skips_nonce_gap_without_forcing_replay_nonce_backwards() {
        assert_eq!(
            classify_prior_replay_nonce(9, 7),
            PriorReplayDecision::SkipNonceGap { expected_nonce: 7 }
        );
    }
}
