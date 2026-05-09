use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PoolLifecycle {
    #[default]
    Discovered,
    LiquidityDeposited,
    Trading,
    CannotSell,
    Dust,
    Drained,
    Active,
    Scam,
    Evicted,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolRuntimeState {
    pub denom_reserve: f64,
    pub token_reserve: f64,
    pub total_liquidity: f64,
    pub price_token_per_denom: f64,
    pub price_denom_per_token: f64,
    pub last_update_block: u64,
    pub last_sync_block: u64,
    pub lifecycle: PoolLifecycle,
    pub can_buy: bool,
    pub can_sell: bool,
    pub denom_volume_in: f64,
    pub token_volume_in: f64,
    pub denom_volume_out: f64,
    pub token_volume_out: f64,
    pub total_mints: u64,
    pub total_burns: u64,
    pub total_swaps: u64,
}

impl Default for PoolRuntimeState {
    fn default() -> Self {
        Self {
            denom_reserve: 0.0,
            token_reserve: 0.0,
            total_liquidity: 0.0,
            price_token_per_denom: 0.0,
            price_denom_per_token: 0.0,
            last_update_block: 0,
            last_sync_block: 0,
            lifecycle: PoolLifecycle::Discovered,
            can_buy: false,
            can_sell: false,
            denom_volume_in: 0.0,
            token_volume_in: 0.0,
            denom_volume_out: 0.0,
            token_volume_out: 0.0,
            total_mints: 0,
            total_burns: 0,
            total_swaps: 0,
        }
    }
}

impl PoolRuntimeState {
    pub fn update_reserves(&mut self, denom_reserve: f64, token_reserve: f64, block_number: u64) {
        self.denom_reserve = denom_reserve;
        self.token_reserve = token_reserve;
        self.last_update_block = block_number;
        self.last_sync_block = block_number;
        self.update_prices();

        if denom_reserve > 0.0 || token_reserve > 0.0 {
            self.lifecycle = PoolLifecycle::LiquidityDeposited;
        }
    }

    pub fn mark_active(&mut self) {
        self.can_buy = true;
        self.can_sell = true;
        self.lifecycle = PoolLifecycle::Trading;
    }

    pub fn mark_scam(&mut self) {
        self.can_buy = false;
        self.can_sell = false;
        self.lifecycle = PoolLifecycle::Scam;
    }

    pub fn record_swap(&mut self, denom_in: f64, token_in: f64, denom_out: f64, token_out: f64) {
        self.denom_volume_in += denom_in.max(0.0);
        self.token_volume_in += token_in.max(0.0);
        self.denom_volume_out += denom_out.max(0.0);
        self.token_volume_out += token_out.max(0.0);
        self.total_swaps += 1;
    }

    pub fn record_mint(&mut self, liquidity_delta: f64) {
        self.total_liquidity += liquidity_delta.max(0.0);
        self.total_mints += 1;
    }

    pub fn record_burn(&mut self, liquidity_delta: f64) {
        self.total_liquidity = (self.total_liquidity - liquidity_delta.max(0.0)).max(0.0);
        self.total_burns += 1;
    }

    fn update_prices(&mut self) {
        self.price_token_per_denom = if self.denom_reserve > 0.0 {
            self.token_reserve / self.denom_reserve
        } else {
            0.0
        };
        self.price_denom_per_token = if self.token_reserve > 0.0 {
            self.denom_reserve / self.token_reserve
        } else {
            0.0
        };
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolLiquiditySnapshot {
    pub pool_address: String,
    pub denom_address: String,
    pub protocol: String,
    pub price: f64,
    pub denom_reserve: f64,
    pub token_reserve: f64,
    pub can_buy: bool,
    pub can_sell: bool,
}

impl PoolLiquiditySnapshot {
    pub fn new(
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        protocol: impl Into<String>,
        state: &PoolRuntimeState,
    ) -> Self {
        Self {
            pool_address: pool_address.into(),
            denom_address: denom_address.into(),
            protocol: protocol.into(),
            price: state.price_denom_per_token,
            denom_reserve: state.denom_reserve,
            token_reserve: state.token_reserve,
            can_buy: state.can_buy,
            can_sell: state.can_sell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn runtime_state_defaults_match_python_defaults() {
        let state = PoolRuntimeState::default();

        assert_eq!(state.denom_reserve, 0.0);
        assert_eq!(state.token_reserve, 0.0);
        assert_eq!(state.lifecycle, PoolLifecycle::Discovered);
        assert!(!state.can_buy);
        assert!(!state.can_sell);
        assert_eq!(state.total_swaps, 0);
    }

    #[test]
    fn lifecycle_serializes_like_python_enum_values() {
        assert_eq!(
            serde_json::to_value(PoolLifecycle::LiquidityDeposited).unwrap(),
            json!("LIQUIDITY_DEPOSITED")
        );
    }

    #[test]
    fn update_reserves_updates_prices_and_lifecycle() {
        let mut state = PoolRuntimeState::default();

        state.update_reserves(2.0, 10.0, 123);

        assert_eq!(state.last_update_block, 123);
        assert_eq!(state.last_sync_block, 123);
        assert_eq!(state.price_token_per_denom, 5.0);
        assert_eq!(state.price_denom_per_token, 0.2);
        assert_eq!(state.lifecycle, PoolLifecycle::LiquidityDeposited);
    }

    #[test]
    fn liquidity_snapshot_uses_pool_state() {
        let mut state = PoolRuntimeState::default();
        state.update_reserves(2.0, 10.0, 123);
        state.mark_active();

        let snapshot = PoolLiquiditySnapshot::new("pool", "denom", "UNISWAP-V2", &state);

        assert_eq!(snapshot.pool_address, "pool");
        assert_eq!(snapshot.price, 0.2);
        assert!(snapshot.can_buy);
        assert!(snapshot.can_sell);
    }
}
