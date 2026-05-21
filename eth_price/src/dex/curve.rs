//! Curve Finance Price Reader.
//!
//! Handles reading state from various types of Curve pools (StableSwap, TriCrypto, etc.).
//!
//! # Implementation Details
//!
//! *   **Hybrid Access:** Uses `TxSimulator` to call view functions like `balances()` and `price_scale()` because Curve's storage layout varies by pool version and compiler.
//! *   **Pool Discovery:** Can dynamically find pools using `find_curve_pool_for_coins`.
//! *   **TriCrypto Support:** Specifically handles V2 crypto pools by reading the internal `price_scale` for oracle-like price feeds.
//! *   **Math:** Implements specific logic for stable vs. crypto pool price derivation.

// Storage-based + view-call hybrid for Curve pools (copied from state_based/curve.rs)
use crate::core::{
    AmmPriceSource, LiquidityMetrics, Pool, PoolKind, PoolState, PriceData, PriceError, PricePair,
    PriceSource, Protocol, RawChainPrice, Token,
};
use crate::shared_simulator::get_or_create_simulator_with_provider;
use alloy_primitives::{Address, Bytes, U256};
use reth_chain_query::dex::find_curve_pool_for_coins;
use reth_provider::{BlockNumReader, BlockReader};
use std::collections::HashMap;
use std::sync::Arc;
use tx_simulator::TxSimulator;

#[derive(Clone, Debug)]
struct TokenInfo {
    token: Token,
    index: usize,
}

#[derive(Clone, Debug)]
struct PoolInfo {
    address: Address,
    pool_type: CurvePoolType,
    tokens: Vec<TokenInfo>,
    base_index: usize,
    quote_index: usize,
    price_scale_index: Option<u8>,
}

async fn fetch_pool_balances(
    simulator: Arc<TxSimulator>,
    pool: Address,
    tokens: &[TokenInfo],
    block_number: u64,
) -> Result<Vec<U256>, PriceError> {
    let mut balances = Vec::with_capacity(tokens.len());
    for token_info in tokens {
        let mut call = Vec::with_capacity(36);
        call.extend_from_slice(&[0x49, 0x03, 0xb0, 0xd1]); // balances(uint256)
        let mut index_bytes = [0u8; 32];
        index_bytes[31] = token_info.index as u8;
        call.extend_from_slice(&index_bytes);
        let result = simulator
            .simulate_view_function(pool, Bytes::from(call), Some(block_number))
            .await
            .map_err(|e| PriceError::DatabaseError(format!("Curve balances() call failed: {e}")))?;

        if !result.success || result.output.len() < 32 {
            return Err(PriceError::PriceNotAvailable(
                "Curve balances() returned no data".into(),
            ));
        }

        balances.push(U256::from_be_slice(&result.output[0..32]));
    }
    Ok(balances)
}

async fn fetch_price_scale(
    simulator: Arc<TxSimulator>,
    pool: Address,
    index: u8,
    block_number: u64,
) -> Result<U256, PriceError> {
    let mut call = Vec::with_capacity(36);
    call.extend_from_slice(&[0xa3, 0xf7, 0xcd, 0xd5]); // price_scale(uint256)
    let mut index_bytes = [0u8; 32];
    index_bytes[31] = index;
    call.extend_from_slice(&index_bytes);
    let result = simulator
        .simulate_view_function(pool, Bytes::from(call), Some(block_number))
        .await
        .map_err(|e| PriceError::DatabaseError(format!("Curve price_scale call failed: {e}")))?;

    if !result.success || result.output.len() < 32 {
        return Err(PriceError::PriceNotAvailable(
            "Curve price_scale returned no data".into(),
        ));
    }

    Ok(U256::from_be_slice(&result.output[0..32]))
}

fn price_from_scale(scale: U256, base_decimals: u8, quote_decimals: u8) -> RawChainPrice {
    let quote_scale = pow10_u256(quote_decimals as u32);
    let base_scale = pow10_u256(base_decimals as u32);
    let numerator = safe_mul_u256(scale, quote_scale);
    let denominator = safe_mul_u256(pow10_u256(18), base_scale);
    RawChainPrice::new(numerator, denominator)
}

fn pow10_u256(exp: u32) -> U256 {
    U256::from(10u8).pow(U256::from(exp))
}

