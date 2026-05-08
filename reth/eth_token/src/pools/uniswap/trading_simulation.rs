use std::sync::Arc;

use alloy_primitives::{Address, U256};
use eyre::{eyre, Result};
use reth_chain_query::provider::BlockHeader;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{
    LivePoolBuySellSimulator, PoolBuySellParameters, PoolBuySellSimulationResult,
    PoolBuySellSimulator, PoolType, ProcessedTransaction, TxSimulator,
};

use crate::pools::base::DEFAULT_TEST_BUY_ETH;

use super::v2::{UniswapV2Pool, UniswapV2TxContext};
use super::v3::UniswapV3Pool;
use super::v4::UniswapV4Pool;

#[derive(Clone, Debug, Default)]
pub struct UniswapV2TradingSimulationConfig {
    pub test_amount: Option<U256>,
    pub buyer_address: Option<Address>,
    pub prior_txs: Vec<ProcessedTransaction>,
    pub block_number: Option<u64>,
    pub block_header: Option<BlockHeader>,
    pub slippage_tolerance: Option<f64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub buy_gas_limit: Option<u64>,
    pub approve_gas_limit: Option<u64>,
    pub sell_gas_limit: Option<u64>,
    pub block_delay: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UniswapV2TradingSimulationOutcome {
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub is_tradeable: bool,
    pub buy_tax_percent: f64,
    pub sell_tax_percent: f64,
    pub failure_reason: Option<String>,
    pub simulation_block_number: u64,
}

pub type PoolTradingSimulationConfig = UniswapV2TradingSimulationConfig;
pub type PoolTradingSimulationOutcome = UniswapV2TradingSimulationOutcome;

impl From<&PoolBuySellSimulationResult> for UniswapV2TradingSimulationOutcome {
    fn from(result: &PoolBuySellSimulationResult) -> Self {
        Self {
            can_buy: result.can_buy,
            can_approve: result.can_approve,
            can_sell: result.can_sell,
            is_tradeable: result.is_tradeable,
            buy_tax_percent: result.buy_tax_percent,
            sell_tax_percent: result.sell_tax_percent,
            failure_reason: result.failure_reason.clone(),
            simulation_block_number: result.block_number,
        }
    }
}

impl UniswapV2Pool {
    pub fn build_buy_sell_parameters(
        &self,
        config: &UniswapV2TradingSimulationConfig,
    ) -> Result<PoolBuySellParameters> {
        let token_address = parse_address(&self.base.identity.token_address)?;
        let pool_address = parse_address(&self.base.identity.pool_address)?;
        let denom_address = parse_address(&self.base.identity.denom_address)?;
        let denom_decimals = self
            .base
            .config
            .denom_decimals
            .unwrap_or(self.base.config.token_decimals);
        let denom_amount = if self.base.config.test_buy_amount_eth > 0.0 {
            self.base.config.test_buy_amount_eth
        } else {
            DEFAULT_TEST_BUY_ETH
        };
        let test_amount = config
            .test_amount
            .unwrap_or(scaled_decimal_amount(denom_amount, denom_decimals)?);

        let mut params =
            PoolBuySellParameters::new(token_address, pool_address, PoolType::UniswapV2)
                .with_test_amount(test_amount)
                .with_denom_address(denom_address)
                .with_denom_decimals(denom_decimals)
                .with_token_decimals(self.base.config.token_decimals)
                .with_prior_transactions(config.prior_txs.clone());

        if let Some(block_number) = config.block_number {
            params = params.with_block(block_number);
        }
        if let Some(block_header) = config.block_header.clone() {
            params = params.with_block_header(block_header);
        }
        if let Some(buyer_address) = config.buyer_address {
            params = params.with_buyer(buyer_address);
        }
        if let Some(block_delay) = config.block_delay {
            params = params.with_block_delay(block_delay);
        }
        if let Some(gas_price) = config.gas_price {
            params = params.with_legacy_gas_price(gas_price);
        }
        if let Some(max_fee_per_gas) = config.max_fee_per_gas {
            params = params.with_max_fee_per_gas(max_fee_per_gas);
        }
        if let Some(max_priority_fee_per_gas) = config.max_priority_fee_per_gas {
            params = params.with_max_priority_fee_per_gas(max_priority_fee_per_gas);
        }
        if let Some(slippage_tolerance) = config.slippage_tolerance {
            params.slippage_tolerance = slippage_tolerance;
        }
        if let Some(buy_gas_limit) = config.buy_gas_limit {
            params.buy_gas_limit = buy_gas_limit;
        }
        if let Some(approve_gas_limit) = config.approve_gas_limit {
            params.approve_gas_limit = approve_gas_limit;
        }
        if let Some(sell_gas_limit) = config.sell_gas_limit {
            params.sell_gas_limit = sell_gas_limit;
        }

        Ok(params)
    }

