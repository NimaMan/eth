use alloy_primitives::{Address as AlloyAddress, I256, U256};
use serde_json::to_string_pretty;
use tracing::{error, info, warn};
use tx_processor::{PoolBuySellParameters, PoolType, ProcessedTransaction};

use crate::{
    common::convert::ipc_to_call_request, token_tracking::PoolType as CachePoolType,
    tx_router::TransactionCategory,
};

use super::replay_context::{derive_pools_from_replay, PoolCandidate};
use super::{SimulationManager, SimulationResult, TxSimulationJob};

impl SimulationManager {
    pub(super) async fn simulate_tx_with_buy_sell_all_pools(
        &self,
        request: &TxSimulationJob,
        replay_sequence: &[ProcessedTransaction],
    ) -> Vec<SimulationResult> {
        // Extract token address from category
        let token_address_str = match &request.category {
            TransactionCategory::CreatorTransaction {
                target_token,
                creator,
                ..
            } => match target_token {
                Some(token) => token.clone(),
                None => {
                    if let Some(token_info) = self.token_cache.get_token_for_creator(creator).await
                    {
                        token_info.address.clone()
                    } else {
                        return vec![SimulationResult {
                            request: request.clone(),
                            pool_viability_result: None,
                            liquidity_removal_result: None,
                            error: Some(format!("No token address found for creator {}", creator)),
                            token_address: None,
                            pool_address: None,
                            pool_type: None,
                            debug_info: None,
                            simulation_time_ms: 0.0,
                        }];
                    }
                }
            },
            TransactionCategory::ContractCreation {
                contract_address, ..
            } => {
                if contract_address == "pending" {
                    return vec![SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        liquidity_removal_result: None,
                        error: Some(
                            "Contract creation: address calculation not implemented yet"
                                .to_string(),
                        ),
                        token_address: None,
                        pool_address: None,
                        pool_type: None,
                        debug_info: None,
                        simulation_time_ms: 0.0,
                    }];
                } else {
                    contract_address.clone()
                }
            }
            _ => {
                return vec![SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    liquidity_removal_result: None,
                    error: Some("Category doesn't support buy/sell simulation".to_string()),
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    simulation_time_ms: 0.0,
                }]
            }
        };

        // Convert string address to AlloyAddress
        let token_address = match token_address_str
            .trim_start_matches("0x")
            .parse::<AlloyAddress>()
        {
            Ok(addr) => addr,
            Err(e) => {
                return vec![SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    liquidity_removal_result: None,
                    error: Some(format!("Invalid token address: {}", e)),
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    simulation_time_ms: 0.0,
                }]
            }
        };

        // WETH constant used for ETH-denominated pools
        let weth_address = AlloyAddress::from([
            0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
            0xd9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
        ]);

        let mut pool_candidates: Vec<PoolCandidate> = self
            .token_cache
            .get_pools_for_token(&token_address_str)
            .await
            .into_iter()
            .map(PoolCandidate::from_cache)
            .collect();

        if pool_candidates.is_empty() {
            pool_candidates =
                derive_pools_from_replay(token_address, replay_sequence, weth_address);
            if pool_candidates.is_empty() {
                info!(
                    "  No pools found in cache or replay fallback for token {}",
                    token_address_str
                );
                return vec![];
            }

            info!(
                "  Using {} replay-derived pool(s) for token {}",
                pool_candidates.len(),
                token_address_str
            );
        }

        if pool_candidates.is_empty() {
            info!("  No pools found for token");
            return vec![];
        }

        let mut results = Vec::new();

        for (pool_idx, pool_state) in pool_candidates.into_iter().enumerate() {
            if matches!(pool_state.pool_type, CachePoolType::UniswapV4) {
                info!(
                    "  [Pool {}] Skipping Uniswap V4 pool {} for buy/sell entry probe; V4 entry signals require full V4 simulation support",
                    pool_idx, pool_state.address
                );
                results.push(SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: None,
                    simulation_time_ms: 0.0,
                    token_address: Some(token_address),
                    pool_address: None,
                    pool_type: Some(cache_pool_type_label(&pool_state.pool_type).to_string()),
                    debug_info: Some(
                        "entry_probe_unsupported: Uniswap V4 buy/sell simulation is not enabled"
                            .to_string(),
                    ),
                    liquidity_removal_result: None,
                });
                continue;
            }

            let pool_address = match pool_state
                .address
                .trim_start_matches("0x")
                .parse::<AlloyAddress>()
            {
                Ok(addr) => addr,
                Err(e) => {
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(format!(
                            "Invalid pool address {}: {}",
                            pool_state.address, e
                        )),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: None,
                        pool_type: Some(cache_pool_type_label(&pool_state.pool_type).to_string()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                    continue;
                }
            };

            let pool_type = cache_pool_type_label(&pool_state.pool_type).to_string();
            info!(
                "  [Pool {}] Simulating pool: {:?} (Type: {}, ETH: {:.6})",
                pool_idx, pool_address, &pool_type, pool_state.eth_reserve_hint
            );

            let denom_address = match pool_state
                .denom_address
                .trim_start_matches("0x")
                .parse::<AlloyAddress>()
            {
                Ok(addr) if !addr.is_zero() => addr,
                Ok(_) => {
                    info!(
                        "  [Pool {}] Denom address is zero, defaulting to WETH",
                        pool_idx
                    );
                    weth_address
                }
                Err(e) => {
                    warn!(
                        "  [Pool {}] Failed to parse denom address {}: {}. Defaulting to WETH",
                        pool_idx, pool_state.denom_address, e
                    );
                    weth_address
                }
            };

            let denom_decimals = if let Some(denom_token) =
                self.token_cache.get_token(&pool_state.denom_address).await
            {
                denom_token.decimals
            } else if pool_state.denom_currency.eq_ignore_ascii_case("ETH")
                || pool_state.denom_currency.eq_ignore_ascii_case("WETH")
            {
                18
            } else {
                18
            };

            let mut tx_call_request = match ipc_to_call_request(&request.tx.data) {
                Ok(req) => req,
                Err(e) => {
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(format!("Failed to convert transaction: {}", e)),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                    continue;
                }
            };

            if tx_call_request.gas.is_none() {
                error!("WARNING: Gas limit is None for transaction!",);
                error!(
                    "Raw IPC data: {}",
                    to_string_pretty(&request.tx.data).unwrap_or_default()
                );
                results.push(SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some("Gas limit missing from transaction - parsing error".to_string()),
                    simulation_time_ms: 0.0,
                    token_address: Some(token_address),
                    pool_address: Some(pool_address),
                    pool_type: Some(pool_type.clone()),
                    debug_info: None,
                    liquidity_removal_result: None,
                });
                continue;
            }

            tx_call_request.nonce = None;

            let original_gas_price = tx_call_request.gas_price;
            let original_max_fee = tx_call_request.max_fee_per_gas;

            let pool_type_enum = match simulation_pool_type(&pool_state) {
                Ok(pool_type) => pool_type,
                Err(reason) => {
                    warn!(
                        "  [Pool {}] Skipping pool {} ({}): {}",
                        pool_idx, pool_state.address, pool_type, reason
                    );
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(reason),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                    continue;
                }
            };

            let token_decimals =
                if let Some(token) = self.token_cache.get_token(&token_address_str).await {
                    token.decimals
                } else {
                    let error_msg = format!(
                        "Missing token decimals for pool {:?} (token {:?})",
                        pool_address, token_address
                    );
                    error!("{}", error_msg);
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(error_msg),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                    continue;
                };

            let leg_gas_limit: u64 = 500_000;
            let buyer_address = tx_call_request
                .from
                .unwrap_or_else(|| self.mempool_simulator.get_buyer_address());

            let config = PoolBuySellParameters {
                token_address,
                pool_address,
                pool_type: pool_type_enum,
                test_amount: U256::from(10_000_000_000_000_000u64),
                buyer_address,
                prior_txs: replay_sequence.to_vec(),
                block_number: None,
                block_header: None,
                slippage_tolerance: 5.0,
                gas_price: original_gas_price.map(|v| v as u128),
                max_fee_per_gas: original_max_fee.map(|v| v as u128),
                max_priority_fee_per_gas: tx_call_request.max_priority_fee_per_gas,
                buy_gas_limit: leg_gas_limit,
                approve_gas_limit: 200_000,
                sell_gas_limit: leg_gas_limit,
                weth_address,
                denom_address,
                denom_decimals,
                block_delay: 0,
                token_decimals,
                uniswap_v4_config: None,
            };

            let simulation_result = match self
                .mempool_simulator
                .simulate_pool_buy_sell(config.clone())
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("GasPriceLessThanBasefee")
                        || error_str.contains("base fee")
                    {
                        info!("  Base fee error detected, retrying with higher gas");
                        let new_gas_price = original_gas_price.map(|p| p * 3);
                        let new_max_fee = original_max_fee.map(|p| p * 3);

                        let retry_config = PoolBuySellParameters {
                            gas_price: new_gas_price.map(|v| v as u128),
                            max_fee_per_gas: new_max_fee.map(|v| v as u128),
                            max_priority_fee_per_gas: Some(2_000_000_000),
                            prior_txs: replay_sequence.to_vec(),
                            ..config
                        };

                        self.mempool_simulator
                            .simulate_pool_buy_sell(retry_config)
                            .await
                    } else {
                        Err(e)
                    }
                }
            };

            match simulation_result {
                Ok(result) => {
                    if let Some(failure) = result.failure_reason.clone() {
                        warn!(
                            "  [Pool {}] Simulation reported failure: {}",
                            pool_idx, failure
                        );
                    }

                    if let Some(ref revert_reason) = result.failure_reason {
                        warn!("  [DEBUG] Liquidity removal FAILED: {}", revert_reason);
                    }

                    for (addr, changes) in result
                        .buy_transaction
                        .address_balance_changes
                        .iter()
                        .take(3)
                    {
                        let eth_change = changes
                            .currency_net
                            .get("ETH")
                            .copied()
                            .unwrap_or(I256::ZERO);
                        info!("  [DEBUG] Address {} ETH change: {:?}", addr, eth_change);
                    }

                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: Some(result),
                        error: None,
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                }
                Err(e) => {
                    let error_msg = format!("Pool {}: {}", pool_idx, e);
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(error_msg),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                }
            }
        }

        results
    }
}

fn cache_pool_type_label(pool_type: &CachePoolType) -> &'static str {
    match pool_type {
        CachePoolType::UniswapV2 => "UNISWAP-V2",
        CachePoolType::UniswapV3 => "UNISWAP-V3",
        CachePoolType::UniswapV4 => "UNISWAP-V4",
        CachePoolType::Unknown => "UNKNOWN",
    }
}

fn simulation_pool_type(pool_state: &PoolCandidate) -> Result<PoolType, String> {
    match &pool_state.pool_type {
        CachePoolType::UniswapV2 | CachePoolType::Unknown => Ok(PoolType::UniswapV2),
        CachePoolType::UniswapV3 => {
            let fee_tier = pool_state.fee_tier.ok_or_else(|| {
                format!(
                    "Uniswap V3 pool {} is missing fee_tier in TokenTrackingCache",
                    pool_state.address
                )
            })?;
            Ok(PoolType::UniswapV3 { fee_tier })
        }
        CachePoolType::UniswapV4 => Err(format!(
            "Uniswap V4 pool {} is missing full pool-key simulation config in TokenTrackingCache",
            pool_state
                .pool_id
                .as_deref()
                .unwrap_or(pool_state.address.as_str())
        )),
    }
}
