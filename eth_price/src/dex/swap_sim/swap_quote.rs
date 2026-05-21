use crate::core::{
    PriceData, PriceError, PriceId, PricePair, PriceSource, Protocol, RawChainPrice,
    SimulationDirection, SimulationPriceSource, Token,
};
use crate::shared_simulator::get_or_create_simulator_with_provider;
use alloy_primitives::{Address, Bytes, U256};
use reth_chain_query::common_addresses::{
    get_address_by_name, get_token_decimals as lookup_token_decimals,
    get_token_symbol as lookup_token_symbol,
};
use std::sync::Arc;
use tx_processor::trade_simulation::{
    check_can_buy_sell_pool, PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::contract_simulation::decode_string_from_contract_output;
use tx_simulator::tx_builders::amm_swap_route::AmmSwapRoute;
use tx_simulator::TxSimulator;

/// Bid/ask quote derived from a full buy+sell simulation.
#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub ask: PriceData,
    pub bid: Option<PriceData>,
    pub tokens_acquired: U256,
    pub eth_spent: U256,
    pub eth_received: Option<U256>,
    pub buy_tax_percent: Option<f64>,
    pub sell_tax_percent: Option<f64>,
    pub block_number: u64,
}

/// Swap simulation reader backed by `pool_buy_sell_simulator`.
pub struct BuySwapReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
}

impl BuySwapReader {
    /// Initialize with Reth datadir (e.g., ~/.local/share/reth/mainnet)
    pub fn new(db_path: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path)?)
    }

    /// Initialize using existing provider factory
    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        let simulator = get_or_create_simulator_with_provider(provider_factory.clone())?;
        let tx_processor = Arc::new(TxProcessor::new());
        Ok(Self {
            provider_factory: (*provider_factory).clone(),
            simulator,
            tx_processor,
        })
    }

    /// Execute a buy+sell simulation and return the full bid/ask quote.
    pub async fn quote_eth_for_token(
        &self,
        route: AmmSwapRoute,
        token_out: Address,
        eth_in_wei: U256,
        buyer: Address,
        block_number: Option<u64>,
    ) -> Result<SwapQuote, PriceError> {
        if eth_in_wei.is_zero() {
            return Err(PriceError::PriceNotAvailable(
                "Simulation requires non-zero ETH input".into(),
            ));
        }

        let (pool_type, pool_address, protocol, fee_tier) = map_route(&route)?;

        let weth_address = get_address_by_name("WETH")
            .ok_or_else(|| PriceError::ConversionError("Missing WETH address".into()))?;

        let simulator = self.simulator.clone();
        let (quote_symbol, quote_decimals) =
            resolve_token_metadata(simulator.clone(), token_out, block_number).await?;

        let mut params = PoolBuySellParameters::new(token_out, pool_address, pool_type)
            .with_test_amount(eth_in_wei)
            .with_buyer(buyer)
            .with_token_decimals(quote_decimals)
            .with_denom_address(weth_address)
            .with_denom_decimals(18);
        if let Some(block) = block_number {
            params = params.with_block(block);
        }

        let result = check_can_buy_sell_pool(simulator, self.tx_processor.clone(), params)
            .await
            .map_err(|e| PriceError::PriceNotAvailable(format!("Simulation error: {e}")))?;

        if !result.can_buy {
            return Err(PriceError::PriceNotAvailable(
                result
                    .failure_reason
                    .unwrap_or_else(|| "Buy leg failed".to_string()),
            ));
        }

        let base_symbol = lookup_token_symbol(weth_address)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "WETH".to_string());
        let base_token = Token::new(weth_address, base_symbol, 18);
        let quote_token = Token::new(token_out, quote_symbol.clone(), quote_decimals);

        let ask = build_price_data(
            &route,
            &result,
            buyer,
            &base_token,
            &quote_token,
            protocol,
            fee_tier,
            TradeSide::Buy,
        )?;
        let bid = if result.can_sell && result.denom_received > U256::ZERO {
            Some(build_price_data(
                &route,
                &result,
                buyer,
                &base_token,
                &quote_token,
                protocol,
                fee_tier,
                TradeSide::Sell,
            )?)
        } else {
            None
        };

        Ok(SwapQuote {
            ask,
            bid,
            tokens_acquired: result.tokens_received,
            eth_spent: result.denom_spent,
            eth_received: if result.can_sell {
                Some(result.denom_received)
            } else {
                None
            },
            buy_tax_percent: normalize_tax(result.buy_tax_percent),
            sell_tax_percent: normalize_tax(result.sell_tax_percent),
            block_number: result.block_number,
        })
    }

    /// Backwards-compatible helper that returns only the ask leg.
    pub async fn price_eth_for_token(
        &self,
        route: AmmSwapRoute,
        token_out: Address,
        eth_in_wei: U256,
        buyer: Address,
        block_number: Option<u64>,
    ) -> Result<PriceData, PriceError> {
        self.quote_eth_for_token(route, token_out, eth_in_wei, buyer, block_number)
            .await
            .map(|quote| quote.ask)
    }
}

#[derive(Clone, Copy)]
enum TradeSide {
    Buy,
    Sell,
}

