use std::time::Duration;

use eyre::{eyre, Result};
use reth_chain_query::provider::BlockHeader;
use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20Token;
use crate::pools::trading_failure::classify_v2_trading_failure;
use crate::pools::uniswap::{
    v4_event_display_key, PoolTradingSimulationConfig, UniswapV2TxContext,
};
use crate::pools::{UniswapV2Pool, UniswapV3Pool, UniswapV4Pool};
use crate::tracking::{address_string, hash_string, same_address_str};

use super::touches::tx_control_addresses;
use super::{V2TradingSimulation, LIVE_POOL_SIMULATION_TIMEOUT_MS, LIVE_TOKEN_TRACKER_LOG_TARGET};

pub(super) async fn simulate_updated_v2_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    current_block_pool_addresses: &[String],
    prior_txs: &[ProcessedTransaction],
    trading_simulation: V2TradingSimulation<'_>,
    force_simulation: bool,
    block_header: Option<&BlockHeader>,
) -> Result<Vec<String>> {
    if pool_addresses.is_empty() {
        return Ok(Vec::new());
    }

    let token_address = token.contract_address.clone();
    let tx_context = UniswapV2TxContext {
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_hash: hash_string(&tx.hash),
        from_address: Some(address_string(&tx.from_address)),
    };
    let config = PoolTradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v2_pool(pool_address)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v2_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }

        let Some(pool) = token.uniswap_v2_pool_mut(pool_address) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_address,
            current_block_pool_addresses,
            trading_simulation,
        ) {
            PoolTradingSimulationConfig {
                block_number: Some(tx.block_number),
                block_header: block_header.cloned(),
                prior_txs: Vec::new(),
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let prior_tx_count = pool_config.prior_txs.len();
        let tx_hash = hash_string(&tx.hash);
        tracing::debug!(
            target: "pool_buy_sell_sim",
            block_number = tx.block_number,
            token_address = %token_address,
            pool_address = %pool_address,
            tx_hash = %tx_hash,
            prior_tx_count,
            force_simulation,
            action = "evaluate_v2_trading",
            result = "started",
            "starting v2 pool trading simulation"
        );

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v2_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v2(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
                    Err(_) => {
                        tracing::warn!(
                            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                            pool_kind = "v2",
                            block_number = tx.block_number,
                            tx_index = tx.tx_index,
                            tx_hash = %tx_hash,
                            token_address = %token_address,
                            pool_address = %pool_address,
                            prior_tx_count,
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v2 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v2 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_address,
                            prior_tx_count,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                let failure_class = failure_reason
                    .as_ref()
                    .and_then(|reason| classify_v2_trading_failure(pool, tx, reason));
                let failure_class_code = failure_class.map(|class| class.code());
                let failure_class_description = failure_class.map(|class| class.description());
                pool.base.set_trading_failure_context(
                    failure_reason.clone(),
                    failure_class_code.map(|code| code.to_string()),
                );
                tracing::info!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    can_buy = pool.base.state.can_buy,
                    can_sell = pool.base.state.can_sell,
                    buy_tax = ?pool.base.buy_tax,
                    sell_tax = ?pool.base.sell_tax,
                    reason = ?failure_reason,
                    failure_class = ?failure_class_code,
                    failure_class_description = ?failure_class_description,
                    action = "evaluate_v2_trading",
                    result = "ok",
                    "completed v2 pool trading simulation"
                );
            }
            Err(error) => {
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    action = "evaluate_v2_trading",
                    result = "error",
                    reason = %error,
                    "failed v2 pool trading simulation"
                );
                return Err(error);
            }
        }
        simulated.push(pool_address.clone());
    }

    Ok(simulated)
}

pub(super) async fn simulate_updated_v3_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    current_block_pool_addresses: &[String],
    prior_txs: &[ProcessedTransaction],
    trading_simulation: V2TradingSimulation<'_>,
    force_simulation: bool,
    block_header: Option<&BlockHeader>,
) -> Result<Vec<String>> {
    if pool_addresses.is_empty() {
        return Ok(Vec::new());
    }

    let token_address = token.contract_address.clone();
    let tx_context = UniswapV2TxContext {
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_hash: hash_string(&tx.hash),
        from_address: Some(address_string(&tx.from_address)),
    };
    let config = PoolTradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v3_pool(pool_address)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v3_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }

        let Some(pool) = token.uniswap_v3_pool_mut(pool_address) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_address,
            current_block_pool_addresses,
            trading_simulation,
        ) {
            PoolTradingSimulationConfig {
                block_number: Some(tx.block_number),
                block_header: block_header.cloned(),
                prior_txs: Vec::new(),
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let prior_tx_count = pool_config.prior_txs.len();
        let tx_hash = hash_string(&tx.hash);

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v3_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v3(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
                    Err(_) => {
                        tracing::warn!(
                            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                            pool_kind = "v3",
                            block_number = tx.block_number,
                            tx_index = tx.tx_index,
                            tx_hash = %tx_hash,
                            token_address = %token_address,
                            pool_address = %pool_address,
                            prior_tx_count,
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v3 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v3 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_address,
                            prior_tx_count,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
                tracing::info!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    can_buy = pool.base.state.can_buy,
                    can_sell = pool.base.state.can_sell,
                    buy_tax = ?pool.base.buy_tax,
                    sell_tax = ?pool.base.sell_tax,
                    reason = ?failure_reason,
                    action = "evaluate_v3_trading",
                    result = "ok",
                    "completed v3 pool trading simulation"
                );
            }
            Err(error) => {
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    action = "evaluate_v3_trading",
                    result = "error",
                    reason = %error,
                    "failed v3 pool trading simulation"
                );
                return Err(error);
            }
        }
        simulated.push(pool_address.clone());
    }

    Ok(simulated)
}

