use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use eyre::{eyre, Result};
use reth_chain_query::provider::BlockHeader;
use tx_processor::{
    sealed_header_from_processed_block_header, BlockStateSession, BlockTxStateSession,
    LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedTransaction,
    UnsignedTxChainSimulation,
};

use crate::erc20::ERC20Token;
use crate::pools::trading_failure::classify_v2_trading_failure;
use crate::pools::uniswap::{v4_event_display_key, UniswapV2TxContext};
use crate::pools::{PoolTradingSimulationConfig, UniswapV2Pool, UniswapV3Pool, UniswapV4Pool};
use crate::tracking::{address_string, hash_string, same_address_str};

use super::token_state_update::tx_control_addresses;
use super::{
    PoolTradingSimulationMode, LIVE_POOL_SIMULATION_TIMEOUT_MS, LIVE_TOKEN_TRACKER_LOG_TARGET,
};

const TOKEN_SIM_SESSION_PROFILE_LOG_TARGET: &str = "token_sim_session_profile";
const TOKEN_SIM_SESSION_PROFILE_ENV: &str = "TOKEN_SIM_SESSION_PROFILE";
static TOKEN_SIM_SESSION_PROFILE_ENABLED: OnceLock<bool> = OnceLock::new();

pub(super) async fn simulate_updated_v2_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    current_block_pool_addresses: &[String],
    trading_simulation: PoolTradingSimulationMode<'_>,
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
    let config = PoolTradingSimulationConfig::default();

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v2_pool(pool_address)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && (force_simulation || should_simulate_v2_trading(pool, tx))
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
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let tx_hash = hash_string(&tx.hash);
        tracing::debug!(
            target: "pool_buy_sell_sim",
            block_number = tx.block_number,
            token_address = %token_address,
            pool_address = %pool_address,
            tx_hash = %tx_hash,
            force_simulation,
            action = "evaluate_v2_trading",
            result = "started",
            "starting v2 pool trading simulation"
        );

        let simulation_result = match trading_simulation {
            PoolTradingSimulationMode::Historical(pool_simulator) => pool
                .evaluate_trading_status_v2_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            PoolTradingSimulationMode::HistoricalBlockSession {
                pool_simulator,
                block_session,
                profile_run_id,
            } => {
                let chain = block_session_chain_after_tx(
                    block_session,
                    pool_simulator,
                    tx,
                    "v2",
                    pool_address,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_block_session(pool_config, tx);
                pool.evaluate_trading_status_v2_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::HistoricalPostBlockSession {
                pool_simulator,
                block_sessions,
                profile_run_id,
            } => {
                let chain = historical_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v2",
                    pool_address,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_state_session(pool_config, tx);
                pool.evaluate_trading_status_v2_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
                direct_state_only,
                profile_run_id,
            } => {
                let Some(chain) = live_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v2",
                    pool_address,
                    direct_state_only,
                    profile_run_id,
                )
                .await?
                else {
                    continue;
                };
                let pool_config = pool_config_for_state_session(pool_config, tx);
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v2_with_chain(
                        pool_simulator,
                        &tx_context,
                        pool_config,
                        chain,
                    ),
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
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v2 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v2 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_address,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            PoolTradingSimulationMode::Noop => Ok(None),
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
                tracing::debug!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
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
                let error_detail = format!("{error:#}");
                if is_optional_direct_live_pool_simulation_error(
                    trading_simulation.uses_direct_live_state_only(),
                    &error_detail,
                ) {
                    tracing::debug!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        pool_kind = "v2",
                        block_number = tx.block_number,
                        tx_index = tx.tx_index,
                        tx_hash = %tx_hash,
                        token_address = %token_address,
                        pool_address = %pool_address,
                        force_simulation,
                        action = "pool_trading_simulation",
                        result = "skipped",
                        reason = %error,
                        "skipping direct live v2 pool trading simulation because optional live context is unavailable"
                    );
                    continue;
                }
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
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
    trading_simulation: PoolTradingSimulationMode<'_>,
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
    let config = PoolTradingSimulationConfig::default();

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v3_pool(pool_address)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && (force_simulation || should_simulate_v3_trading(pool, tx))
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
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let tx_hash = hash_string(&tx.hash);

        let simulation_result = match trading_simulation {
            PoolTradingSimulationMode::Historical(pool_simulator) => pool
                .evaluate_trading_status_v3_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            PoolTradingSimulationMode::HistoricalBlockSession {
                pool_simulator,
                block_session,
                profile_run_id,
            } => {
                let chain = block_session_chain_after_tx(
                    block_session,
                    pool_simulator,
                    tx,
                    "v3",
                    pool_address,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_block_session(pool_config, tx);
                pool.evaluate_trading_status_v3_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::HistoricalPostBlockSession {
                pool_simulator,
                block_sessions,
                profile_run_id,
            } => {
                let chain = historical_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v3",
                    pool_address,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_state_session(pool_config, tx);
                pool.evaluate_trading_status_v3_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
                direct_state_only,
                profile_run_id,
            } => {
                let Some(chain) = live_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v3",
                    pool_address,
                    direct_state_only,
                    profile_run_id,
                )
                .await?
                else {
                    continue;
                };
                let pool_config = pool_config_for_state_session(pool_config, tx);
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v3_with_chain(
                        pool_simulator,
                        &tx_context,
                        pool_config,
                        chain,
                    ),
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
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v3 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v3 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_address,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            PoolTradingSimulationMode::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
                tracing::debug!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
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
                let error_detail = format!("{error:#}");
                if is_optional_direct_live_pool_simulation_error(
                    trading_simulation.uses_direct_live_state_only(),
                    &error_detail,
                ) {
                    tracing::debug!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        pool_kind = "v3",
                        block_number = tx.block_number,
                        tx_index = tx.tx_index,
                        tx_hash = %tx_hash,
                        token_address = %token_address,
                        pool_address = %pool_address,
                        force_simulation,
                        action = "pool_trading_simulation",
                        result = "skipped",
                        reason = %error,
                        "skipping direct live v3 pool trading simulation because optional live context is unavailable"
                    );
                    continue;
                }
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
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
    trading_simulation: PoolTradingSimulationMode<'_>,
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
    let config = PoolTradingSimulationConfig::default();

    let mut simulated = Vec::new();
    for pool_key in pool_keys {
        let should_simulate = token
            .uniswap_v4_pool(pool_key)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && (force_simulation || should_simulate_v4_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }
        #[cfg(test)]
        if matches!(trading_simulation, PoolTradingSimulationMode::Noop) {
            continue;
        }

        let Some(pool) = token.uniswap_v4_pool_mut(pool_key) else {
            continue;
        };

        let simulate_at_current_block = should_simulate_v4_at_current_block(
            pool_key,
            current_block_pool_keys,
            trading_simulation,
            tx,
        );
        let pool_config = if simulate_at_current_block {
            PoolTradingSimulationConfig {
                block_number: Some(tx.block_number),
                block_header: block_header.cloned(),
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let tx_hash = hash_string(&tx.hash);

        let simulation_result = match trading_simulation {
            PoolTradingSimulationMode::Historical(pool_simulator) => pool
                .evaluate_trading_status_v4_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            PoolTradingSimulationMode::HistoricalBlockSession {
                pool_simulator,
                block_session,
                profile_run_id,
            } => {
                let chain = block_session_chain_after_tx(
                    block_session,
                    pool_simulator,
                    tx,
                    "v4",
                    pool_key,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_block_session(pool_config, tx);
                pool.evaluate_trading_status_v4_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::HistoricalPostBlockSession {
                pool_simulator,
                block_sessions,
                profile_run_id,
            } => {
                let chain = historical_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v4",
                    pool_key,
                    profile_run_id,
                )
                .await?;
                let pool_config = pool_config_for_state_session(pool_config, tx);
                pool.evaluate_trading_status_v4_with_pool_simulator_and_chain(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                    chain,
                )
                .await
                .map(|result| result.failure_reason.clone())
            }
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
                direct_state_only,
                profile_run_id,
            } => {
                let Some(chain) = live_block_state_session_chain(
                    block_sessions,
                    pool_simulator,
                    &pool_config,
                    tx,
                    "v4",
                    pool_key,
                    direct_state_only,
                    profile_run_id,
                )
                .await?
                else {
                    continue;
                };
                let pool_config = pool_config_for_state_session(pool_config, tx);
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v4_with_chain(
                        pool_simulator,
                        &tx_context,
                        pool_config,
                        chain,
                    ),
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
                            force_simulation,
                            timeout_ms = timeout.as_millis(),
                            action = "pool_trading_simulation",
                            result = "timeout",
                            "live v4 pool trading simulation timed out"
                        );
                        Err(eyre!(
                            "live v4 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} force_simulation={}",
                            timeout.as_millis(),
                            tx.block_number,
                            tx.tx_index,
                            tx_hash,
                            token_address,
                            pool_key,
                            force_simulation
                        ))
                    }
                }
            }
            #[cfg(test)]
            PoolTradingSimulationMode::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
                tracing::debug!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
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
                let error_detail = format!("{error:#}");
                if is_optional_direct_live_pool_simulation_error(
                    trading_simulation.uses_direct_live_state_only(),
                    &error_detail,
                ) {
                    tracing::debug!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        pool_kind = "v4",
                        block_number = tx.block_number,
                        tx_index = tx.tx_index,
                        tx_hash = %tx_hash,
                        token_address = %token_address,
                        pool_key = %pool_key,
                        force_simulation,
                        action = "pool_trading_simulation",
                        result = "skipped",
                        reason = %error,
                        "skipping direct live v4 pool trading simulation because optional live context is unavailable"
                    );
                    continue;
                }
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
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
    _trading_simulation: PoolTradingSimulationMode<'_>,
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

fn should_simulate_v4_at_current_block(
    pool_key: &str,
    current_block_pool_keys: &[String],
    trading_simulation: PoolTradingSimulationMode<'_>,
    tx: &ProcessedTransaction,
) -> bool {
    should_simulate_at_current_block(pool_key, current_block_pool_keys, trading_simulation)
        || has_v4_trading_simulation_trigger(tx, pool_key)
}

async fn block_session_chain_after_tx(
    block_session: &Mutex<Option<BlockTxStateSession>>,
    pool_simulator: &PoolBuySellSimulator,
    tx: &ProcessedTransaction,
    pool_kind: &'static str,
    pool_id: &str,
    profile_run_id: Option<&str>,
) -> Result<UnsignedTxChainSimulation> {
    let tx_index = usize::try_from(tx.tx_index)
        .map_err(|_| eyre!("tx index {} does not fit usize", tx.tx_index))?;

    let needs_session = block_session
        .lock()
        .map_err(|err| eyre!("block tx state session lock poisoned: {err}"))?
        .is_none();
    let mut session_create_us = 0;
    let mut session_created = false;
    if needs_session {
        let session_create_started = Instant::now();
        let session = pool_simulator
            .simulator()
            .block_tx_state_session(tx.block_number)
            .await
            .map_err(|err| {
                eyre!(
                    "failed to create block tx state session block={} for {} pool={}: {}",
                    tx.block_number,
                    pool_kind,
                    pool_id,
                    err
                )
            })?;
        session_create_us = elapsed_micros(session_create_started);
        let mut guard = block_session
            .lock()
            .map_err(|err| eyre!("block tx state session lock poisoned: {err}"))?;
        if guard.is_none() {
            *guard = Some(session);
            session_created = true;
        }
    }

    let mut guard = block_session
        .lock()
        .map_err(|err| eyre!("block tx state session lock poisoned: {err}"))?;
    let session = guard
        .as_mut()
        .ok_or_else(|| eyre!("block tx state session was not initialized"))?;
    let branch_started = Instant::now();
    let chain = session.simulation_chain_after_tx(tx_index).map_err(|err| {
        eyre!(
            "failed to create {} session branch block={} tx_index={} pool={}: {}",
            pool_kind,
            tx.block_number,
            tx.tx_index,
            pool_id,
            err
        )
    })?;
    let branch_us = elapsed_micros(branch_started);
    if token_sim_session_profile_enabled() {
        tracing::info!(
            target: TOKEN_SIM_SESSION_PROFILE_LOG_TARGET,
            run_id = profile_run_id.unwrap_or(""),
            mode = "historical",
            block_number = tx.block_number,
            tx_index = tx.tx_index,
            pool_kind,
            pool_id = %pool_id,
            session_needed = needs_session,
            session_created,
            session_create_us,
            session_create_ms = session_create_us / 1_000,
            branch_us,
            branch_ms = branch_us / 1_000,
            "token simulation session profile"
        );
    }
    Ok(chain)
}

fn pool_config_for_block_session(
    mut pool_config: PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
) -> PoolTradingSimulationConfig {
    pool_config.block_number = Some(tx.block_number);
    pool_config
}

async fn live_block_state_session_chain(
    block_sessions: &Mutex<BTreeMap<u64, BlockStateSession>>,
    pool_simulator: &LivePoolBuySellSimulator,
    pool_config: &PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
    pool_kind: &'static str,
    pool_id: &str,
    direct_state_only: bool,
    profile_run_id: Option<&str>,
) -> Result<Option<UnsignedTxChainSimulation>> {
    let block_number = state_session_block_number(pool_config, tx);
    let needs_session = !block_sessions
        .lock()
        .map_err(|err| eyre!("live block state session lock poisoned: {err}"))?
        .contains_key(&block_number);
    let mut session_create_us = 0;
    let mut session_created = false;

    if needs_session {
        if direct_state_only {
            tracing::debug!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number = tx.block_number,
                state_session_block_number = block_number,
                tx_index = tx.tx_index,
                tx_hash = %hash_string(&tx.hash),
                pool_kind,
                pool_id = %pool_id,
                "skipping live pool trading simulation because direct state session is unavailable"
            );
            return Ok(None);
        }
        if missing_live_current_block_header(pool_config, tx) {
            return Err(eyre!(
                "live current-block {} simulation requires ProcessedBlock.header block={} pool={}",
                pool_kind,
                block_number,
                pool_id
            ));
        }
        let session_create_started = Instant::now();
        let simulator = pool_simulator.simulator();
        let session = if let Some(block_header) = pool_config.block_header.as_ref() {
            if block_header.number != block_number {
                return Err(eyre!(
                    "live block state session header mismatch for {} pool={}: header={}, requested={}",
                    pool_kind,
                    pool_id,
                    block_header.number,
                    block_number
                ));
            }
            simulator
                .block_state_session_with_header(
                    block_number,
                    sealed_header_from_processed_block_header(block_header),
                )
                .await
        } else {
            simulator.block_state_session(block_number).await
        }
        .map_err(|err| {
            eyre!(
                "failed to create live block state session block={} for {} pool={}: {}",
                block_number,
                pool_kind,
                pool_id,
                err
            )
        })?;
        session_create_us = elapsed_micros(session_create_started);
        let mut guard = block_sessions
            .lock()
            .map_err(|err| eyre!("live block state session lock poisoned: {err}"))?;
        if !guard.contains_key(&block_number) {
            guard.insert(block_number, session);
            session_created = true;
        }
    }

    let guard = block_sessions
        .lock()
        .map_err(|err| eyre!("live block state session lock poisoned: {err}"))?;
    let session = guard.get(&block_number).ok_or_else(|| {
        eyre!(
            "live block state session was not initialized for block {}",
            block_number
        )
    })?;
    let branch_started = Instant::now();
    let chain = session.simulation_chain();
    let branch_us = elapsed_micros(branch_started);
    if token_sim_session_profile_enabled() {
        tracing::info!(
            target: TOKEN_SIM_SESSION_PROFILE_LOG_TARGET,
            run_id = profile_run_id.unwrap_or(""),
            mode = "live",
            block_number = tx.block_number,
            state_session_block_number = block_number,
            tx_index = tx.tx_index,
            pool_kind,
            pool_id = %pool_id,
            session_needed = needs_session,
            session_created,
            session_create_us,
            session_create_ms = session_create_us / 1_000,
            branch_us,
            branch_ms = branch_us / 1_000,
            session_cache_size = guard.len(),
            "token simulation session profile"
        );
    }
    Ok(Some(chain))
}

async fn historical_block_state_session_chain(
    block_sessions: &Mutex<BTreeMap<u64, BlockStateSession>>,
    pool_simulator: &PoolBuySellSimulator,
    pool_config: &PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
    pool_kind: &'static str,
    pool_id: &str,
    profile_run_id: Option<&str>,
) -> Result<UnsignedTxChainSimulation> {
    let block_number = state_session_block_number(pool_config, tx);
    let needs_session = !block_sessions
        .lock()
        .map_err(|err| eyre!("historical block state session lock poisoned: {err}"))?
        .contains_key(&block_number);
    let mut session_create_us = 0;
    let mut session_created = false;

    if needs_session {
        let session_create_started = Instant::now();
        let session = pool_simulator
            .simulator()
            .block_state_session(block_number)
            .await
            .map_err(|err| {
                eyre!(
                    "failed to create historical block state session block={} for {} pool={}: {}",
                    block_number,
                    pool_kind,
                    pool_id,
                    err
                )
            })?;
        session_create_us = elapsed_micros(session_create_started);
        let mut guard = block_sessions
            .lock()
            .map_err(|err| eyre!("historical block state session lock poisoned: {err}"))?;
        if !guard.contains_key(&block_number) {
            guard.insert(block_number, session);
            session_created = true;
        }
    }

    let guard = block_sessions
        .lock()
        .map_err(|err| eyre!("historical block state session lock poisoned: {err}"))?;
    let session = guard.get(&block_number).ok_or_else(|| {
        eyre!(
            "historical block state session was not initialized for block {}",
            block_number
        )
    })?;
    let branch_started = Instant::now();
    let chain = session.simulation_chain();
    let branch_us = elapsed_micros(branch_started);
    if token_sim_session_profile_enabled() {
        tracing::info!(
            target: TOKEN_SIM_SESSION_PROFILE_LOG_TARGET,
            run_id = profile_run_id.unwrap_or(""),
            mode = "historical_post_block",
            block_number = tx.block_number,
            state_session_block_number = block_number,
            tx_index = tx.tx_index,
            pool_kind,
            pool_id = %pool_id,
            session_needed = needs_session,
            session_created,
            session_create_us,
            session_create_ms = session_create_us / 1_000,
            branch_us,
            branch_ms = branch_us / 1_000,
            session_cache_size = guard.len(),
            "token simulation session profile"
        );
    }
    Ok(chain)
}

fn elapsed_micros(started: Instant) -> u128 {
    started.elapsed().as_micros()
}

fn token_sim_session_profile_enabled() -> bool {
    *TOKEN_SIM_SESSION_PROFILE_ENABLED.get_or_init(|| {
        std::env::var(TOKEN_SIM_SESSION_PROFILE_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false)
    })
}

fn pool_config_for_state_session(
    mut pool_config: PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
) -> PoolTradingSimulationConfig {
    pool_config.block_number = Some(state_session_block_number(&pool_config, tx));
    pool_config
}

fn state_session_block_number(
    pool_config: &PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
) -> u64 {
    pool_config
        .block_number
        .unwrap_or_else(|| tx.block_number.saturating_sub(1))
}

fn missing_live_current_block_header(
    pool_config: &PoolTradingSimulationConfig,
    tx: &ProcessedTransaction,
) -> bool {
    pool_config.block_header.is_none()
        && state_session_block_number(pool_config, tx) == tx.block_number
}

pub(super) fn is_optional_direct_live_pool_simulation_error(
    direct_state_only: bool,
    message: &str,
) -> bool {
    direct_state_only
        && (message.contains("live chain cache not configured")
            || message.contains("missing live block header"))
}

pub(super) fn should_simulate_v2_trading(pool: &UniswapV2Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.has_liquidity_removal() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v2_trading_simulation_trigger(tx, pool_address))
}

pub(super) fn should_simulate_v3_trading(pool: &UniswapV3Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.has_liquidity_removal() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v3_trading_simulation_trigger(tx, pool_address))
}

pub(super) fn should_simulate_v4_trading(pool: &UniswapV4Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.has_liquidity_removal() {
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

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, B256, U256};
    use tx_processor::tx_processor::data_models::receipt_models::UniswapV4SwapEvent;

    use super::*;

    #[test]
    fn v4_event_trigger_uses_current_block_even_without_current_pool_list_entry() {
        let pool_manager = address!("000000000004444c5dc75cb358380d2e3de08a90");
        let pool_id = b256!("896b942201672b004f285d16b1467dab7d77e957daa550fc406688ec60269ac4");
        let pool_key = v4_event_display_key(pool_manager, pool_id);
        let mut tx = ProcessedTransaction::new(
            B256::ZERO,
            25_086_268,
            0,
            54,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Some(pool_manager),
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );
        tx.uniswap_v4_swaps.push(UniswapV4SwapEvent {
            pool_manager_address: pool_manager,
            event_id: pool_id,
            sender: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount0: -1,
            amount1: 1,
            sqrt_price_x96: U256::ZERO,
            liquidity: 1,
            tick: 0,
            fee: 3000,
            log_index: 1,
        });

        assert!(should_simulate_v4_at_current_block(
            &pool_key,
            &[],
            PoolTradingSimulationMode::Noop,
            &tx
        ));
    }

    #[test]
    fn live_current_block_simulation_requires_header_hint() {
        let tx = ProcessedTransaction::new(
            B256::ZERO,
            25_086_268,
            0,
            54,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );
        let current_block_config = PoolTradingSimulationConfig {
            block_number: Some(tx.block_number),
            ..Default::default()
        };
        let parent_block_config = PoolTradingSimulationConfig::default();

        assert!(missing_live_current_block_header(
            &current_block_config,
            &tx
        ));
        assert!(!missing_live_current_block_header(
            &parent_block_config,
            &tx
        ));
    }
}
