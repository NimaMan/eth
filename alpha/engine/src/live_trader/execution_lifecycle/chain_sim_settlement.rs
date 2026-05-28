use crate::LiveChainSimExecutionAdapter;
use eth_alpha_core::{error::Result, execution::ExecutionReport};
use eth_alpha_store::PostgresTradingStore;
use tracing::warn;

use super::super::backtest::ChainSimGasPolicyBacktestAdapter;

const CHAIN_SIM_SUBMITTED_SETTLEMENT_LIMIT: usize = 100;

#[derive(Clone)]
pub(in crate::live_trader) struct ChainSimSettlement {
    store: PostgresTradingStore,
    simulator: LiveChainSimExecutionAdapter,
    gas_shadow: Option<ChainSimGasPolicyBacktestAdapter<LiveChainSimExecutionAdapter>>,
}

#[derive(Default)]
pub(in crate::live_trader) struct ChainSimSettlementBatch {
    pub reports: Vec<ExecutionReport>,
    pub loaded: usize,
    pub pending_future_block: usize,
    pub waiting_for_live_state: usize,
    pub missing_submission_block: usize,
}

impl ChainSimSettlement {
    pub(in crate::live_trader) fn new(
        store: PostgresTradingStore,
        simulator: LiveChainSimExecutionAdapter,
        gas_shadow: Option<ChainSimGasPolicyBacktestAdapter<LiveChainSimExecutionAdapter>>,
    ) -> Self {
        Self {
            store,
            simulator,
            gas_shadow,
        }
    }

    pub(in crate::live_trader) async fn reports_due_at(
        &self,
        current_block: u64,
    ) -> Result<ChainSimSettlementBatch> {
        let submitted = self
            .store
            .load_chain_sim_submitted_executions(CHAIN_SIM_SUBMITTED_SETTLEMENT_LIMIT)
            .await?;
        let mut batch = ChainSimSettlementBatch {
            loaded: submitted.len(),
            ..Default::default()
        };

        for record in submitted {
            let submitted_block = record.submitted_block_number.or_else(|| {
                record
                    .expected_confirmation_block
                    .and_then(|block| block.checked_sub(1))
            });
            let Some(submitted_block) = submitted_block else {
                batch.missing_submission_block += 1;
                warn!(
                    order_id = %record.order_id.0,
                    position_id = %record.position_id.0,
                    trade_id = ?record.trade_id.as_ref().map(|id| id.0.as_str()),
                    order_side = ?record.order_side,
                    token_address = %record.token_address,
                    "chain-sim submitted execution cannot be settled because submitted block is missing"
                );
                continue;
            };
            let Some(execution_block) = record
                .expected_confirmation_block
                .or_else(|| submitted_block.checked_add(1))
            else {
                batch.missing_submission_block += 1;
                warn!(
                    order_id = %record.order_id.0,
                    position_id = %record.position_id.0,
                    submitted_block,
                    "chain-sim submitted execution cannot be settled because execution block overflowed"
                );
                continue;
            };

            if execution_block > current_block {
                batch.pending_future_block += 1;
                continue;
            }
            let Some(mut report) = self
                .simulator
                .simulate_submitted_order(
                    record.order_id,
                    record.intent.clone(),
                    submitted_block,
                    execution_block,
                    record.submitted_block_hash,
                )
                .await?
            else {
                batch.waiting_for_live_state += 1;
                warn!(
                    position_id = %record.position_id.0,
                    order_side = ?record.order_side,
                    submitted_block,
                    submitted_block_hash = ?record.submitted_block_hash,
                    execution_block,
                    current_block,
                    "chain-sim submitted execution is due but chain-server exact simulation state is unavailable"
                );
                continue;
            };
            if let Some(gas_shadow) = &self.gas_shadow {
                gas_shadow
                    .apply_shadow_if_applicable(&record.intent, &mut report)
                    .await;
            }
            batch.reports.push(report);
        }

        Ok(batch)
    }
}