    pub async fn evaluate_trading_status_v2(
        &mut self,
        simulator: Arc<TxSimulator>,
        tx_processor: Arc<TxProcessor>,
        tx: &UniswapV2TxContext,
        config: UniswapV2TradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        let pool_simulator = PoolBuySellSimulator::new(simulator, tx_processor);
        self.evaluate_trading_status_v2_with_pool_simulator(&pool_simulator, tx, config)
            .await
    }

    pub async fn evaluate_trading_status_v2_with_pool_simulator(
        &mut self,
        pool_simulator: &PoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        mut config: UniswapV2TradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        if config.block_number.is_none() {
            config.block_number = Some(tx.block_number.saturating_sub(1));
        }

        let params = self.build_buy_sell_parameters(&config)?;
        let result = pool_simulator.check_pool(params).await?;
        self.apply_trading_simulation_outcome(
            tx,
            &UniswapV2TradingSimulationOutcome::from(&result),
        );
        Ok(result)
    }

    pub async fn evaluate_live_trading_status_v2(
        &mut self,
        pool_simulator: &LivePoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        config: UniswapV2TradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        let params = self.build_buy_sell_parameters(&config)?;
        let result = if let Some(block_number) = config.block_number {
            pool_simulator
                .check_pool_at_block(params, block_number)
                .await?
        } else {
            pool_simulator
                .check_pool_before_live_block(params, tx.block_number)
                .await?
        };
        self.apply_trading_simulation_outcome(
            tx,
            &UniswapV2TradingSimulationOutcome::from(&result),
        );
        Ok(result)
    }

