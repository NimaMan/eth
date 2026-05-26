use std::time::Duration;

use eth_live_feed::LiveTokenRuntime;
use eyre::{eyre, Result, WrapErr};
use tokio::time;

use super::wire::{LiveUnsignedTxSimulationRequest, LiveUnsignedTxSimulationResponse};

const LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT: Duration = Duration::from_millis(2_500);

pub async fn simulate_live_unsigned_transaction(
    live_tracker: &LiveTokenRuntime,
    request: LiveUnsignedTxSimulationRequest,
) -> Result<LiveUnsignedTxSimulationResponse> {
    if request.block == 0 {
        return Err(eyre!(
            "live unsigned tx simulation requires a non-zero block"
        ));
    }
    let (status, mut chain) = live_tracker
        .live_simulation_chain_at(request.block)
        .await
        .wrap_err_with(|| {
            format!(
                "exact live simulation state is unavailable for block {}",
                request.block
            )
        })?;
    let base_fee = chain.block_base_fee();
    let mut transaction = request.transaction;
    if transaction.gas_price.is_none() && transaction.max_fee_per_gas.is_none() {
        transaction.max_fee_per_gas = base_fee.or(Some(1));
        transaction.max_priority_fee_per_gas.get_or_insert(0);
    }

    let result = time::timeout(
        LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT,
        chain.step_with_trace(transaction),
    )
    .await
    .map_err(|_| {
        eyre!(
            "live unsigned tx simulation timed out after {} ms at block {}",
            LIVE_UNSIGNED_TX_SIMULATION_TIMEOUT.as_millis(),
            status.selected_block_number
        )
    })?
    .wrap_err_with(|| {
        format!(
            "live unsigned tx simulation failed at block {}",
            status.selected_block_number
        )
    })?;

    Ok(LiveUnsignedTxSimulationResponse {
        schema: "eth_live_unsigned_tx_simulation_v1",
        block: status.selected_block_number,
        block_hash: status.selected_block_hash,
        state_source: "chain_server_live_tx_simulator",
        success: result.success,
        gas_used: result.gas_used,
        effective_gas_price_wei: result.effective_gas_price.map(|value| value.to_string()),
        tx_type: result.tx_type,
        revert_reason: result.revert_reason,
        log_count: result.logs.len(),
        logs: result.logs,
        base_fee_per_gas_wei: base_fee.map(|value| value.to_string()),
    })
}
