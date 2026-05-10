use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::append_with_history_limit;

use super::data_models::{PoolLifecycle, PoolLiquiditySnapshot, PoolRuntimeState};
use super::reserves::PoolReserveTracker;

pub const DEFAULT_TEST_BUY_ETH: f64 = 0.01;
const MIN_MEANINGFUL_WETH_LIQUIDITY: f64 = 0.01;
const MIN_MEANINGFUL_STABLE_LIQUIDITY: f64 = 10.0;
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolIdentity {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub protocol: String,
}

impl PoolIdentity {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        protocol: impl Into<String>,
    ) -> Self {
        Self {
            pool_address: normalize_address_string(pool_address),
            token_address: normalize_address_string(token_address),
            denom_address: normalize_address_string(denom_address),
            protocol: protocol.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasePoolConfig {
    pub token_decimals: u8,
    pub denom_decimals: Option<u8>,
    pub token1_is_denom: Option<bool>,
    pub history_limit: usize,
    pub denom_threshold: f64,
    pub threshold_unit: Option<String>,
    pub test_buy_amount_eth: f64,
}

impl BasePoolConfig {
    pub fn new(token_decimals: u8) -> Self {
        Self {
            token_decimals,
            denom_decimals: None,
            token1_is_denom: None,
            history_limit: 100,
            denom_threshold: 0.0,
            threshold_unit: None,
            test_buy_amount_eth: DEFAULT_TEST_BUY_ETH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TradingStatus {
    pub can_buy: bool,
    pub can_sell: bool,
    pub trading_enabled: bool,
    pub can_buy_and_sell: bool,
    pub block: Option<u64>,
    pub tx: Option<String>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_check_block: Option<u64>,
    pub tax_check_tx: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasePool {
    pub identity: PoolIdentity,
    pub config: BasePoolConfig,
    pub state: PoolRuntimeState,
    pub sync_events: Vec<Value>,
    pub swap_events: Vec<Value>,
    pub mint_events: Vec<Value>,
    pub burn_events: Vec<Value>,
    pub price_history: Vec<(u64, f64)>,
    pub creation_block: Option<u64>,
    pub creation_tx: Option<String>,
    pub creation_timestamp: Option<u64>,
    pub can_buy_block: Option<u64>,
    pub can_buy_tx: Option<String>,
    pub can_buy_timestamp: Option<u64>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_check_block: Option<u64>,
    pub tax_check_tx: Option<String>,
    pub last_trading_failure_reason: Option<String>,
    pub last_trading_failure_class: Option<String>,
    pub scam_label: Option<String>,
    pub scam_block: Option<u64>,
    pub scam_tx_hash: Option<String>,
    pub reserve_tracker: PoolReserveTracker,
    pub token_control_addresses: HashSet<String>,
    pub latest_block_number: Option<u64>,
    pub latest_block_control_address_txs: HashMap<String, Value>,
}

impl BasePool {
    pub fn new(identity: PoolIdentity, config: BasePoolConfig) -> Self {
        let mut reserve_tracker = PoolReserveTracker::new(
            identity.pool_address.clone(),
            identity.denom_address.clone(),
            Some(identity.token_address.clone()),
            Some(identity.protocol.clone()),
            config.history_limit,
        );
        if config.denom_threshold > 0.0 {
            reserve_tracker.denom_threshold = Some(config.denom_threshold);
            reserve_tracker.threshold_unit = config.threshold_unit.clone();
        }

        Self {
            identity,
            config,
            state: PoolRuntimeState::default(),
            sync_events: Vec::new(),
            swap_events: Vec::new(),
            mint_events: Vec::new(),
            burn_events: Vec::new(),
            price_history: Vec::new(),
            creation_block: None,
            creation_tx: None,
            creation_timestamp: None,
            can_buy_block: None,
            can_buy_tx: None,
            can_buy_timestamp: None,
            buy_tax: None,
            sell_tax: None,
            tax_check_block: None,
            tax_check_tx: None,
            last_trading_failure_reason: None,
            last_trading_failure_class: None,
            scam_label: None,
            scam_block: None,
            scam_tx_hash: None,
            reserve_tracker,
            token_control_addresses: HashSet::new(),
            latest_block_number: None,
            latest_block_control_address_txs: HashMap::new(),
        }
    }

    pub fn price(&self) -> f64 {
        if self.has_liquidity_removal() {
            0.0
        } else {
            self.state.price_denom_per_token.max(0.0)
        }
    }

    pub fn token_reserve(&self) -> f64 {
        self.state.token_reserve
    }

    pub fn denom_reserve(&self) -> f64 {
        self.state.denom_reserve
    }

    pub fn initial_price(&self) -> Option<f64> {
        self.reserve_tracker
            .initial_price()
            .filter(|value| value.is_finite())
    }

    pub fn price_ratio_to_initial(&self) -> Option<f64> {
        self.reserve_tracker
            .price_ratio_to_initial()
            .filter(|value| value.is_finite())
    }

    pub fn fully_diluted_value_denom(&self, total_supply: f64) -> Option<f64> {
        let price = self.price();
        if total_supply > 0.0 && price > 0.0 && total_supply.is_finite() && price.is_finite() {
            Some(total_supply * price)
        } else {
            None
        }
    }

    pub fn pooled_token_supply_ratio(&self, total_supply: f64) -> Option<f64> {
        let token_reserve = self.token_reserve();
        if total_supply > 0.0 && token_reserve >= 0.0 && total_supply.is_finite() {
            Some(token_reserve / total_supply)
        } else {
            None
        }
    }

    pub fn liquidity_to_fdv_ratio(&self, total_supply: f64) -> Option<f64> {
        let fdv = self.fully_diluted_value_denom(total_supply)?;
        let liquidity = self.state.total_liquidity;
        if fdv > 0.0 && liquidity >= 0.0 && liquidity.is_finite() {
            Some(liquidity / fdv)
        } else {
            None
        }
    }

    pub fn update_reserves(
        &mut self,
        token_reserve: f64,
        denom_reserve: f64,
        block_number: u64,
        timestamp: u64,
        tx_hash: impl Into<String>,
    ) {
        let tx_hash = tx_hash.into();
        self.state
            .update_reserves(denom_reserve, token_reserve, block_number);

        let price = self.price();
        if price > 0.0 {
            append_with_history_limit(
                &mut self.price_history,
                (block_number, price),
                self.config.history_limit,
            );
        }

        self.state.total_liquidity = denom_reserve.max(0.0);

        self.reserve_tracker.update_reserves(
            denom_reserve,
            token_reserve,
            price,
            block_number,
            timestamp,
            tx_hash,
        );
        self.sync_liquidity_removal_state_from_reserve_tracker();
    }

    pub fn map_token_and_denom(&self, token0_value: f64, token1_value: f64) -> (f64, f64) {
        if self.config.token1_is_denom.unwrap_or(false) {
            (token0_value, token1_value)
        } else {
            (token1_value, token0_value)
        }
    }

    pub fn set_simulated_buy_status(
        &mut self,
        can_buy: bool,
        block_number: u64,
        tx_hash: impl Into<String>,
        timestamp: u64,
    ) {
        self.state.can_buy = can_buy;
        if can_buy && self.can_buy_block.is_none() {
            self.can_buy_block = Some(block_number);
            self.can_buy_tx = Some(tx_hash.into());
            self.can_buy_timestamp = Some(timestamp);
        }
        self.refresh_lifecycle();
    }

    pub fn set_simulated_sell_status(
        &mut self,
        can_sell: bool,
        buy_tax: Option<f64>,
        sell_tax: Option<f64>,
        block_number: u64,
        tx_hash: impl Into<String>,
    ) {
        self.state.can_sell = can_sell;
        self.buy_tax = buy_tax;
        self.sell_tax = sell_tax;
        self.tax_check_block = Some(block_number);
        self.tax_check_tx = Some(tx_hash.into());
        if can_sell {
            self.last_trading_failure_reason = None;
            self.last_trading_failure_class = None;
        }
        self.refresh_lifecycle();
    }

    pub fn set_trading_failure_context(&mut self, reason: Option<String>, class: Option<String>) {
        self.last_trading_failure_reason = reason;
        self.last_trading_failure_class = class;
    }

    pub fn trading_status(&self) -> TradingStatus {
        TradingStatus {
            can_buy: self.state.can_buy,
            can_sell: self.state.can_sell,
            trading_enabled: self.trading_enabled(),
            can_buy_and_sell: self.can_buy_and_sell(),
            block: self.can_buy_block,
            tx: self.can_buy_tx.clone(),
            buy_tax: self.buy_tax,
            sell_tax: self.sell_tax,
            tax_check_block: self.tax_check_block,
            tax_check_tx: self.tax_check_tx.clone(),
        }
    }

    pub fn trading_enabled(&self) -> bool {
        self.state.can_buy
    }

    pub fn can_buy_and_sell(&self) -> bool {
        self.state.can_buy && self.state.can_sell
    }

    pub fn effective_can_buy(&self) -> bool {
        self.current_liquidity_allows_trading() && (self.state.can_buy || self.has_observed_buy())
    }

    pub fn effective_can_sell(&self) -> bool {
        self.current_liquidity_allows_trading() && (self.state.can_sell || self.has_observed_sell())
    }

    pub fn has_observed_buy(&self) -> bool {
        positive_finite(self.state.denom_volume_in) && positive_finite(self.state.token_volume_out)
    }

    pub fn has_observed_sell(&self) -> bool {
        positive_finite(self.state.token_volume_in) && positive_finite(self.state.denom_volume_out)
    }

    pub fn current_liquidity_allows_trading(&self) -> bool {
        if matches!(
            self.state.lifecycle,
            PoolLifecycle::Dust
                | PoolLifecycle::Drained
                | PoolLifecycle::LiquidityRemoved
                | PoolLifecycle::Evicted
        ) || self.has_liquidity_removal()
        {
            return false;
        }

        let has_seen_reserves = self.state.last_update_block > 0 || self.state.last_sync_block > 0;
        !has_seen_reserves || self.has_meaningful_liquidity()
    }

    pub fn is_scam(&self) -> bool {
        self.has_liquidity_removal()
    }

    pub fn has_liquidity_removal(&self) -> bool {
        self.reserve_tracker.is_scam
    }

    pub fn mark_liquidity_removal(
        &mut self,
        label: impl Into<String>,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) {
        self.reserve_tracker.is_scam = true;
        self.reserve_tracker.scam_label = Some(label.into());
        self.reserve_tracker.scam_block = block_number;
        self.reserve_tracker.scam_tx_hash = tx_hash;
        self.sync_liquidity_removal_state_from_reserve_tracker();
    }

    pub fn register_token_control_addresses(
        &mut self,
        addresses: impl IntoIterator<Item = impl AsRef<str>>,
    ) {
        self.token_control_addresses
            .extend(addresses.into_iter().filter_map(normalize_address));
    }

    pub fn has_control_address(
        &self,
        addresses: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> bool {
        addresses
            .into_iter()
            .filter_map(normalize_address)
            .any(|address| self.token_control_addresses.contains(&address))
    }

    pub fn set_latest_block_control_transactions(
        &mut self,
        block_number: Option<u64>,
        transactions: HashMap<String, Value>,
    ) {
        self.latest_block_number = block_number;
        self.latest_block_control_address_txs = transactions;
    }

    pub fn clear_latest_block_control_transactions(&mut self) {
        self.latest_block_number = None;
        self.latest_block_control_address_txs.clear();
    }

    pub fn latest_block_control_address_txs_list(&self) -> Vec<&Value> {
        self.latest_block_control_address_txs.values().collect()
    }

    pub fn pool_age_blocks(&self, current_block: u64) -> Option<u64> {
        self.creation_block
            .map(|creation_block| current_block.saturating_sub(creation_block))
    }

    pub fn trading_age_blocks(&self, current_block: u64) -> Option<u64> {
        self.can_buy_block
            .filter(|_| self.state.can_buy)
            .map(|can_buy_block| current_block.saturating_sub(can_buy_block))
    }

    pub fn liquidity_snapshot(&self) -> PoolLiquiditySnapshot {
        PoolLiquiditySnapshot::new(
            self.identity.pool_address.clone(),
            self.identity.denom_address.clone(),
            self.identity.protocol.clone(),
            &self.state,
        )
    }

    fn sync_liquidity_removal_state_from_reserve_tracker(&mut self) {
        if self.reserve_tracker.is_scam {
            self.scam_label = self.reserve_tracker.scam_label.clone();
            self.scam_block = self.reserve_tracker.scam_block;
            self.scam_tx_hash = self.reserve_tracker.scam_tx_hash.clone();
            self.clear_current_trading_status();
            self.state.lifecycle = PoolLifecycle::LiquidityRemoved;
        } else {
            self.scam_label = None;
            self.scam_block = None;
            self.scam_tx_hash = None;
            self.refresh_lifecycle();
        }
    }

    fn refresh_lifecycle(&mut self) {
        if self.reserve_tracker.is_scam {
            self.clear_current_trading_status();
            self.state.lifecycle = PoolLifecycle::LiquidityRemoved;
            return;
        }

        let has_seen_reserves = self.state.last_update_block > 0 || self.state.last_sync_block > 0;
        if !has_seen_reserves {
            self.state.lifecycle = if self.state.can_buy && self.state.can_sell {
                PoolLifecycle::Trading
            } else if self.state.can_buy {
                PoolLifecycle::CannotSell
            } else {
                PoolLifecycle::Discovered
            };
            return;
        }

        if self.state.denom_reserve <= 0.0 || self.state.token_reserve <= 0.0 {
            self.clear_current_trading_status();
            self.state.lifecycle = PoolLifecycle::Drained;
            return;
        }

        if !self.has_meaningful_liquidity() {
            self.clear_current_trading_status();
            self.state.lifecycle = PoolLifecycle::Dust;
            return;
        }

        self.state.lifecycle = if self.state.can_buy && self.state.can_sell {
            PoolLifecycle::Trading
        } else if self.state.can_buy {
            PoolLifecycle::CannotSell
        } else {
            PoolLifecycle::LiquidityDeposited
        };
    }

    fn has_meaningful_liquidity(&self) -> bool {
        let configured_threshold = self.config.denom_threshold.max(0.0);
        let display_threshold = meaningful_liquidity_threshold(&self.identity.denom_address);
        self.state.denom_reserve >= configured_threshold.max(display_threshold)
            && self.state.token_reserve > 0.0
    }

    fn clear_current_trading_status(&mut self) {
        self.state.can_buy = false;
        self.state.can_sell = false;
    }
}

fn meaningful_liquidity_threshold(denom_address: &str) -> f64 {
    match normalize_address_string(denom_address) {
        address if address == WETH_ADDRESS => MIN_MEANINGFUL_WETH_LIQUIDITY,
        address if is_stable_denom(&address) => MIN_MEANINGFUL_STABLE_LIQUIDITY,
        _ => 0.0,
    }
}

fn is_stable_denom(denom_address: &str) -> bool {
    matches!(denom_address, USDC_ADDRESS | USDT_ADDRESS | DAI_ADDRESS)
}

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn normalize_address(value: impl AsRef<str>) -> Option<String> {
    let cleaned = value.as_ref().trim();
    if cleaned.is_empty() {
        return None;
    }
    Some(cleaned.to_ascii_lowercase())
}

fn normalize_address_string(value: impl Into<String>) -> String {
    value.into().trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_pool() -> BasePool {
        BasePool::new(
            PoolIdentity::new("0xPOOL", "0xTOKEN", "0xDENOM", "UNISWAP-V2"),
            BasePoolConfig {
                denom_threshold: 0.05,
                threshold_unit: Some("ETH".to_string()),
                token1_is_denom: Some(true),
                ..BasePoolConfig::new(18)
            },
        )
    }

    fn weth_pool() -> BasePool {
        BasePool::new(
            PoolIdentity::new("0xPOOL", "0xTOKEN", WETH_ADDRESS, "UNISWAP-V2"),
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                ..BasePoolConfig::new(18)
            },
        )
    }

    #[test]
    fn update_reserves_sets_prices_liquidity_and_history() {
        let mut pool = test_pool();

        pool.update_reserves(100.0, 2.0, 10, 1_700, "0xTX");

        assert_eq!(pool.token_reserve(), 100.0);
        assert_eq!(pool.denom_reserve(), 2.0);
        assert_eq!(pool.price(), 0.02);
        assert_eq!(pool.state.total_liquidity, 2.0);
        assert_eq!(pool.price_history, vec![(10, 0.02)]);
    }

    #[test]
    fn valuation_ratios_are_computed_from_pool_state() {
        let mut pool = test_pool();

        pool.update_reserves(100.0, 2.0, 10, 1_700, "0xTX1");
        pool.update_reserves(50.0, 2.0, 11, 1_710, "0xTX2");

        assert_eq!(pool.initial_price(), Some(0.02));
        assert_eq!(pool.price_ratio_to_initial(), Some(2.0));
        assert_eq!(pool.fully_diluted_value_denom(1_000.0), Some(40.0));
        assert_eq!(pool.pooled_token_supply_ratio(1_000.0), Some(0.05));
        assert_eq!(pool.liquidity_to_fdv_ratio(1_000.0), Some(0.05));
    }

    #[test]
    fn below_threshold_marks_pool_as_liquidity_removed_and_zeroes_price() {
        let mut pool = test_pool();

        pool.update_reserves(100.0, 0.01, 10, 1_700, "0xTX");

        assert!(pool.has_liquidity_removal());
        assert_eq!(pool.price(), 0.0);
        assert_eq!(pool.state.lifecycle, PoolLifecycle::LiquidityRemoved);
        assert_eq!(pool.scam_block, Some(10));
    }

    #[test]
    fn weth_dust_reserve_is_dust_not_liquidity_deposited() {
        let mut pool = weth_pool();

        pool.update_reserves(100.0, 0.001, 10, 1_700, "0xTX");

        assert!(!pool.has_liquidity_removal());
        assert_eq!(pool.state.total_liquidity, 0.001);
        assert_eq!(pool.state.lifecycle, PoolLifecycle::Dust);
    }

    #[test]
    fn current_pool_lifecycle_tracks_trading_and_cannot_sell() {
        let mut pool = weth_pool();

        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        assert_eq!(pool.state.lifecycle, PoolLifecycle::LiquidityDeposited);

        pool.set_simulated_buy_status(true, 11, "0xBUY", 1_710);
        assert_eq!(pool.state.lifecycle, PoolLifecycle::CannotSell);

        pool.set_simulated_sell_status(true, Some(0.0), Some(0.0), 12, "0xSELL");
        assert_eq!(pool.state.lifecycle, PoolLifecycle::Trading);
    }

    #[test]
    fn empty_reserves_mark_seen_pool_as_drained() {
        let mut pool = weth_pool();

        pool.update_reserves(0.0, 0.0, 10, 1_700, "0xSYNC");

        assert_eq!(pool.state.lifecycle, PoolLifecycle::Drained);
    }

    #[test]
    fn depleted_reserves_clear_current_trading_status_but_keep_first_buy_metadata() {
        let mut pool = weth_pool();

        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        pool.set_simulated_buy_status(true, 11, "0xBUY", 1_710);
        pool.set_simulated_sell_status(true, Some(0.0), Some(0.0), 12, "0xSELL");
        assert!(pool.trading_enabled());
        assert!(pool.can_buy_and_sell());

        pool.update_reserves(0.0, 0.0, 13, 1_730, "0xDRAIN");

        assert_eq!(pool.state.lifecycle, PoolLifecycle::Drained);
        assert!(!pool.state.can_buy);
        assert!(!pool.state.can_sell);
        assert!(!pool.trading_enabled());
        assert!(!pool.can_buy_and_sell());
        assert_eq!(pool.can_buy_block, Some(11));
        assert_eq!(pool.can_buy_tx.as_deref(), Some("0xBUY"));
        assert_eq!(pool.can_buy_timestamp, Some(1_710));
    }

    #[test]
    fn dust_reserves_clear_current_trading_status() {
        let mut pool = weth_pool();

        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        pool.set_simulated_buy_status(true, 11, "0xBUY", 1_710);
        pool.set_simulated_sell_status(true, Some(0.0), Some(0.0), 12, "0xSELL");

        pool.update_reserves(100.0, 0.001, 13, 1_730, "0xDUST");

        assert_eq!(pool.state.lifecycle, PoolLifecycle::Dust);
        assert!(!pool.state.can_buy);
        assert!(!pool.state.can_sell);
        assert_eq!(pool.can_buy_block, Some(11));
    }

    #[test]
    fn control_addresses_are_normalized_for_membership_checks() {
        let mut pool = test_pool();
        pool.register_token_control_addresses(["0xAbC", ""]);

        assert!(pool.has_control_address(["0xabc"]));
        assert!(!pool.has_control_address(["0xdef"]));
    }

    #[test]
    fn trading_status_tracks_buy_and_sell_state() {
        let mut pool = test_pool();

        pool.set_simulated_buy_status(true, 20, "0xBUY", 2_000);
        pool.set_simulated_sell_status(true, Some(1.5), Some(2.0), 21, "0xSELL");

        let status = pool.trading_status();
        assert!(status.trading_enabled);
        assert!(status.can_buy_and_sell);
        assert_eq!(status.block, Some(20));
        assert_eq!(status.tx.as_deref(), Some("0xBUY"));
        assert_eq!(status.sell_tax, Some(2.0));
        assert_eq!(pool.trading_age_blocks(25), Some(5));
    }

    #[test]
    fn observed_swaps_are_chain_evidence_without_mutating_simulation_flags() {
        let mut pool = weth_pool();
        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        pool.set_simulated_buy_status(true, 11, "0xBUY", 1_710);
        pool.set_simulated_sell_status(false, None, None, 12, "0xSIM_FAIL");

        pool.state.record_swap(0.0, 25.0, 0.1, 0.0);

        assert!(pool.has_observed_sell());
        assert!(!pool.state.can_sell);
        assert!(pool.effective_can_sell());
        assert!(!pool.can_buy_and_sell());

        let status = pool.trading_status();
        assert!(!status.can_sell);
        assert!(!status.can_buy_and_sell);
    }

    #[test]
    fn observed_swaps_do_not_make_dust_or_drained_pools_tradable() {
        let mut pool = weth_pool();
        pool.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
        pool.state.record_swap(1.0, 25.0, 0.1, 10.0);
        assert!(pool.effective_can_buy());
        assert!(pool.effective_can_sell());
        assert!(!pool.trading_enabled());

        pool.update_reserves(100.0, 0.001, 11, 1_712, "0xDUST");

        assert!(pool.has_observed_buy());
        assert!(pool.has_observed_sell());
        assert!(!pool.effective_can_buy());
        assert!(!pool.effective_can_sell());
        assert!(!pool.trading_enabled());
    }

    #[test]
    fn latest_control_transactions_can_be_set_and_cleared() {
        let mut pool = test_pool();
        let mut txs = HashMap::new();
        txs.insert("0x1".to_string(), json!({"hash": "0x1"}));

        pool.set_latest_block_control_transactions(Some(30), txs);
        assert_eq!(pool.latest_block_number, Some(30));
        assert_eq!(pool.latest_block_control_address_txs_list().len(), 1);

        pool.clear_latest_block_control_transactions();
        assert!(pool.latest_block_number.is_none());
        assert!(pool.latest_block_control_address_txs.is_empty());
    }
}