fn safe_mul_u256(a: U256, b: U256) -> U256 {
    a.checked_mul(b).unwrap_or_else(|| a.saturating_mul(b))
}
#[derive(Clone, Debug)]
enum CurvePoolType {
    StableSwap3Pool,
    StableSwap2Pool,
    TriCrypto2,
    CurveTricrypto,
}

#[derive(Clone)]
pub struct CurveReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    pool_addresses: HashMap<String, PoolInfo>,
    simulator: Arc<TxSimulator>,
}

impl CurveReader {
    pub fn new(db_path_str: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path_str)?)
    }

    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        let simulator = get_or_create_simulator_with_provider(provider_factory.clone())?;
        let mut pool_addresses = HashMap::new();

        for info in reth_chain_query::common_addresses::dex_token_denom_pairs::curve_pools() {
            let mut tokens = Vec::new();
            for t in &info.tokens {
                tokens.push(TokenInfo {
                    token: Token::new(t.token_address, t.symbol.to_string(), t.decimals),
                    index: t.index,
                });
            }

            let pool_type = match info.name {
                "3pool" => CurvePoolType::StableSwap3Pool,
                "stETH-ETH" => CurvePoolType::StableSwap2Pool,
                "TriCrypto2" => CurvePoolType::TriCrypto2,
                "USDD/ETH_CURVE_V2" => CurvePoolType::CurveTricrypto, // Explicitly match USDD/ETH pool
                _ => CurvePoolType::StableSwap2Pool,                  // Default or error?
            };

            let price_scale_index = if info.name == "TriCrypto2" {
                // For TriCrypto2: 0: USDT, 1: WBTC, 2: WETH.
                // price_scale(0) -> WBTC/USDT, price_scale(1) -> WETH/USDT.
                // We want WETH/USDT price, so we use index 1.
                Some(1)
            } else {
                None
            };

            pool_addresses.insert(
                // Map specific known names to pair labels expected by client
                match info.name {
                    "TriCrypto2" => "ETH/USDT".to_string(), // Map to "ETH/USDT" directly for simplicity
                    name => name.to_string(),
                },
                PoolInfo {
                    address: info.pool_address,
                    pool_type,
                    tokens,
                    base_index: info.base_token_index,
                    quote_index: info.quote_token_index,
                    price_scale_index,
                },
            );
        }

        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            pool_addresses,
            simulator,
        })
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
        let pool_info = self
            .pool_addresses
            .get(pair)
            .ok_or_else(|| PriceError::PriceNotAvailable(pair.to_string()))?;

        let pool_addr = self
            .resolve_pool_address(pair, pool_info, Some(block_number))
            .await?;

        let balances = fetch_pool_balances(
            self.simulator.clone(),
            pool_addr,
            &pool_info.tokens,
            block_number,
        )
        .await?;

        let base_info = pool_info
            .tokens
            .get(pool_info.base_index)
            .ok_or_else(|| PriceError::ConversionError("Invalid base index".into()))?;
        let quote_info = pool_info
            .tokens
            .get(pool_info.quote_index)
            .ok_or_else(|| PriceError::ConversionError("Invalid quote index".into()))?;

        let base_balance = balances
            .get(pool_info.base_index)
            .copied()
            .unwrap_or(U256::ZERO);
        let quote_balance = balances
            .get(pool_info.quote_index)
            .copied()
            .unwrap_or(U256::ZERO);

        if base_balance.is_zero() || quote_balance.is_zero() {
            return Err(PriceError::PriceNotAvailable(format!(
                "Curve pool {} has zero balance for tracked tokens",
                pair
            )));
        }

        let precise_price = if let Some(scale_index) = pool_info.price_scale_index {
            let raw_scale =
                fetch_price_scale(self.simulator.clone(), pool_addr, scale_index, block_number)
                    .await?;
            price_from_scale(
                raw_scale,
                base_info.token.decimals,
                quote_info.token.decimals,
            )
        } else {
            RawChainPrice::from_reserves(quote_balance, base_balance)
        };

        let block = self
            .provider_factory
            .block_by_number(block_number)
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            .ok_or(PriceError::InvalidBlockNumber(block_number))?;

        let pool_kind = PoolKind::Curve {
            balances: balances.clone(),
            amplification: U256::ZERO,
            fee_bps: 0,
            admin_fee_bps: 0,
        };

        let pool = Pool::new(
            pool_addr,
            Protocol::Curve,
            base_info.token.clone(),
            quote_info.token.clone(),
            pool_kind,
        );

        let pool_state = PoolState::new(pool.clone(), block_number, block.timestamp);
        let liquidity = LiquidityMetrics::from_pool(&pool);
        let price_pair = PricePair::new(
            pair.to_string(),
            base_info.token.clone(),
            quote_info.token.clone(),
        );
        let price_id = pool.create_price_id(base_info.token.address, quote_info.token.address);

        let source = PriceSource::Amm(AmmPriceSource {
            protocol: Protocol::Curve,
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

    pub fn supported_pairs(&self) -> Vec<String> {
        self.pool_addresses.keys().cloned().collect()
    }

    pub async fn get_eth_usdt_tricrypto2_latest_by_pool(
        &self,
        pool: Address,
        weth_index: u8,
    ) -> Result<(f64, u64), PriceError> {
        let latest_block = self
            .provider_factory
            .last_block_number()
            .map_err(|e| PriceError::DatabaseError(e.to_string()))?;
        let price = self
            .get_eth_usdt_tricrypto2_price_by_pool_at_block(pool, weth_index, latest_block)
            .await?;
        Ok((price, latest_block))
    }

    pub async fn get_eth_usdt_tricrypto2_price_by_pool_at_block(
        &self,
        pool: Address,
        weth_index: u8,
        block_number: u64,
    ) -> Result<f64, PriceError> {
        let _ = (pool, weth_index); // parameters retained for API compatibility
        let data = self.get_price_at_block("ETH/USDT", block_number).await?;
        Ok(data.price_as_f64())
    }

    async fn resolve_pool_address(
        &self,
        _pair: &str,
        info: &PoolInfo,
        block: Option<u64>,
    ) -> Result<Address, PriceError> {
        if info.address != Address::ZERO {
            return Ok(info.address);
        }

        let resolved = match info.pool_type {
            CurvePoolType::StableSwap3Pool => {
                if info.tokens.len() < 2 {
                    return Ok(Address::ZERO);
                }
                let token_a = info.tokens[0].token.address;
                let token_b = info.tokens[1].token.address;
                find_curve_pool_for_coins(&self.simulator, token_a, token_b, block)
                    .await
                    .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            }
            CurvePoolType::StableSwap2Pool => {
                if info.tokens.len() < 2 {
                    return Ok(Address::ZERO);
                }
                let token_a = info.tokens[0].token.address;
                let token_b = info.tokens[1].token.address;
                find_curve_pool_for_coins(&self.simulator, token_a, token_b, block)
                    .await
                    .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            }
            CurvePoolType::TriCrypto2 => {
                let usdt = info
                    .tokens
                    .iter()
                    .find(|t| t.token.symbol == "USDT")
                    .map(|t| t.token.address)
                    .ok_or_else(|| {
                        PriceError::ConversionError("USDT address not configured".into())
                    })?;
                let weth = info
                    .tokens
                    .iter()
                    .find(|t| t.token.symbol == "WETH")
                    .map(|t| t.token.address)
                    .ok_or_else(|| {
                        PriceError::ConversionError("WETH address not configured".into())
                    })?;
                find_curve_pool_for_coins(&self.simulator, usdt, weth, block)
                    .await
                    .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            }
            CurvePoolType::CurveTricrypto => {
                let usdd = info
                    .tokens
                    .iter()
                    .find(|t| t.token.symbol == "USDD")
                    .map(|t| t.token.address)
                    .ok_or_else(|| {
                        PriceError::ConversionError("USDD address not configured".into())
                    })?;
                let weth = info
                    .tokens
                    .iter()
                    .find(|t| t.token.symbol == "WETH")
                    .map(|t| t.token.address)
                    .ok_or_else(|| {
                        PriceError::ConversionError("WETH address not configured".into())
                    })?;
                find_curve_pool_for_coins(&self.simulator, usdd, weth, block)
                    .await
                    .map_err(|e| PriceError::DatabaseError(e.to_string()))?
            }
        };
        Ok(resolved.unwrap_or(Address::ZERO))
    }
}
