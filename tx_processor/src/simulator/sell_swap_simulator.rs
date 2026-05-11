use alloy_primitives::{keccak256, Address, Bytes, I256, U256};
use eyre::{eyre, Result};
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation};

use super::types::{PoolBuySellParameters, PoolType};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;
use tx_simulator::tx_builders::{self, amm_swap_route::AmmSwapRoute};

const BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const ERC20_BALANCE_SLOT_SEARCH_LIMIT: u64 = 64;

#[derive(Debug, Clone)]
pub struct SellSwapResult {
    pub success: bool,
    pub seller_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub tokens_sold: U256,
    pub denom_received: U256,
    pub sell_transaction: ProcessedTransaction,
    pub block_number: u64,
    pub failure_reason: Option<String>,
}

pub async fn simulate_sell_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    token_address: Address,
    pool_address: Address,
    pool_type: PoolType,
    tokens_to_sell: U256,
    block_number: Option<u64>,
) -> Result<SellSwapResult> {
    // Defaults consistent with buy simulator
    let slippage_tolerance = 0.5_f64;
    let default_cfg = PoolBuySellParameters::default();
    let seller_address = default_cfg.buyer_address;

    let block = match block_number {
        Some(b) => b,
        None => simulator.latest_historical_context_block_number()?,
    };

    // Build route
    let route = if let Some(protocol) = pool_type.known_v2_protocol() {
        AmmSwapRoute::V2Router {
            pool: pool_address,
            router: protocol.router(),
        }
    } else {
        match pool_type {
            PoolType::UniswapV3 { fee_tier } => AmmSwapRoute::UniswapV3 {
                pool: pool_address,
                fee_tier,
            },
            _ => {
                return Err(eyre::eyre!(
                    "Pool type {:?} not yet supported for sell-only simulation",
                    pool_type
                ));
            }
        }
    };

    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee = chain.block_base_fee();
    chain.set_eth_balance(seller_address, U256::from(1_000_000_000_000_000_000u128))?;
    let Some(balance_slot) =
        inject_standard_erc20_balance(&mut chain, token_address, seller_address, tokens_to_sell)?
    else {
        return Err(eyre!(
            "unable to inject synthetic ERC20 balance for token {token_address:#x}; unsupported balance storage layout"
        ));
    };
    tracing::debug!(
        token = %token_address,
        seller = %seller_address,
        balance_slot,
        amount = %tokens_to_sell,
        "injected synthetic ERC20 balance for chain-sim sell"
    );

    let mut approve_tx =
        tx_builders::build_approve_for_route(&route, seller_address, token_address, tokens_to_sell);
    apply_sell_fee_policy(&mut approve_tx, default_cfg.approve_gas_limit, base_fee);
    let approve_sim = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(&approve_tx, &approve_sim, block, 0)
        .await?;
    if !approve_sim.success {
        return Ok(SellSwapResult {
            success: false,
            seller_address,
            token_address,
            pool_address,
            pool_type,
            tokens_sold: tokens_to_sell,
            denom_received: U256::ZERO,
            sell_transaction: approve_processed,
            block_number: block,
            failure_reason: Some(format_failure_with_revert(
                "Sell approve transaction failed",
                approve_sim.revert_reason.as_deref(),
            )),
        });
    }

    // Build SELL transaction
    let slippage_bps = (slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let mut sell_tx = tx_builders::build_sell_swap(
        &route,
        seller_address,
        token_address,
        tokens_to_sell,
        slippage_bps,
        deadline,
    );
    apply_sell_fee_policy(&mut sell_tx, default_cfg.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 1)
        .await?;

    // Extract ETH received by seller
    let denom_received = extract_denom_received(&processed, seller_address);
    let success = sell_sim.success;

    Ok(SellSwapResult {
        success,
        seller_address,
        token_address,
        pool_address,
        pool_type,
        tokens_sold: tokens_to_sell,
        denom_received,
        sell_transaction: processed,
        block_number: block,
        failure_reason: if success {
            None
        } else {
            Some(format_failure_with_revert(
                "Sell transaction failed",
                sell_sim.revert_reason.as_deref(),
            ))
        },
    })
}

fn inject_standard_erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    owner: Address,
    amount: U256,
) -> Result<Option<u64>> {
    for slot in 0..ERC20_BALANCE_SLOT_SEARCH_LIMIT {
        let storage_key = standard_erc20_balance_storage_key(owner, slot);
        let previous = chain.set_account_storage(token_address, storage_key, amount)?;
        let observed = read_erc20_balance(chain, token_address, owner)?;
        if observed == amount {
            return Ok(Some(slot));
        }
        chain.set_account_storage(token_address, storage_key, previous)?;
    }
    Ok(None)
}

fn standard_erc20_balance_storage_key(owner: Address, slot: u64) -> U256 {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(owner.into_word().as_slice());
    input[32..].copy_from_slice(&U256::from(slot).to_be_bytes::<32>());
    U256::from_be_slice(keccak256(input).as_slice())
}

fn read_erc20_balance(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
    owner: Address,
) -> Result<U256> {
    let mut calldata = Vec::with_capacity(36);
    calldata.extend_from_slice(&BALANCE_OF_SELECTOR);
    calldata.extend_from_slice(owner.into_word().as_slice());
    let result = chain.simulate_view_call(token_address, Bytes::from(calldata))?;
    if !result.success || result.output.len() < 32 {
        return Ok(U256::ZERO);
    }
    Ok(U256::from_be_slice(&result.output[..32]))
}

fn apply_sell_fee_policy(tx: &mut UnsignedTransaction, gas_limit: u64, base_fee: Option<u128>) {
    tx.gas = Some(gas_limit);
    if let Some(base_fee) = base_fee {
        tx.gas_price = None;
        tx.max_fee_per_gas = Some(base_fee);
        tx.max_priority_fee_per_gas = Some(0);
    } else if tx.gas_price.is_none() && tx.max_fee_per_gas.is_none() {
        tx.gas_price = Some(1);
    }
}

fn format_failure_with_revert(prefix: &str, revert_reason: Option<&str>) -> String {
    match revert_reason.filter(|reason| !reason.trim().is_empty()) {
        Some(reason) => format!("{prefix}: {reason}"),
        None => prefix.to_string(),
    }
}

fn extract_denom_received(processed_tx: &ProcessedTransaction, recipient_address: Address) -> U256 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if let Some(&amount) = balance_changes.currency_net.get("ETH") {
            if amount > I256::ZERO {
                return amount.unsigned_abs();
            }
        }
    }
    U256::ZERO
}
