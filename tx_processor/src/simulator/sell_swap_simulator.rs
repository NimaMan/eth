use alloy_primitives::{address, keccak256, Address, Bytes, I256, U256};
use eyre::{eyre, Result};
use reth_chain_query::to_checksum_address;
use std::sync::Arc;
use tx_simulator::{TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation};

use super::types::{PoolBuySellParameters, PoolType};
use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::TxProcessor;
use tx_simulator::tx_builders::{
    self,
    amm_swap_route::AmmSwapRoute,
    permit2::build_permit2_approve_tx,
    uniswap_v4::{
        build_token_approval_tx, build_universal_router_v4_exact_input_single_tx,
        infer_orientation_from_input, UniswapV4PoolKey as BuilderV4PoolKey,
        UniversalRouterV4ExactInputSingleRequest, UniversalRouterV4InputPayment,
    },
};

const BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const ERC20_BALANCE_SLOT_SEARCH_LIMIT: u64 = 64;
const UNIVERSAL_ROUTER_V4: Address = address!("66a9893cC07D91D95644AEDD05D03f95e1dBA8Af");
const PERMIT2: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");
const PERMIT2_EXPIRATION: u64 = (1_u64 << 48) - 1;

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
    let mut config = PoolBuySellParameters::new(token_address, pool_address, pool_type);
    config.block_number = block_number;
    simulate_sell_swap_with_params(simulator, tx_processor, config, tokens_to_sell).await
}

