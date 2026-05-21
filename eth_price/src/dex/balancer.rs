//! Balancer V2 Price Reader.
//!
//! Handles reading state from Balancer V2 pools via the main Vault contract.
//!
//! # Implementation Details
//!
//! *   **Vault Architecture:** Unlike other DEXs, Balancer stores all token balances in a single "Vault" contract.
//! *   **Simulation:** Uses `TxSimulator` to call `getPoolTokens(poolId)` on the Vault to retrieve balances.
//! *   **Weighted Math:** Computes spot prices using the weighted geometric mean formula for Weighted Pools (e.g., 80/20 pools).
//! *   **Token Management:** Maps internal token indices to their actual contract addresses and weights.

use crate::core::{
    LiquidityMetrics, Pool, PoolKind, PoolState, PriceData, PriceError, PricePair, PriceSource,
    Protocol, RawChainPrice, Token,
};
use crate::shared_simulator::get_or_create_simulator_with_provider;
use alloy_primitives::{Address, B256, U256};
use reth_chain_query::dex::get_balancer_pool_tokens;
use reth_provider::{BlockNumReader, BlockReader};
use std::collections::HashMap;
use std::sync::Arc;
use tx_simulator::TxSimulator;

#[derive(Clone, Debug)]
struct TokenInfo {
    address: Address,
    symbol: String,
    decimals: u8,
    index: usize,
}

#[derive(Clone, Debug)]
struct PoolInfo {
    pool_id: B256,
    pool_address: Address,
    tokens: Vec<TokenInfo>,
    weights: Vec<U256>,
    base_index: usize,
    quote_index: usize,
    swap_fee_bps: u32,
}

#[derive(Clone)]
pub struct BalancerReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    pool_info: HashMap<String, PoolInfo>,
    simulator: Arc<TxSimulator>,
}