pub(super) async fn simulate_updated_v4_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_keys: &[String],
    current_block_pool_keys: &[String],
    prior_txs: &[ProcessedTransaction],
    trading_simulation: V2TradingSimulation<'_>,
    force_simulation: bool,
    block_header: Option<&BlockHeader>,
) -> Result<Vec<String>> {
    if pool_keys.is_empty() {
        return Ok(Vec::new());
    }

    let token_address = token.contract_address.clone();
    let tx_context = UniswapV2TxContext {
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_hash: hash_string(&tx.hash),
        from_address: Some(address_string(&tx.from_address)),
    };
    let config = PoolTradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_key in pool_keys {
        let should_simulate = token
            .uniswap_v4_pool(pool_key)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v4_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }
        #[cfg(test)]
        if matches!(trading_simulation, V2TradingSimulation::Noop) {
            continue;
        }

        let Some(pool) = token.uniswap_v4_pool_mut(pool_key) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_key,
            current_block_pool_keys,
            trading_simulation,
        ) {
            PoolTradingSimulationConfig {
                block_number: Some(tx.block_number),
                block_header: block_header.cloned(),
                prior_txs: Vec::new(),
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let prior_tx_count = pool_config.prior_txs.len();
        let tx_hash = hash_string(&tx.hash);

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v4_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v4(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
                    Err(_) => {
                        tracing::warn!(
                            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                            pool_kind = "v4",
                            block_number = tx.block_number,
                            tx_index = tx.tx_index,
                            tx_hash = %tx_hash,
                            token_address = %token_address,
                            pool_key = %pool_key,
                            prior_tx_count,
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v4 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v4 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_key,
                            prior_tx_count,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
                tracing::info!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    can_buy = pool.base.state.can_buy,
                    can_sell = pool.base.state.can_sell,
                    buy_tax = ?pool.base.buy_tax,
                    sell_tax = ?pool.base.sell_tax,
                    reason = ?failure_reason,
                    action = "evaluate_v4_trading",
                    result = "ok",
                    "completed v4 pool trading simulation"
                );
            }
            Err(error) => {
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    action = "evaluate_v4_trading",
                    result = "error",
                    reason = %error,
                    "failed v4 pool trading simulation"
                );
                return Err(error);
            }
        }
        simulated.push(pool_key.clone());
    }

    Ok(simulated)
}

pub(super) fn should_simulate_at_current_block(
    pool_address: &str,
    current_block_pool_addresses: &[String],
    _trading_simulation: V2TradingSimulation<'_>,
) -> bool {
    current_block_pool_addresses
        .iter()
        .any(|current_pool| current_pool == pool_address)
}

pub(super) fn simulation_pool_addresses(
    all_pool_addresses: Vec<String>,
    updated: &[String],
    include_all_token_pools: bool,
) -> Vec<String> {
    let mut addresses = updated.to_vec();
    if include_all_token_pools {
        addresses.extend(all_pool_addresses);
    }
    addresses.sort();
    addresses.dedup();
    addresses
}

pub(super) fn current_block_simulation_pool_addresses(
    simulation_pool_addresses: &[String],
    updated: &[String],
    discovered: &[String],
    force_simulation: bool,
) -> Vec<String> {
    let mut addresses = if force_simulation {
        simulation_pool_addresses.to_vec()
    } else {
        Vec::new()
    };
    addresses.extend(updated.iter().cloned());
    addresses.extend(discovered.iter().cloned());
    addresses.sort();
    addresses.dedup();
    addresses
}

pub(super) fn simulation_prior_txs(
    prior_txs: &[ProcessedTransaction],
    current_tx: &ProcessedTransaction,
    include_current_tx: bool,
) -> Vec<ProcessedTransaction> {
    let mut simulation_prior_txs = prior_txs.to_vec();
    if include_current_tx
        && !simulation_prior_txs
            .iter()
            .any(|prior_tx| prior_tx.hash == current_tx.hash)
    {
        simulation_prior_txs.push(current_tx.clone());
    }
    simulation_prior_txs.sort_by_key(|tx| tx.tx_index);
    simulation_prior_txs.dedup_by_key(|tx| tx.hash);
    simulation_prior_txs
}

fn should_simulate_v2_trading(pool: &UniswapV2Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v2_trading_simulation_trigger(tx, pool_address))
}

fn should_simulate_v3_trading(pool: &UniswapV3Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v3_trading_simulation_trigger(tx, pool_address))
}

fn should_simulate_v4_trading(pool: &UniswapV4Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_key = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v4_trading_simulation_trigger(tx, pool_key))
}

fn has_v2_trading_simulation_trigger(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_swaps
        .iter()
        .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
}

fn has_v3_trading_simulation_trigger(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v3_initializations
        .iter()
        .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_swaps
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_mints
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_burns
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
}

fn has_v4_trading_simulation_trigger(tx: &ProcessedTransaction, pool_key: &str) -> bool {
    tx.uniswap_v4_initializes
        .iter()
        .any(|event| v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key)
        || tx.uniswap_v4_modifies.iter().any(|event| {
            v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key
        })
        || tx.uniswap_v4_swaps.iter().any(|event| {
            v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key
        })
}