fn build_price_data(
    route: &AmmSwapRoute,
    result: &PoolBuySellSimulationResult,
    buyer: Address,
    base_token: &Token,
    quote_token: &Token,
    protocol: Protocol,
    fee_tier: Option<u32>,
    side: TradeSide,
) -> Result<PriceData, PriceError> {
    let (price_pair, price_ratio, source) = match side {
        TradeSide::Buy => {
            if result.denom_spent.is_zero() || result.tokens_received.is_zero() {
                return Err(PriceError::PriceNotAvailable(
                    "Buy simulation returned zero amounts".to_string(),
                ));
            }
            let pair_label = format!("{}/{}_SWAP_ASK", base_token.symbol, quote_token.symbol);
            let pair = PricePair::new(pair_label, base_token.clone(), quote_token.clone());
            let ratio = RawChainPrice::new(result.tokens_received, result.denom_spent);
            let source = PriceSource::Simulation(SimulationPriceSource {
                price_id: PriceId::new(
                    base_token.address,
                    quote_token.address,
                    protocol,
                    result.pool_address,
                    fee_tier,
                ),
                direction: SimulationDirection::Buy,
                route: format!("{:?}", route),
                account: buyer,
                input_token: base_token.clone(),
                output_token: quote_token.clone(),
                input_amount: result.denom_spent,
                output_amount: result.tokens_received,
                tax_percent: normalize_tax(result.buy_tax_percent),
                liquidity: None,
            });
            (pair, ratio, source)
        }
        TradeSide::Sell => {
            if result.tokens_received.is_zero() || result.denom_received.is_zero() {
                return Err(PriceError::PriceNotAvailable(
                    "Sell simulation returned zero amounts".to_string(),
                ));
            }
            let pair_label = format!("{}/{}_SWAP_BID", quote_token.symbol, base_token.symbol);
            let pair = PricePair::new(pair_label, quote_token.clone(), base_token.clone());
            let ratio = RawChainPrice::new(result.denom_received, result.tokens_received);
            let source = PriceSource::Simulation(SimulationPriceSource {
                price_id: PriceId::new(
                    quote_token.address,
                    base_token.address,
                    protocol,
                    result.pool_address,
                    fee_tier,
                ),
                direction: SimulationDirection::Sell,
                route: format!("{:?}", route),
                account: buyer,
                input_token: quote_token.clone(),
                output_token: base_token.clone(),
                input_amount: result.tokens_received,
                output_amount: result.denom_received,
                tax_percent: normalize_tax(result.sell_tax_percent),
                liquidity: None,
            });
            (pair, ratio, source)
        }
    };

    let (block_number, block_timestamp) = match side {
        TradeSide::Buy => (
            result.buy_transaction.block_number,
            result.buy_transaction.block_timestamp,
        ),
        TradeSide::Sell => (
            result.sell_transaction.block_number,
            result.sell_transaction.block_timestamp,
        ),
    };

    Ok(PriceData::new(
        price_pair,
        price_ratio,
        block_number,
        block_timestamp,
        source,
    ))
}

fn map_route(
    route: &AmmSwapRoute,
) -> Result<(PoolType, Address, Protocol, Option<u32>), PriceError> {
    match *route {
        AmmSwapRoute::UniswapV2 { pool } => {
            Ok((PoolType::UniswapV2, pool, Protocol::UniswapV2, None))
        }
        AmmSwapRoute::SushiswapV2 { pool } => {
            Ok((PoolType::SushiSwap, pool, Protocol::SushiSwap, None))
        }
        AmmSwapRoute::UniswapV3 { pool, fee_tier } => Ok((
            PoolType::UniswapV3 { fee_tier },
            pool,
            Protocol::UniswapV3,
            Some(fee_tier),
        )),
        _ => Err(PriceError::SourceNotSupported(format!(
            "Route {:?} not supported in swap simulator",
            route
        ))),
    }
}

fn normalize_tax(value: f64) -> Option<f64> {
    if value >= 0.0 {
        Some(value)
    } else {
        None
    }
}

async fn resolve_token_metadata(
    simulator: Arc<TxSimulator>,
    token: Address,
    block_number: Option<u64>,
) -> Result<(String, u8), PriceError> {
    let mut symbol = lookup_token_symbol(token).map(|s| s.to_string());
    let mut decimals = symbol.as_deref().and_then(lookup_token_decimals);

    if decimals.is_none() {
        if let Ok(value) = call_token_decimals(simulator.clone(), token, block_number).await {
            if value > 0 {
                decimals = Some(value);
            }
        }
    }

    if symbol.is_none() {
        if let Ok(value) = call_token_symbol(simulator.clone(), token, block_number).await {
            if !value.is_empty() {
                symbol = Some(value);
            }
        }
    }

    let symbol = symbol.unwrap_or_else(|| format!("0x{:x}", token));
    let decimals = decimals.unwrap_or(18);

    Ok((symbol, decimals))
}

async fn call_token_decimals(
    simulator: Arc<TxSimulator>,
    token: Address,
    block_number: Option<u64>,
) -> Result<u8, PriceError> {
    let selector = Bytes::from(vec![0x31, 0x3c, 0xe5, 0x67]);
    let result = simulator
        .simulate_view_function(token, selector, block_number)
        .await
        .map_err(|e| PriceError::DatabaseError(format!("decimals() call failed: {e}")))?;

    if result.success && result.output.len() >= 32 {
        Ok(result.output[31])
    } else {
        Err(PriceError::PriceNotAvailable(format!(
            "Token decimals unavailable for {token:#x}"
        )))
    }
}

async fn call_token_symbol(
    simulator: Arc<TxSimulator>,
    token: Address,
    block_number: Option<u64>,
) -> Result<String, PriceError> {
    let selector = Bytes::from(vec![0x95, 0xd8, 0x9b, 0x41]);
    let result = simulator
        .simulate_view_function(token, selector, block_number)
        .await
        .map_err(|e| PriceError::DatabaseError(format!("symbol() call failed: {e}")))?;

    if result.success {
        Ok(decode_string_from_contract_output(&result.output))
    } else {
        Err(PriceError::PriceNotAvailable(format!(
            "Token symbol unavailable for {token:#x}"
        )))
    }
}