impl BalancerReader {
    pub fn new(db_path_str: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path_str)?)
    }

    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        let simulator = get_or_create_simulator_with_provider(provider_factory.clone())?;
        let mut pool_info = HashMap::new();

        for info in reth_chain_query::common_addresses::dex_token_denom_pairs::balancer_pools() {
            let mut tokens = Vec::new();
            let mut weights = Vec::new();

            for (i, t) in info.tokens.iter().enumerate() {
                tokens.push(TokenInfo {
                    address: t.token_address,
                    symbol: t.symbol.to_string(),
                    decimals: t.decimals,
                    index: i, // Ensure index matches position in the vector
                });
                weights.push(t.weight.unwrap_or(U256::ZERO));
            }

            let (base_index, quote_index, label) = match info.name {
                "BAL-WETH 80/20" => (0, 1, "BAL/WETH_BAL"), // Base BAL, Quote WETH
                "WETH-USDC 50/50" => (1, 0, "ETH/USDC"), // Base WETH, Quote USDC. Wait, client requests "ETH/USDC".
                _ => (0, 1, info.name),
            };

            pool_info.insert(
                label.to_string(),
                PoolInfo {
                    pool_id: info.pool_id,
                    pool_address: info.pool_address,
                    tokens,
                    weights,
                    base_index,
                    quote_index,
                    swap_fee_bps: info.swap_fee_bps,
                },
            );
        }

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            pool_info,
            simulator,
        })
    }

    pub fn supported_pairs(&self) -> Vec<String> {
        self.pool_info.keys().cloned().collect()
    }

    pub async fn get_latest_price(&self, pair: &str) -> Result<PriceData, PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        self.get_price_at_block(pair, latest_block).await
    }

    pub async fn get_price_at_block(
        &self,
        pair: &str,
        block_number: u64,
    ) -> Result<PriceData, PriceError> {
        let info = self
            .pool_info
            .get(pair)
            .ok_or_else(|| PriceError::PriceNotAvailable(pair.to_string()))?;

        let pool_id_bytes: [u8; 32] = info.pool_id.into();
        let (tokens, balances) =
            match get_balancer_pool_tokens(&self.simulator, pool_id_bytes, Some(block_number))
                .await
                .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            {
                Some(result) => result,
                None => {
                    return Err(PriceError::PriceNotAvailable(format!(
                        "Balancer pool tokens unavailable for {}",
                        pair
                    )))
                }
            };

        if tokens.len() <= info.base_index || tokens.len() <= info.quote_index {
            return Err(PriceError::PriceNotAvailable(format!(
                "Balancer token indices invalid for {}",
                pair
            )));
        }

        let base_token_info = &info.tokens[info.base_index];
        let quote_token_info = &info.tokens[info.quote_index];
        let base_balance_raw = balances
            .get(base_token_info.index)
            .copied()
            .ok_or_else(|| PriceError::PriceNotAvailable("Missing base balance".into()))?;
        let quote_balance_raw = balances
            .get(quote_token_info.index)
            .copied()
            .ok_or_else(|| PriceError::PriceNotAvailable("Missing quote balance".into()))?;

        let base_weight = info.weights[info.base_index];
        let quote_weight = info.weights[info.quote_index];
        let mut scaled_base = base_balance_raw;
        let mut scaled_quote = quote_balance_raw;

        for _ in 0..192 {
            if scaled_base.checked_mul(quote_weight).is_some()
                && scaled_quote.checked_mul(base_weight).is_some()
            {
                break;
            }
            scaled_base >>= 1;
            scaled_quote >>= 1;
        }

        let numerator = scaled_quote.checked_mul(base_weight).ok_or_else(|| {
            PriceError::ConversionError("Failed to compute Balancer numerator".into())
        })?;
        let denominator = scaled_base.checked_mul(quote_weight).ok_or_else(|| {
            PriceError::ConversionError("Failed to compute Balancer denominator".into())
        })?;

        if denominator.is_zero() {
            return Err(PriceError::PriceNotAvailable(
                "Balancer denominator is zero".into(),
            ));
        }

        let precise_price = RawChainPrice::new(numerator, denominator);

        let base_token = Token::new(
            base_token_info.address,
            base_token_info.symbol.clone(),
            base_token_info.decimals,
        );
        let quote_token = Token::new(
            quote_token_info.address,
            quote_token_info.symbol.clone(),
            quote_token_info.decimals,
        );

        if info.tokens.len() < 2 {
            return Err(PriceError::PriceNotAvailable(
                "Balancer pool missing token metadata".into(),
            ));
        }

        let token_a = Token::new(
            info.tokens[0].address,
            info.tokens[0].symbol.clone(),
            info.tokens[0].decimals,
        );
        let token_b = Token::new(
            info.tokens[1].address,
            info.tokens[1].symbol.clone(),
            info.tokens[1].decimals,
        );

        let pool_kind = PoolKind::Balancer {
            balances: balances.clone(),
            weights: info.weights.clone(),
            swap_fee_bps: info.swap_fee_bps,
        };

        let pool = Pool::new(
            info.pool_address,
            Protocol::Balancer,
            token_a,
            token_b,
            pool_kind,
        );
        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;
        let pool_state = PoolState::new(pool.clone(), block_number, block.timestamp);
        let liquidity = LiquidityMetrics::from_pool(&pool);

        let price_pair = PricePair::new(pair.to_string(), base_token.clone(), quote_token.clone());
        let price_id = pool.create_price_id(base_token.address, quote_token.address);

        let source = PriceSource::Amm(crate::core::AmmPriceSource {
            protocol: Protocol::Balancer,
            price_id,
            pool,
            pool_state,
            liquidity,
        });

        Ok(PriceData::new(
            price_pair,
            precise_price,
            block_number,
            block.timestamp,
            source,
        ))
    }

    pub async fn get_weighted_two_token_price_latest_by_pool(
        &self,
        _vault: Address,
        pool_id: B256,
        token0_decimals: u8,
        token1_decimals: u8,
        weight0: u128,
        weight1: u128,
        base_is_token1: bool,
    ) -> Result<(f64, u64), PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        let pool_id_bytes: [u8; 32] = pool_id.into();
        let (_tokens, balances) =
            get_balancer_pool_tokens(&self.simulator, pool_id_bytes, Some(latest_block))
                .await
                .map_err(|e| PriceError::DatabaseError(e.to_string()))?
                .ok_or_else(|| {
                    PriceError::PriceNotAvailable("Balancer pool balances unavailable".into())
                })?;

        if balances.len() < 2 {
            return Err(PriceError::PriceNotAvailable(
                "Balancer pool does not have two balances".into(),
            ));
        }

        let token0_balance = balances[0].min(U256::from(u128::MAX)).to::<u128>() as f64
            / 10f64.powi(token0_decimals as i32);
        let token1_balance = balances[1].min(U256::from(u128::MAX)).to::<u128>() as f64
            / 10f64.powi(token1_decimals as i32);

        let weight0_f = weight0 as f64 / 1e18;
        let weight1_f = weight1 as f64 / 1e18;

        let price = if base_is_token1 {
            if token1_balance > 0.0 && weight1_f > 0.0 {
                (token0_balance / weight0_f) / (token1_balance / weight1_f)
            } else {
                0.0
            }
        } else if token0_balance > 0.0 && weight0_f > 0.0 {
            (token1_balance / weight1_f) / (token0_balance / weight0_f)
        } else {
            0.0
        };

        Ok((price, latest_block))
    }
}