    pub fn apply_trading_simulation_outcome(
        &mut self,
        tx: &UniswapV2TxContext,
        outcome: &UniswapV2TradingSimulationOutcome,
    ) {
        if outcome.can_buy && !self.base.state.can_buy {
            self.base.mark_can_buy_from_event(
                tx.block_number,
                tx.tx_hash.clone(),
                tx.block_timestamp,
            );
        }

        self.base.set_sell_status(
            outcome.can_sell,
            Some(outcome.buy_tax_percent),
            Some(outcome.sell_tax_percent),
            tx.block_number,
            tx.tx_hash.clone(),
        );
    }
}

impl UniswapV3Pool {
    pub fn build_buy_sell_parameters(
        &self,
        config: &PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellParameters> {
        let token_address = parse_address(&self.base.identity.token_address)?;
        let pool_address = parse_address(&self.base.identity.pool_address)?;
        let denom_address = parse_address(&self.base.identity.denom_address)?;
        let denom_decimals = self
            .base
            .config
            .denom_decimals
            .unwrap_or(self.base.config.token_decimals);
        let denom_amount = if self.base.config.test_buy_amount_eth > 0.0 {
            self.base.config.test_buy_amount_eth
        } else {
            DEFAULT_TEST_BUY_ETH
        };
        let test_amount = config
            .test_amount
            .unwrap_or(scaled_decimal_amount(denom_amount, denom_decimals)?);

        let mut params = PoolBuySellParameters::new(
            token_address,
            pool_address,
            PoolType::UniswapV3 {
                fee_tier: self.fee_tier,
            },
        )
        .with_test_amount(test_amount)
        .with_denom_address(denom_address)
        .with_denom_decimals(denom_decimals)
        .with_token_decimals(self.base.config.token_decimals)
        .with_prior_transactions(config.prior_txs.clone());

        apply_common_parameter_config(&mut params, config);
        Ok(params)
    }

    pub async fn evaluate_trading_status_v3_with_pool_simulator(
        &mut self,
        pool_simulator: &PoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        mut config: PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        if config.block_number.is_none() {
            config.block_number = Some(tx.block_number.saturating_sub(1));
        }

        let params = self.build_buy_sell_parameters(&config)?;
        let result = pool_simulator.check_pool(params).await?;
        self.apply_trading_simulation_outcome(tx, &PoolTradingSimulationOutcome::from(&result));
        Ok(result)
    }

    pub async fn evaluate_live_trading_status_v3(
        &mut self,
        pool_simulator: &LivePoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        config: PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        let params = self.build_buy_sell_parameters(&config)?;
        let result = if let Some(block_number) = config.block_number {
            pool_simulator
                .check_pool_at_block(params, block_number)
                .await?
        } else {
            pool_simulator
                .check_pool_before_live_block(params, tx.block_number)
                .await?
        };
        self.apply_trading_simulation_outcome(tx, &PoolTradingSimulationOutcome::from(&result));
        Ok(result)
    }

    pub fn apply_trading_simulation_outcome(
        &mut self,
        tx: &UniswapV2TxContext,
        outcome: &PoolTradingSimulationOutcome,
    ) {
        if outcome.can_buy && !self.base.state.can_buy {
            self.base.mark_can_buy_from_event(
                tx.block_number,
                tx.tx_hash.clone(),
                tx.block_timestamp,
            );
        }

        self.base.set_sell_status(
            outcome.can_sell,
            Some(outcome.buy_tax_percent),
            Some(outcome.sell_tax_percent),
            tx.block_number,
            tx.tx_hash.clone(),
        );
    }
}

impl UniswapV4Pool {
    pub fn build_buy_sell_parameters(
        &self,
        config: &PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellParameters> {
        let token_address = parse_address(&self.base.identity.token_address)?;
        let pool_manager = parse_address(&self.pool_manager_address)?;
        let denom_address = parse_address(&self.base.identity.denom_address)?;
        let denom_decimals = self
            .base
            .config
            .denom_decimals
            .unwrap_or(self.base.config.token_decimals);
        let denom_amount = if self.base.config.test_buy_amount_eth > 0.0 {
            self.base.config.test_buy_amount_eth
        } else {
            DEFAULT_TEST_BUY_ETH
        };
        let test_amount = config
            .test_amount
            .unwrap_or(scaled_decimal_amount(denom_amount, denom_decimals)?);

        let mut params =
            PoolBuySellParameters::new(token_address, pool_manager, PoolType::UniswapV4)
                .with_test_amount(test_amount)
                .with_denom_address(denom_address)
                .with_denom_decimals(denom_decimals)
                .with_token_decimals(self.base.config.token_decimals)
                .with_uniswap_v4_config(self.build_simulator_pool_config()?)
                .with_prior_transactions(config.prior_txs.clone());

        apply_common_parameter_config(&mut params, config);
        Ok(params)
    }

    pub async fn evaluate_trading_status_v4_with_pool_simulator(
        &mut self,
        pool_simulator: &PoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        mut config: PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        if config.block_number.is_none() {
            config.block_number = Some(tx.block_number.saturating_sub(1));
        }

        let params = self.build_buy_sell_parameters(&config)?;
        let result = pool_simulator.check_pool(params).await?;
        self.apply_trading_simulation_outcome(tx, &PoolTradingSimulationOutcome::from(&result));
        Ok(result)
    }

    pub async fn evaluate_live_trading_status_v4(
        &mut self,
        pool_simulator: &LivePoolBuySellSimulator,
        tx: &UniswapV2TxContext,
        config: PoolTradingSimulationConfig,
    ) -> Result<PoolBuySellSimulationResult> {
        let params = self.build_buy_sell_parameters(&config)?;
        let result = if let Some(block_number) = config.block_number {
            pool_simulator
                .check_pool_at_block(params, block_number)
                .await?
        } else {
            pool_simulator
                .check_pool_before_live_block(params, tx.block_number)
                .await?
        };
        self.apply_trading_simulation_outcome(tx, &PoolTradingSimulationOutcome::from(&result));
        Ok(result)
    }

    pub fn apply_trading_simulation_outcome(
        &mut self,
        tx: &UniswapV2TxContext,
        outcome: &PoolTradingSimulationOutcome,
    ) {
        if outcome.can_buy && !self.base.state.can_buy {
            self.base.mark_can_buy_from_event(
                tx.block_number,
                tx.tx_hash.clone(),
                tx.block_timestamp,
            );
        }

        self.base.set_sell_status(
            outcome.can_sell,
            Some(outcome.buy_tax_percent),
            Some(outcome.sell_tax_percent),
            tx.block_number,
            tx.tx_hash.clone(),
        );
    }
}

fn parse_address(value: &str) -> Result<Address> {
    value
        .parse()
        .map_err(|err| eyre!("invalid address {value}: {err}"))
}

fn apply_common_parameter_config(
    params: &mut PoolBuySellParameters,
    config: &PoolTradingSimulationConfig,
) {
    if let Some(block_number) = config.block_number {
        params.block_number = Some(block_number);
    }
    if let Some(block_header) = config.block_header.clone() {
        params.block_header = Some(block_header);
    }
    if let Some(buyer_address) = config.buyer_address {
        params.buyer_address = buyer_address;
    }
    if let Some(block_delay) = config.block_delay {
        params.block_delay = block_delay;
    }
    if let Some(gas_price) = config.gas_price {
        params.gas_price = Some(gas_price);
    }
    if let Some(max_fee_per_gas) = config.max_fee_per_gas {
        params.max_fee_per_gas = Some(max_fee_per_gas);
    }
    if let Some(max_priority_fee_per_gas) = config.max_priority_fee_per_gas {
        params.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
    }
    if let Some(slippage_tolerance) = config.slippage_tolerance {
        params.slippage_tolerance = slippage_tolerance;
    }
    if let Some(buy_gas_limit) = config.buy_gas_limit {
        params.buy_gas_limit = buy_gas_limit;
    }
    if let Some(approve_gas_limit) = config.approve_gas_limit {
        params.approve_gas_limit = approve_gas_limit;
    }
    if let Some(sell_gas_limit) = config.sell_gas_limit {
        params.sell_gas_limit = sell_gas_limit;
    }
}

fn scaled_decimal_amount(amount: f64, decimals: u8) -> Result<U256> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(eyre!("invalid decimal amount {amount}"));
    }
    let scale = 10_f64.powi(i32::from(decimals));
    let raw = (amount * scale).round();
    if raw > u128::MAX as f64 {
        return Err(eyre!("decimal amount {amount} overflows u128 scaling"));
    }
    Ok(U256::from(raw as u128))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pools::base::BasePoolConfig;
    use crate::pools::uniswap::v2::UNISWAP_V2_PROTOCOL;

