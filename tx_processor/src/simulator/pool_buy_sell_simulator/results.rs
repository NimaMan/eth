use alloy_primitives::{B256, U256};

use crate::tx_processor::data_models::ProcessedTransaction;

use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult};

pub(super) fn create_failed_result(
    config: PoolBuySellParameters,
    block_number: u64,
    prior_txs: Vec<ProcessedTransaction>,
    buy_tx: Option<ProcessedTransaction>,
    approve_tx: Option<ProcessedTransaction>,
    sell_tx: Option<ProcessedTransaction>,
    failure_reason: String,
    can_buy: bool,
    can_approve: bool,
    can_sell: bool,
) -> PoolBuySellSimulationResult {
    let mut dummy_tx = ProcessedTransaction::new(
        B256::ZERO,
        block_number,
        0,
        0,
        config.buyer_address,
        None,
        U256::ZERO,
        false,
        0,
        0,
        Vec::new(),
    );
    dummy_tx.tx_type = "UNKNOWN".to_string();
    PoolBuySellSimulationResult {
        pool_type: config.pool_type,
        pool_address: config.pool_address,
        token_address: config.token_address,
        can_buy,
        can_approve,
        can_sell,
        is_tradeable: false,
        buy_tax_percent: -1.0,
        sell_tax_percent: -1.0,
        tokens_received: U256::ZERO,
        denom_spent: config.test_amount,
        denom_received: U256::ZERO,
        buy_transaction: buy_tx.unwrap_or_else(|| dummy_tx.clone()),
        sell_transaction: sell_tx.unwrap_or_else(|| dummy_tx.clone()),
        approve_transaction: approve_tx.unwrap_or_else(|| dummy_tx.clone()),
        prior_transactions: prior_txs,
        failure_reason: Some(failure_reason),
        block_number,
    }
}
