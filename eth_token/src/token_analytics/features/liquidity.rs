use serde::{Deserialize, Serialize};

use super::utils::{finite_non_negative, ratio_to_initial, valid_positive};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolLiquidityFeatures {
    pub denom_reserve: f64,
    pub token_reserve: f64,
    pub total_liquidity_denom: f64,
    pub price_denom_per_token: f64,
    pub initial_price_denom_per_token: Option<f64>,
    pub price_to_initial_ratio: Option<f64>,
    pub initial_denom_reserve: Option<f64>,
    pub denom_reserve_to_initial_ratio: Option<f64>,
    pub initial_token_reserve: Option<f64>,
    pub token_reserve_to_initial_ratio: Option<f64>,
    pub max_denom_reserve_so_far: Option<f64>,
    pub denom_reserve_drawdown_from_max: Option<f64>,
    pub reserve_observation_count: u32,
}

impl PoolLiquidityFeatures {
    pub fn new(
        denom_reserve: f64,
        token_reserve: f64,
        total_liquidity_denom: f64,
        price_denom_per_token: f64,
    ) -> Self {
        Self {
            denom_reserve: finite_non_negative(denom_reserve),
            token_reserve: finite_non_negative(token_reserve),
            total_liquidity_denom: finite_non_negative(total_liquidity_denom),
            price_denom_per_token: finite_non_negative(price_denom_per_token),
            ..Default::default()
        }
    }

    pub fn with_initial_reserves(
        mut self,
        initial_denom_reserve: Option<f64>,
        initial_token_reserve: Option<f64>,
        initial_price_denom_per_token: Option<f64>,
    ) -> Self {
        self.initial_denom_reserve = valid_positive(initial_denom_reserve);
        self.initial_token_reserve = valid_positive(initial_token_reserve);
        self.initial_price_denom_per_token = valid_positive(initial_price_denom_per_token);
        self.denom_reserve_to_initial_ratio =
            ratio_to_initial(self.denom_reserve, self.initial_denom_reserve);
        self.token_reserve_to_initial_ratio =
            ratio_to_initial(self.token_reserve, self.initial_token_reserve);
        self.price_to_initial_ratio = ratio_to_initial(
            self.price_denom_per_token,
            self.initial_price_denom_per_token,
        );
        self
    }

    pub fn with_max_denom_reserve(mut self, max_denom_reserve_so_far: Option<f64>) -> Self {
        self.max_denom_reserve_so_far = valid_positive(max_denom_reserve_so_far);
        self.denom_reserve_drawdown_from_max = self.max_denom_reserve_so_far.and_then(|max| {
            if max > 0.0 {
                Some((max - self.denom_reserve).max(0.0) / max)
            } else {
                None
            }
        });
        self
    }
}