    fn pool() -> UniswapV2Pool {
        UniswapV2Pool::new(
            "0x0000000000000000000000000000000000000002",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000003",
            BasePoolConfig {
                token_decimals: 9,
                denom_decimals: Some(6),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: 1.25,
            },
            std::iter::empty::<&str>(),
        )
    }

    #[test]
    fn builds_v2_buy_sell_parameters_from_pool_identity() {
        let params = pool()
            .build_buy_sell_parameters(&UniswapV2TradingSimulationConfig {
                block_number: Some(100),
                slippage_tolerance: Some(2.5),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(
            params.token_address,
            "0x0000000000000000000000000000000000000001"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(params.pool_type, PoolType::UniswapV2);
        assert_eq!(params.test_amount, U256::from(1_250_000u64));
        assert_eq!(params.block_number, Some(100));
        assert_eq!(params.denom_decimals, 6);
        assert_eq!(params.token_decimals, 9);
        assert_eq!(params.slippage_tolerance, 2.5);
        assert_eq!(pool().base.identity.protocol, UNISWAP_V2_PROTOCOL);
    }

    #[test]
    fn applies_trading_outcome_to_pool_state() {
        let mut pool = pool();
        let tx = UniswapV2TxContext::new(200, 1_700, "0xTX");
        let outcome = UniswapV2TradingSimulationOutcome {
            can_buy: true,
            can_approve: true,
            can_sell: true,
            is_tradeable: true,
            buy_tax_percent: 1.0,
            sell_tax_percent: 2.5,
            failure_reason: None,
            simulation_block_number: 199,
        };

        pool.apply_trading_simulation_outcome(&tx, &outcome);

        assert!(pool.base.state.can_buy);
        assert!(pool.base.state.can_sell);
        assert_eq!(pool.base.can_buy_block, Some(200));
        assert_eq!(pool.base.tax_check_block, Some(200));
        assert_eq!(pool.base.buy_tax, Some(1.0));
        assert_eq!(pool.base.sell_tax, Some(2.5));
    }
}
