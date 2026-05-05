use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::append_with_history_limit;

use super::data_models::{PoolLifecycle, PoolLiquiditySnapshot, PoolRuntimeState};
use super::reserves::PoolReserveTracker;

pub const DEFAULT_TEST_BUY_ETH: f64 = 0.01;

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
        if self.is_scam() {
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

        if denom_reserve >= self.config.denom_threshold {
            self.state.total_liquidity = denom_reserve;
        } else {
            self.state.total_liquidity = 0.0;
        }

        self.reserve_tracker.update_reserves(
            denom_reserve,
            token_reserve,
            price,
            block_number,
            timestamp,
            tx_hash,
        );
        self.sync_scam_state_from_reserve_tracker();
    }

    pub fn map_token_and_denom(&self, token0_value: f64, token1_value: f64) -> (f64, f64) {
        if self.config.token1_is_denom.unwrap_or(false) {
            (token0_value, token1_value)
        } else {
            (token1_value, token0_value)
        }
    }

    pub fn mark_can_buy_from_event(
        &mut self,
        block_number: u64,
        tx_hash: impl Into<String>,
        timestamp: u64,
    ) {
        if !self.state.can_buy {
            self.state.can_buy = true;
            self.can_buy_block = Some(block_number);
            self.can_buy_tx = Some(tx_hash.into());
            self.can_buy_timestamp = Some(timestamp);
            self.state.lifecycle = PoolLifecycle::Active;
        }
    }

    pub fn set_sell_status(
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

    pub fn is_scam(&self) -> bool {
        self.reserve_tracker.is_scam
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

    fn sync_scam_state_from_reserve_tracker(&mut self) {
        if self.reserve_tracker.is_scam {
            self.scam_label = self.reserve_tracker.scam_label.clone();
            self.scam_block = self.reserve_tracker.scam_block;
            self.scam_tx_hash = self.reserve_tracker.scam_tx_hash.clone();
            self.state.lifecycle = PoolLifecycle::Scam;
        } else {
            self.scam_label = None;
            self.scam_block = None;
            self.scam_tx_hash = None;
            if self.state.denom_reserve >= self.config.denom_threshold
                && self.state.lifecycle == PoolLifecycle::Discovered
            {
                self.state.lifecycle = PoolLifecycle::LiquidityDeposited;
            }
        }
    }
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
    fn below_threshold_marks_pool_as_scam_and_zeroes_price() {
        let mut pool = test_pool();

        pool.update_reserves(100.0, 0.01, 10, 1_700, "0xTX");

        assert!(pool.is_scam());
        assert_eq!(pool.price(), 0.0);
        assert_eq!(pool.state.lifecycle, PoolLifecycle::Scam);
        assert_eq!(pool.scam_block, Some(10));
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

        pool.mark_can_buy_from_event(20, "0xBUY", 2_000);
        pool.set_sell_status(true, Some(1.5), Some(2.0), 21, "0xSELL");

        let status = pool.trading_status();
        assert!(status.trading_enabled);
        assert!(status.can_buy_and_sell);
        assert_eq!(status.block, Some(20));
        assert_eq!(status.tx.as_deref(), Some("0xBUY"));
        assert_eq!(status.sell_tax, Some(2.0));
        assert_eq!(pool.trading_age_blocks(25), Some(5));
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
