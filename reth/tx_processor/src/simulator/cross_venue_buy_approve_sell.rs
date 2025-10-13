use alloy_primitives::{Address, I256, U256};
use eyre::Result;
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction};

use super::types::PoolBuySellParameters;
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;
use reth_chain_query::tx_builders::{self, amm_swap_route::AmmSwapRoute, spender_for_route};

#[derive(Debug, Clone)]
pub struct CrossVenueArbResult {
    pub token: Address,
    pub amount_in_eth: U256,
    pub tokens_bought: U256,
    pub eth_back: U256,
    pub buy_route: AmmSwapRoute,
    pub sell_route: AmmSwapRoute,
    pub buy_processed: ProcessedTransaction,
    pub approve_processed: Option<ProcessedTransaction>,
    pub sell_processed: ProcessedTransaction,
    pub gas_buy: u64,
    pub gas_approve: u64,
    pub gas_sell: u64,
    pub gas_total: u64,
    pub block_number: u64,
}

/// Simulate a cross-venue buy -> (approve) -> sell sequence at a fixed block.
/// - Buy ETH->token on `buy_route`
/// - Approve spender for `sell_route` if needed (MAX allowance)
/// - Sell token->ETH on `sell_route`
/// Returns detailed processed transactions and gas usage.
pub async fn simulate_cross_venue_buy_approve_sell(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token: Address,
    buy_route: AmmSwapRoute,
    sell_route: AmmSwapRoute,
    amount_in_eth: U256,
    block: u64,
) -> Result<CrossVenueArbResult> {
    // Start a simulation chain at the target block so state persists across steps
    let mut chain = simulator.start_simulation_chain(Some(block), None).await?;

    // Slippage and deadline
    let slippage_bps = 50u32; // 0.5%
    let deadline = u64::MAX;

    // Buyer/seller address: reuse default from PoolBuySellParameters for consistency
    let buyer = PoolBuySellParameters::default().buyer_address;

    // BUY
    let buy_tx: UnsignedTransaction = tx_builders::build_buy_swap(
        &buy_route,
        buyer,
        token,
        amount_in_eth,
        slippage_bps,
        deadline,
    );
    let buy_sim = chain.step_with_trace(buy_tx.clone()).await?;
    let buy_processed = tx_processor
        .process_transaction_from_simulation_result(&buy_tx, &buy_sim, block, 0)
        .await?;
    let tokens_bought = extract_token_increase(&buy_processed, buyer, token);

    // APPROVE for SELL route spender (if any). We always issue MAX approve here for simplicity.
    let approve_tx = tx_builders::build_approve_for_route(&sell_route, buyer, token, U256::MAX);
    let approve_sim = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(&approve_tx, &approve_sim, block, 1)
        .await?;

    // SELL entire bought amount
    let sell_tx = tx_builders::build_sell_swap(
        &sell_route,
        buyer,
        token,
        tokens_bought,
        slippage_bps,
        deadline,
    );
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let sell_processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 2)
        .await?;
    let eth_back = extract_eth_increase(&sell_processed, buyer);

    let gas_buy = buy_sim.gas_used;
    let gas_approve = approve_sim.gas_used;
    let gas_sell = sell_sim.gas_used;

    Ok(CrossVenueArbResult {
        token,
        amount_in_eth,
        tokens_bought,
        eth_back,
        buy_route,
        sell_route,
        buy_processed,
        approve_processed: Some(approve_processed),
        sell_processed,
        gas_buy,
        gas_approve,
        gas_sell,
        gas_total: gas_buy.saturating_add(gas_approve).saturating_add(gas_sell),
        block_number: block,
    })
}

fn extract_token_increase(
    processed: &ProcessedTransaction,
    owner: Address,
    token: Address,
) -> U256 {
    use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
    use reth_chain_query::to_checksum_address;
    if let Some(changes) = processed.address_balance_changes.get(&owner) {
        if let Some(sym) = get_token_symbol(&token) {
            if let Some(&amt) = changes.currency_net.get(sym) {
                if amt > I256::ZERO {
                    return amt.unsigned_abs();
                }
            }
        }
        let key = to_checksum_address(&token);
        if let Some(&amt) = changes.token_net.get(&key) {
            if amt > I256::ZERO {
                return amt.unsigned_abs();
            }
        }
    }
    U256::ZERO
}

fn extract_eth_increase(processed: &ProcessedTransaction, owner: Address) -> U256 {
    if let Some(changes) = processed.address_balance_changes.get(&owner) {
        if let Some(&amt) = changes.currency_net.get("ETH") {
            if amt > I256::ZERO {
                return amt.unsigned_abs();
            }
        }
    }
    U256::ZERO
}