pub async fn simulate_sell_swap_with_params(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    mut config: PoolBuySellParameters,
    tokens_to_sell: U256,
) -> Result<SellSwapResult> {
    let seller_address = config.buyer_address;

    let block = match config.block_number {
        Some(b) => b,
        None => simulator.latest_historical_context_block_number()?,
    };
    config.block_number = Some(block);

    if matches!(config.pool_type, PoolType::UniswapV4) {
        return simulate_v4_sell_swap(simulator, tx_processor, config, tokens_to_sell, block).await;
    }

    // Build route
    let route = if let Some(protocol) = config.pool_type.known_v2_protocol() {
        AmmSwapRoute::V2Router {
            pool: config.pool_address,
            router: protocol.router(),
        }
    } else if let (Some(protocol), Some(fee_tier)) = (
        config.pool_type.known_v3_protocol(),
        config.pool_type.v3_fee_tier(),
    ) {
        AmmSwapRoute::V3Router {
            pool: config.pool_address,
            router: protocol.router(),
            fee_tier,
        }
    } else {
        match config.pool_type {
            _ => {
                return Err(eyre::eyre!(
                    "Pool type {:?} not yet supported for sell-only simulation",
                    config.pool_type
                ));
            }
        }
    };

    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee = chain.block_base_fee();
    chain.set_eth_balance(seller_address, U256::from(1_000_000_000_000_000_000u128))?;
    let Some(balance_slot) = inject_standard_erc20_balance(
        &mut chain,
        config.token_address,
        seller_address,
        tokens_to_sell,
    )?
    else {
        return Err(eyre!(
            "unable to inject synthetic ERC20 balance for token {:#x}; unsupported balance storage layout",
            config.token_address
        ));
    };
    tracing::debug!(
        token = %config.token_address,
        seller = %seller_address,
        balance_slot,
        amount = %tokens_to_sell,
        "injected synthetic ERC20 balance for chain-sim sell"
    );

    let mut approve_tx = tx_builders::build_approve_for_route(
        &route,
        seller_address,
        config.token_address,
        tokens_to_sell,
    );
    apply_sell_fee_policy(&mut approve_tx, config.approve_gas_limit, base_fee);
    let approve_sim = chain.step_with_trace(approve_tx.clone()).await?;
    let approve_processed = tx_processor
        .process_transaction_from_simulation_result(&approve_tx, &approve_sim, block, 0)
        .await?;
    if !approve_sim.success {
        return Ok(SellSwapResult {
            success: false,
            seller_address,
            token_address: config.token_address,
            pool_address: config.pool_address,
            pool_type: config.pool_type,
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
    let slippage_bps = (config.slippage_tolerance * 100.0).round() as u32;
    let deadline = u64::MAX;
    let mut sell_tx = tx_builders::build_sell_swap(
        &route,
        seller_address,
        config.token_address,
        tokens_to_sell,
        slippage_bps,
        deadline,
    );
    apply_sell_fee_policy(&mut sell_tx, config.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 1)
        .await?;

    // Extract ETH received by seller
    let denom_received = extract_denom_received(&processed, seller_address, config.denom_address);
    let success = sell_sim.success;

    Ok(SellSwapResult {
        success,
        seller_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
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

async fn simulate_v4_sell_swap(
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    config: PoolBuySellParameters,
    tokens_to_sell: U256,
    block: u64,
) -> Result<SellSwapResult> {
    let v4_cfg = config
        .uniswap_v4_config
        .clone()
        .ok_or_else(|| eyre!("Uniswap V4 configuration must be provided"))?;
    let mut chain = simulator.start_simulation_chain(Some(block)).await?;
    let base_fee = chain.block_base_fee();
    let seller_address = config.buyer_address;

    if !chain.account_has_code(v4_cfg.pool_manager)? {
        return Err(eyre!(
            "Uniswap V4 PoolManager {:#x} has no bytecode at block {}",
            v4_cfg.pool_manager,
            block
        ));
    }

    let pool_key = BuilderV4PoolKey {
        currency0: v4_cfg.currency0,
        currency1: v4_cfg.currency1,
        fee: v4_cfg.fee,
        tick_spacing: v4_cfg.tick_spacing,
        hooks: v4_cfg.hooks,
    };
    let sell_orientation = infer_orientation_from_input(&pool_key, config.token_address)?;
    if !currency_matches_denom(
        sell_orientation.output_currency,
        config.denom_address,
        config.weth_address,
    ) {
        return Err(eyre!(
            "Uniswap V4 sell output currency {:#x} does not match configured denom {:#x}",
            sell_orientation.output_currency,
            config.denom_address
        ));
    }

    chain.set_eth_balance(seller_address, U256::from(1_000_000_000_000_000_000u128))?;
    let Some(balance_slot) = inject_standard_erc20_balance(
        &mut chain,
        config.token_address,
        seller_address,
        tokens_to_sell,
    )?
    else {
        return Err(eyre!(
            "unable to inject synthetic ERC20 balance for token {:#x}; unsupported balance storage layout",
            config.token_address
        ));
    };
    tracing::debug!(
        token = %config.token_address,
        seller = %seller_address,
        balance_slot,
        amount = %tokens_to_sell,
        "injected synthetic ERC20 balance for V4 chain-sim sell"
    );

    let mut token_approve =
        build_token_approval_tx(seller_address, config.token_address, PERMIT2, U256::MAX);
    apply_sell_fee_policy(&mut token_approve, config.approve_gas_limit, base_fee);
    let token_approve_sim = chain.step_with_trace(token_approve.clone()).await?;
    let token_approve_processed = tx_processor
        .process_transaction_from_simulation_result(&token_approve, &token_approve_sim, block, 0)
        .await?;
    if !token_approve_sim.success {
        return Ok(v4_failed_sell_result(
            &config,
            tokens_to_sell,
            token_approve_processed,
            block,
            "Token approval for Permit2 failed",
            token_approve_sim.revert_reason.as_deref(),
        ));
    }

    let mut permit2_approve = build_permit2_approve_tx(
        seller_address,
        PERMIT2,
        config.token_address,
        UNIVERSAL_ROUTER_V4,
        permit2_amount(tokens_to_sell)?,
        PERMIT2_EXPIRATION,
    )?;
    apply_sell_fee_policy(&mut permit2_approve, config.approve_gas_limit, base_fee);
    let permit2_approve_sim = chain.step_with_trace(permit2_approve.clone()).await?;
    let permit2_approve_processed = tx_processor
        .process_transaction_from_simulation_result(
            &permit2_approve,
            &permit2_approve_sim,
            block,
            1,
        )
        .await?;
    if !permit2_approve_sim.success {
        return Ok(v4_failed_sell_result(
            &config,
            tokens_to_sell,
            permit2_approve_processed,
            block,
            "Permit2 approval for Universal Router failed",
            permit2_approve_sim.revert_reason.as_deref(),
        ));
    }

    let mut sell_tx = build_universal_router_v4_exact_input_single_tx(
        &UniversalRouterV4ExactInputSingleRequest {
            universal_router: UNIVERSAL_ROUTER_V4,
            caller: seller_address,
            pool_key,
            token_in: config.token_address,
            token_out: sell_orientation.output_currency,
            amount_in: tokens_to_sell,
            min_amount_out: U256::ZERO,
            deadline: U256::from(u64::MAX),
            hook_data: v4_cfg.hook_data,
            input_payment: UniversalRouterV4InputPayment::Permit2User,
        },
    )?;
    sell_tx.gas = Some(config.sell_gas_limit);
    apply_sell_fee_policy(&mut sell_tx, config.sell_gas_limit, base_fee);
    let sell_sim = chain.step_with_trace(sell_tx.clone()).await?;
    let processed = tx_processor
        .process_transaction_from_simulation_result(&sell_tx, &sell_sim, block, 2)
        .await?;
    let denom_received = extract_denom_received(&processed, seller_address, config.denom_address);
    let success = sell_sim.success;

    Ok(SellSwapResult {
        success,
        seller_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        tokens_sold: tokens_to_sell,
        denom_received,
        sell_transaction: processed,
        block_number: block,
        failure_reason: if success {
            None
        } else {
            Some(format_failure_with_revert(
                "Universal Router V4 sell transaction failed",
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

fn v4_failed_sell_result(
    config: &PoolBuySellParameters,
    tokens_to_sell: U256,
    processed: ProcessedTransaction,
    block: u64,
    message: &str,
    revert_reason: Option<&str>,
) -> SellSwapResult {
    SellSwapResult {
        success: false,
        seller_address: config.buyer_address,
        token_address: config.token_address,
        pool_address: config.pool_address,
        pool_type: config.pool_type,
        tokens_sold: tokens_to_sell,
        denom_received: U256::ZERO,
        sell_transaction: processed,
        block_number: block,
        failure_reason: Some(format_failure_with_revert(message, revert_reason)),
    }
}

fn currency_matches_denom(currency: Address, denom: Address, weth: Address) -> bool {
    currency == denom || (currency.is_zero() && (denom.is_zero() || denom == weth))
}

fn permit2_amount(amount: U256) -> Result<U256> {
    let max = (U256::from(1_u8) << 160) - U256::from(1_u8);
    if amount > max {
        return Err(eyre!("Permit2 allowance amount must fit uint160"));
    }
    Ok(amount)
}

fn extract_denom_received(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    denom_address: Address,
) -> U256 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if denom_address.is_zero() {
            if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                if amount > I256::ZERO {
                    return amount.unsigned_abs();
                }
            }
        }

        if let Some(symbol) = get_token_symbol(&denom_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > I256::ZERO {
                    return amount.unsigned_abs();
                }
            }
            if symbol == "WETH" {
                if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                    if amount > I256::ZERO {
                        return amount.unsigned_abs();
                    }
                }
            }
        }

        let token_key = to_checksum_address(&denom_address);
        if let Some(&amount) = balance_changes.token_net.get(&token_key) {
            if amount > I256::ZERO {
                return amount.unsigned_abs();
            }
        }
    }
    U256::ZERO
}
