use alloy_primitives::{Address, U256};
use eth_pool_classification::{
    classify_pool_with_config, EligiblePoolOutcome, PoolClassification, PoolClassificationConfig,
    PoolClassificationInput,
};
use eth_price::liquidity::{assess_denom_liquidity, LiquidityReference, PoolLiquidityLevel};
use eth_token::erc20::ERC20Token;
use eth_token::pools::{
    BalancerPool, BasePool, CurvePool, LPHolderSnapshot, PoolLifecycle, PoolRuntimeState,
    TaxBucket, TradingStatus, TradingStatusSnapshot, UniswapV2Pool, UniswapV3Pool, UniswapV4Pool,
};
use eth_token::token_activity::TokenBlockActivity;
use reth_chain_query::common_addresses::get_token_symbol;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;

use display::{
    current_lifecycle_view, current_trading_view, display_price_ratio, display_price_ratio_history,
    display_reserve_quality, display_supply_ratio, display_tax, economic_sellable_from_tax,
    liquidity_history, max_denom_reserve, pool_risk, ratio_percent, tax_bucket_key,
    truncate_history_at_liquidity_removal,
};

mod display;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Serialize)]
pub struct PriceRatioPoint {
    pub block_number: u64,
    pub price: f64,
    pub ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiquidityPoint {
    pub block_number: u64,
    pub liquidity: f64,
    pub denom_reserve: f64,
    pub token_reserve: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolRiskLevel {
    Clear,
    LiquidityRemoval,
    Honeypot,
    HighTax,
    ExtremeTax,
}

#[derive(Clone, Debug)]
struct PoolRiskView {
    level: PoolRiskLevel,
    label: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct CurrentTradingView {
    can_buy: bool,
    can_sell: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolView {
    pub token_address: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    pub pool_address: String,
    pub protocol: String,
    pub pool_id: Option<String>,
    pub pool_manager_address: Option<String>,
    pub currency0: Option<String>,
    pub currency1: Option<String>,
    pub fee_tier: Option<u32>,
    pub tick_spacing: Option<i32>,
    pub hooks: Option<String>,
    pub current_tick: Option<i32>,
    pub sqrt_price_x96: Option<String>,
    pub active_liquidity: Option<String>,
    pub virtual_reserves: Option<VirtualReserveView>,
    pub vault_address: Option<String>,
    pub lp_token_address: Option<String>,
    pub swap_fee_bps: Option<u32>,
    pub base_token_index: Option<usize>,
    pub quote_token_index: Option<usize>,
    pub pool_tokens: Vec<PoolComponentView>,
    pub denom_address: String,
    pub denom_symbol: Option<String>,
    pub currency: String,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub price: f64,
    pub initial_price: Option<f64>,
    pub raw_price_ratio_to_initial: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub price_ratio_history: Vec<PriceRatioPoint>,
    pub liquidity_history: Vec<LiquidityPoint>,
    pub total_liquidity: f64,
    pub liquidity_level: PoolLiquidityLevel,
    pub liquidity_label: String,
    pub liquidity_rank: u8,
    pub liquidity_rank_score: f64,
    pub liquidity_eth: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub liquidity_denom_class: String,
    pub token_total_supply_scaled: Option<f64>,
    pub fully_diluted_value_denom: Option<f64>,
    pub pooled_token_supply_ratio: Option<f64>,
    pub pooled_token_supply_percent: Option<f64>,
    pub liquidity_to_fdv_ratio: Option<f64>,
    pub liquidity_to_fdv_percent: Option<f64>,
    pub supply_ratio_status: String,
    pub supply_ratio_label: Option<String>,
    pub reserve_quality_status: String,
    pub reserve_quality_label: Option<String>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub economic_sellable: Option<bool>,
    pub has_observed_buy: bool,
    pub has_observed_sell: bool,
    pub trading_enabled: bool,
    pub stage: PoolLifecycle,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub buy_tax_bucket: TaxBucket,
    pub sell_tax_bucket: TaxBucket,
    pub tax_bucket: TaxBucket,
    pub last_trading_failure_reason: Option<String>,
    pub last_trading_failure_class: Option<String>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub scam_mechanism_evidence: Option<Value>,
    pub liquidity_removal: bool,
    pub liquidity_removal_label: Option<String>,
    pub liquidity_removal_block: Option<u64>,
    pub liquidity_removal_tx_hash: Option<String>,
    pub risk_level: PoolRiskLevel,
    pub risk_label: Option<String>,
    pub pool_classification: PoolClassification,
    pub strategy_classification: PoolClassification,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub can_buy_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub trading_status: TradingStatus,
    pub trading_status_history: Vec<TradingStatusSnapshot>,
    pub runtime_state: PoolRuntimeState,
    pub lp_total_supply: f64,
    pub lp_supply_known: bool,
    pub lp_supply_status: String,
    pub lp_holder_count: usize,
    pub lp_holders: Vec<LPHolderSnapshot>,
    pub lp_total_approved_to_routers: f64,
    pub lp_approved_percentage: f64,
    pub lp_last_approval_block: Option<u64>,
    pub lp_last_approval: Option<Value>,
    pub lp_holders_with_approvals: Vec<String>,
    pub lp_transfer_count: usize,
    pub lp_approval_count: usize,
    pub liquidity_positions: Vec<ConcentratedLiquidityPositionView>,
    pub recent_denom_activity: Vec<TokenBlockActivity>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct VirtualReserveView {
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub token0_reserve: f64,
    pub token1_reserve: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolComponentView {
    pub symbol: Option<String>,
    pub address: String,
    pub decimals: u8,
    pub index: usize,
    pub weight: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConcentratedLiquidityPositionView {
    pub position_id: String,
    pub token_id: Option<String>,
    pub owner: String,
    pub position_manager_address: Option<String>,
    pub liquidity: String,
    pub position_share_pct: Option<f64>,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub last_update_block: u64,
    pub last_update_tx: String,
}

#[derive(Clone, Debug, Default)]
struct ConcentratedPoolViewFields {
    pool_id: Option<String>,
    pool_manager_address: Option<String>,
    currency0: Option<String>,
    currency1: Option<String>,
    fee_tier: Option<u32>,
    tick_spacing: Option<i32>,
    hooks: Option<String>,
    current_tick: Option<i32>,
    sqrt_price_x96: Option<String>,
    active_liquidity: Option<String>,
    virtual_reserves: Option<VirtualReserveView>,
    vault_address: Option<String>,
    lp_token_address: Option<String>,
    swap_fee_bps: Option<u32>,
    base_token_index: Option<usize>,
    quote_token_index: Option<usize>,
    pool_tokens: Vec<PoolComponentView>,
    liquidity_positions: Vec<ConcentratedLiquidityPositionView>,
}

#[derive(Clone, Debug, Default)]
struct LpPoolViewFields {
    lp_total_supply: f64,
    lp_supply_known: bool,
    lp_supply_status: &'static str,
    lp_holder_count: usize,
    lp_holders: Vec<LPHolderSnapshot>,
    lp_total_approved_to_routers: f64,
    lp_approved_percentage: f64,
    lp_last_approval_block: Option<u64>,
    lp_last_approval: Option<Value>,
    lp_holders_with_approvals: Vec<String>,
    lp_transfer_count: usize,
    lp_approval_count: usize,
}

impl PoolView {
    pub fn from_pool(token: &ERC20Token, pool: &UniswapV2Pool) -> Self {
        Self::from_v2_pool(token, pool)
    }

    pub fn from_v2_pool(token: &ERC20Token, pool: &UniswapV2Pool) -> Self {
        Self::from_v2_pool_with_activity(token, pool, &[])
    }

    pub fn from_v2_pool_with_activity(
        token: &ERC20Token,
        pool: &UniswapV2Pool,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        let lp_holders = pool.lp_holders();
        let lp_holder_count = lp_holders.len();
        let lp_supply_known =
            pool.lp_tracker.total_supply > 0.0 || !pool.lp_tracker.mint_events.is_empty();
        let lp_fields = LpPoolViewFields {
            lp_total_supply: pool.lp_tracker.total_supply,
            lp_supply_known,
            lp_supply_status: if lp_supply_known {
                "observed"
            } else {
                "unknown"
            },
            lp_holder_count,
            lp_holders,
            lp_total_approved_to_routers: pool.total_approved_to_routers(),
            lp_approved_percentage: pool.lp_approved_percentage(),
            lp_last_approval_block: pool.last_lp_approval_block(),
            lp_last_approval: pool.last_lp_approval_event(),
            lp_holders_with_approvals: pool.holders_with_approvals(),
            lp_transfer_count: pool.lp_tracker.transfers.len(),
            lp_approval_count: pool.lp_tracker.approval_events.len(),
        };
        Self::from_base(
            token,
            &pool.base,
            lp_fields,
            ConcentratedPoolViewFields::default(),
            recent_activity,
        )
    }

    pub fn from_v3_pool(token: &ERC20Token, pool: &UniswapV3Pool) -> Self {
        Self::from_v3_pool_with_activity(token, pool, &[])
    }

    pub fn from_v3_pool_with_activity(
        token: &ERC20Token,
        pool: &UniswapV3Pool,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        let lp_holders = pool.lp_holders();
        let lp_holder_count = lp_holders.len();
        Self::from_base(
            token,
            &pool.base,
            LpPoolViewFields {
                lp_total_supply: pool.lp_total_supply(),
                lp_supply_known: true,
                lp_supply_status: "observed",
                lp_holder_count,
                lp_holders,
                lp_transfer_count: pool.liquidity_position_events.len(),
                ..LpPoolViewFields::default()
            },
            ConcentratedPoolViewFields {
                currency0: Some(pool.token0.clone()),
                currency1: Some(pool.token1.clone()),
                fee_tier: Some(pool.fee_tier),
                tick_spacing: Some(pool.tick_spacing),
                current_tick: pool.current_tick,
                sqrt_price_x96: pool.sqrt_price_x96.clone(),
                active_liquidity: Some(pool.active_liquidity.to_string()),
                lp_token_address: pool.position_manager_address.clone(),
                liquidity_positions: v3_liquidity_position_views(pool),
                virtual_reserves: pool
                    .last_virtual_reserves
                    .map(|reserves| VirtualReserveView {
                        token_reserve: reserves.token_reserve,
                        denom_reserve: reserves.denom_reserve,
                        token0_reserve: reserves.token0_reserve,
                        token1_reserve: reserves.token1_reserve,
                    }),
                ..ConcentratedPoolViewFields::default()
            },
            recent_activity,
        )
    }

    pub fn from_v4_pool(token: &ERC20Token, pool: &UniswapV4Pool) -> Self {
        Self::from_v4_pool_with_activity(token, pool, &[])
    }

    pub fn from_v4_pool_with_activity(
        token: &ERC20Token,
        pool: &UniswapV4Pool,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        let lp_holders = pool.lp_holders();
        let lp_holder_count = lp_holders.len();
        Self::from_base(
            token,
            &pool.base,
            LpPoolViewFields {
                lp_total_supply: pool.lp_total_supply(),
                lp_supply_known: true,
                lp_supply_status: "observed",
                lp_holder_count,
                lp_holders,
                lp_total_approved_to_routers: pool.total_approved_to_routers(),
                lp_approved_percentage: pool.lp_approved_percentage(),
                lp_last_approval_block: pool.last_lp_approval_block(),
                lp_last_approval: pool.last_lp_approval_event(),
                lp_holders_with_approvals: pool.holders_with_approvals(),
                lp_transfer_count: pool.liquidity_position_events.len(),
                lp_approval_count: pool.lp_approval_events.len(),
                ..LpPoolViewFields::default()
            },
            ConcentratedPoolViewFields {
                pool_id: Some(pool.pool_id.clone()),
                pool_manager_address: Some(pool.pool_manager_address.clone()),
                currency0: Some(pool.pool_key.currency0.clone()),
                currency1: Some(pool.pool_key.currency1.clone()),
                fee_tier: Some(pool.pool_key.fee),
                tick_spacing: Some(pool.pool_key.tick_spacing),
                hooks: Some(pool.pool_key.hooks.clone()),
                current_tick: pool.current_tick,
                sqrt_price_x96: pool.sqrt_price_x96.clone(),
                active_liquidity: Some(pool.active_liquidity.to_string()),
                lp_token_address: pool.position_manager_address.clone(),
                liquidity_positions: v4_liquidity_position_views(pool),
                virtual_reserves: pool
                    .last_virtual_reserves
                    .map(|reserves| VirtualReserveView {
                        token_reserve: reserves.token_reserve,
                        denom_reserve: reserves.denom_reserve,
                        token0_reserve: reserves.token0_reserve,
                        token1_reserve: reserves.token1_reserve,
                    }),
                ..ConcentratedPoolViewFields::default()
            },
            recent_activity,
        )
    }

    pub fn from_curve_pool(token: &ERC20Token, pool: &CurvePool) -> Self {
        Self::from_curve_pool_with_activity(token, pool, &[])
    }

    pub fn from_curve_pool_with_activity(
        token: &ERC20Token,
        pool: &CurvePool,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        Self::from_base(
            token,
            &pool.base,
            LpPoolViewFields::default(),
            ConcentratedPoolViewFields {
                lp_token_address: pool.lp_token_address.clone(),
                base_token_index: Some(pool.base_token_index),
                quote_token_index: Some(pool.quote_token_index),
                pool_tokens: pool
                    .tokens
                    .iter()
                    .map(|token| PoolComponentView {
                        symbol: token.symbol.clone(),
                        address: token.address.clone(),
                        decimals: token.decimals,
                        index: token.index,
                        weight: None,
                    })
                    .collect(),
                ..ConcentratedPoolViewFields::default()
            },
            recent_activity,
        )
    }

    pub fn from_balancer_pool(token: &ERC20Token, pool: &BalancerPool) -> Self {
        Self::from_balancer_pool_with_activity(token, pool, &[])
    }

    pub fn from_balancer_pool_with_activity(
        token: &ERC20Token,
        pool: &BalancerPool,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        Self::from_base(
            token,
            &pool.base,
            LpPoolViewFields::default(),
            ConcentratedPoolViewFields {
                pool_id: Some(pool.pool_id.clone()),
                vault_address: Some(pool.vault_address.clone()),
                swap_fee_bps: pool.swap_fee_bps,
                pool_tokens: pool
                    .tokens
                    .iter()
                    .map(|token| PoolComponentView {
                        symbol: token.symbol.clone(),
                        address: token.address.clone(),
                        decimals: token.decimals,
                        index: token.index,
                        weight: token.weight.clone(),
                    })
                    .collect(),
                ..ConcentratedPoolViewFields::default()
            },
            recent_activity,
        )
    }

    pub fn from_token_pools(token: &ERC20Token) -> Vec<Self> {
        Self::from_token_pools_with_activity(token, &[])
    }

    pub fn from_token_pools_with_activity(
        token: &ERC20Token,
        recent_activity: &[TokenBlockActivity],
    ) -> Vec<Self> {
        let mut pools = Vec::with_capacity(token.pool_count());
        pools.extend(
            token
                .v2_pools
                .values()
                .map(|pool| Self::from_v2_pool_with_activity(token, pool, recent_activity)),
        );
        pools.extend(
            token
                .v3_pools
                .values()
                .map(|pool| Self::from_v3_pool_with_activity(token, pool, recent_activity)),
        );
        pools.extend(
            token
                .v4_pools
                .values()
                .map(|pool| Self::from_v4_pool_with_activity(token, pool, recent_activity)),
        );
        pools.extend(
            token
                .curve_pools
                .values()
                .map(|pool| Self::from_curve_pool_with_activity(token, pool, recent_activity)),
        );
        pools.extend(
            token
                .balancer_pools
                .values()
                .map(|pool| Self::from_balancer_pool_with_activity(token, pool, recent_activity)),
        );
        sort_pools_by_liquidity(&mut pools);
        pools
    }

    pub fn from_token_pool_summaries(token: &ERC20Token) -> Vec<Self> {
        Self::from_token_pools(token)
            .into_iter()
            .map(Self::into_list_summary)
            .collect()
    }

    pub fn into_list_summary(mut self) -> Self {
        self.price_ratio_history.clear();
        self.liquidity_history.clear();
        self.lp_holders.clear();
        self.lp_last_approval = None;
        self.recent_denom_activity.clear();
        self
    }

    fn from_base(
        token: &ERC20Token,
        base: &BasePool,
        lp_fields: LpPoolViewFields,
        concentrated: ConcentratedPoolViewFields,
        recent_activity: &[TokenBlockActivity],
    ) -> Self {
        let total_supply = token.total_supply_scaled();
        let denom_symbol = denom_symbol(&base.identity.denom_address);
        let currency = denom_symbol
            .clone()
            .unwrap_or_else(|| base.identity.denom_address.clone());
        let liquidity_assessment = assess_denom_liquidity(
            base.denom_reserve(),
            denom_symbol.as_deref(),
            Some(&base.identity.denom_address),
            LiquidityReference::default(),
        );
        let fully_diluted_value_denom =
            total_supply.and_then(|supply| base.fully_diluted_value_denom(supply));
        let liquidity_history = liquidity_history(base);
        let liquidity_level = liquidity_assessment.level;
        let raw_pooled_token_supply_ratio =
            total_supply.and_then(|supply| base.pooled_token_supply_ratio(supply));
        let supply_ratio = display_supply_ratio(raw_pooled_token_supply_ratio);
        let reserve_quality = display_reserve_quality(&supply_ratio, liquidity_level);
        let raw_price_ratio_to_initial = base.price_ratio_to_initial();
        let price_ratio_to_initial = display_price_ratio(
            raw_price_ratio_to_initial,
            liquidity_level,
            &reserve_quality,
        );
        let price_ratio_history =
            display_price_ratio_history(&base.price_history, liquidity_level, &reserve_quality);
        let (liquidity_history, price_ratio_history) = truncate_history_at_liquidity_removal(
            &liquidity_history,
            &price_ratio_history,
            base.scam_block,
        );
        let liquidity_to_fdv_ratio = supply_ratio
            .pooled_token_supply_ratio
            .and_then(|_| total_supply.and_then(|supply| base.liquidity_to_fdv_ratio(supply)));
        let buy_tax = display_tax(base.buy_tax);
        let sell_tax = display_tax(base.sell_tax);
        let buy_tax_bucket = TaxBucket::from_percent(buy_tax);
        let sell_tax_bucket = TaxBucket::from_percent(sell_tax);
        let tax_bucket = TaxBucket::combined(buy_tax, sell_tax);
        let current_trading = current_trading_view(base, liquidity_level);
        let mut trading_status = base.trading_status();
        trading_status.effective_can_buy = current_trading.can_buy;
        trading_status.effective_can_sell = current_trading.can_sell;
        let mut risk = pool_risk(base, current_trading);
        let mut stage = current_lifecycle_view(base, current_trading);
        let explicit_liquidity_removal = base.has_liquidity_removal();
        let max_denom_reserve = max_denom_reserve(&liquidity_history, base.denom_reserve());
        let cohort_can_buy = base.state.can_buy || base.has_observed_buy();
        let cohort_can_sell = base.state.can_sell || base.has_observed_sell();
        let lp_max_holder_share = lp_fields
            .lp_holders
            .iter()
            .map(|holder| holder.share)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let classification_input = PoolClassificationInput {
            quote_symbol: Some(currency.clone()),
            denom_reserve: Some(base.denom_reserve()),
            max_denom_reserve: Some(max_denom_reserve),
            token_reserve: Some(base.token_reserve()),
            can_buy: current_trading.can_buy,
            can_sell: current_trading.can_sell,
            cohort_can_buy: Some(cohort_can_buy),
            cohort_can_sell: Some(cohort_can_sell),
            is_scam: explicit_liquidity_removal || matches!(risk.level, PoolRiskLevel::Honeypot),
            hidden_mint: token.hidden_mint_detected(),
            liquidity_removed: explicit_liquidity_removal,
            honeypot: matches!(risk.level, PoolRiskLevel::Honeypot),
            tax_bucket: Some(tax_bucket_key(tax_bucket).to_string()),
            creation_block: base.creation_block,
            creation_timestamp: base.creation_timestamp,
            has_price_history: !base.price_history.is_empty(),
            lp_approved_percentage: Some(lp_fields.lp_approved_percentage)
                .filter(|v| v.is_finite()),
            lp_max_holder_share: lp_max_holder_share.filter(|v| v.is_finite()),
            supply_ratio_status: Some(supply_ratio.status.to_string()),
            ownership_renounced: Some(token.ownership_renounced()),
        };
        let pool_classification =
            classify_pool_with_config(&classification_input, &PoolClassificationConfig::default());
        let strategy_classification = classify_pool_with_config(
            &classification_input,
            &PoolClassificationConfig {
                require_creation_data: true,
                require_price_history: true,
                ..PoolClassificationConfig::default()
            },
        );
        let derived_liquidity_removal = matches!(
            pool_classification.eligible_outcome,
            Some(EligiblePoolOutcome::LiquidityRemoval)
        );
        let liquidity_removal = explicit_liquidity_removal || derived_liquidity_removal;
        let inferred_mechanism = base.inferred_scam_mechanism();
        let scam_mechanism = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.mechanism.clone());
        let scam_mechanism_label = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.label.clone());
        let scam_mechanism_evidence = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.evidence.clone());
        let liquidity_removal_label = if explicit_liquidity_removal {
            base.scam_label
                .clone()
                .or_else(|| scam_mechanism_label.clone())
        } else if derived_liquidity_removal {
            scam_mechanism_label
                .clone()
                .or_else(|| Some("liquidity_removal (derived from reserve drop)".to_string()))
        } else {
            None
        };
        if derived_liquidity_removal && !explicit_liquidity_removal {
            risk = PoolRiskView {
                level: PoolRiskLevel::LiquidityRemoval,
                label: liquidity_removal_label.clone(),
            };
            stage = PoolLifecycle::LiquidityRemoved;
        }
        Self {
            token_address: token.contract_address.clone(),
            token_symbol: token.symbol.clone(),
            token_decimals: token.decimals,
            pool_address: base.identity.pool_address.clone(),
            protocol: base.identity.protocol.clone(),
            pool_id: concentrated.pool_id,
            pool_manager_address: concentrated.pool_manager_address,
            currency0: concentrated.currency0,
            currency1: concentrated.currency1,
            fee_tier: concentrated.fee_tier,
            tick_spacing: concentrated.tick_spacing,
            hooks: concentrated.hooks,
            current_tick: concentrated.current_tick,
            sqrt_price_x96: concentrated.sqrt_price_x96,
            active_liquidity: concentrated.active_liquidity,
            virtual_reserves: concentrated.virtual_reserves,
            vault_address: concentrated.vault_address,
            lp_token_address: concentrated.lp_token_address,
            swap_fee_bps: concentrated.swap_fee_bps,
            base_token_index: concentrated.base_token_index,
            quote_token_index: concentrated.quote_token_index,
            pool_tokens: concentrated.pool_tokens,
            denom_address: base.identity.denom_address.clone(),
            denom_symbol,
            currency,
            token_reserve: base.token_reserve(),
            denom_reserve: base.denom_reserve(),
            price: base.price(),
            initial_price: base.initial_price(),
            raw_price_ratio_to_initial,
            price_ratio_to_initial,
            price_ratio_history,
            liquidity_history,
            total_liquidity: base.state.total_liquidity,
            liquidity_level,
            liquidity_label: liquidity_level.label().to_string(),
            liquidity_rank: liquidity_level.rank(),
            liquidity_rank_score: liquidity_assessment.rank_score,
            liquidity_eth: liquidity_assessment.value_eth,
            liquidity_usd: liquidity_assessment.value_usd,
            liquidity_denom_class: liquidity_assessment.denom_class.label().to_string(),
            token_total_supply_scaled: total_supply,
            fully_diluted_value_denom,
            pooled_token_supply_ratio: supply_ratio.pooled_token_supply_ratio,
            pooled_token_supply_percent: ratio_percent(supply_ratio.pooled_token_supply_ratio),
            liquidity_to_fdv_ratio,
            liquidity_to_fdv_percent: ratio_percent(liquidity_to_fdv_ratio),
            supply_ratio_status: supply_ratio.status.to_string(),
            supply_ratio_label: supply_ratio.label,
            reserve_quality_status: reserve_quality.status.to_string(),
            reserve_quality_label: reserve_quality.label,
            can_buy: base.state.can_buy,
            can_sell: base.state.can_sell,
            effective_can_buy: current_trading.can_buy,
            effective_can_sell: current_trading.can_sell,
            economic_sellable: economic_sellable_from_tax(base.state.can_sell, sell_tax),
            has_observed_buy: base.has_observed_buy(),
            has_observed_sell: base.has_observed_sell(),
            trading_enabled: current_trading.can_buy,
            stage,
            buy_tax,
            sell_tax,
            buy_tax_bucket,
            sell_tax_bucket,
            tax_bucket,
            last_trading_failure_reason: base.last_trading_failure_reason.clone(),
            last_trading_failure_class: base.last_trading_failure_class.clone(),
            is_scam: liquidity_removal,
            scam_label: liquidity_removal_label.clone(),
            scam_mechanism,
            scam_mechanism_label,
            scam_mechanism_evidence,
            liquidity_removal,
            liquidity_removal_label,
            liquidity_removal_block: base.scam_block.or_else(|| {
                inferred_mechanism
                    .as_ref()
                    .and_then(|mechanism| mechanism.block_number)
            }),
            liquidity_removal_tx_hash: base.scam_tx_hash.clone().or_else(|| {
                inferred_mechanism
                    .as_ref()
                    .and_then(|mechanism| mechanism.tx_hash.clone())
            }),
            risk_level: risk.level,
            risk_label: risk.label,
            pool_classification,
            strategy_classification,
            creation_block: base.creation_block,
            creation_timestamp: base.creation_timestamp,
            can_buy_block: base.can_buy_block,
            latest_block_number: latest_pool_block_number(base),
            trading_status,
            trading_status_history: base.trading_status_history.clone(),
            runtime_state: base.state.clone(),
            lp_total_supply: lp_fields.lp_total_supply,
            lp_supply_known: lp_fields.lp_supply_known,
            lp_supply_status: lp_fields.lp_supply_status.to_string(),
            lp_holder_count: lp_fields.lp_holder_count,
            lp_holders: lp_fields.lp_holders,
            lp_total_approved_to_routers: lp_fields.lp_total_approved_to_routers,
            lp_approved_percentage: lp_fields.lp_approved_percentage,
            lp_last_approval_block: lp_fields.lp_last_approval_block,
            lp_last_approval: lp_fields.lp_last_approval,
            lp_holders_with_approvals: lp_fields.lp_holders_with_approvals,
            lp_transfer_count: lp_fields.lp_transfer_count,
            lp_approval_count: lp_fields.lp_approval_count,
            liquidity_positions: concentrated.liquidity_positions,
            recent_denom_activity: filter_activity_by_denom(
                recent_activity,
                &base.identity.denom_address,
            ),
        }
    }
}

fn v3_liquidity_position_views(pool: &UniswapV3Pool) -> Vec<ConcentratedLiquidityPositionView> {
    let pool_liquidity = pool.lp_total_supply();
    let mut positions = pool
        .liquidity_positions
        .values()
        .filter(|position| position.liquidity > 0)
        .map(|position| ConcentratedLiquidityPositionView {
            position_id: position.position_id.clone(),
            token_id: position_token_id_for_view(&position.position_id),
            owner: position.owner.clone(),
            position_manager_address: position
                .position_manager_address
                .clone()
                .or_else(|| pool.position_manager_address.clone()),
            liquidity: position.liquidity.to_string(),
            position_share_pct: position_share_pct(position.liquidity, pool_liquidity),
            tick_lower: position.tick_lower,
            tick_upper: position.tick_upper,
            last_update_block: position.last_update_block,
            last_update_tx: position.last_update_tx.clone(),
        })
        .collect::<Vec<_>>();
    sort_position_views(&mut positions);
    positions
}

fn v4_liquidity_position_views(pool: &UniswapV4Pool) -> Vec<ConcentratedLiquidityPositionView> {
    let pool_liquidity = pool.lp_total_supply();
    let mut positions = pool
        .liquidity_positions
        .values()
        .filter(|position| position.liquidity > 0)
        .map(|position| ConcentratedLiquidityPositionView {
            position_id: position.position_id.clone(),
            token_id: position_token_id_for_view(&position.position_id),
            owner: position.owner.clone(),
            position_manager_address: Some(position.position_manager_address.clone()),
            liquidity: position.liquidity.to_string(),
            position_share_pct: position_share_pct(position.liquidity, pool_liquidity),
            tick_lower: position.tick_lower,
            tick_upper: position.tick_upper,
            last_update_block: position.last_update_block,
            last_update_tx: position.last_update_tx.clone(),
        })
        .collect::<Vec<_>>();
    sort_position_views(&mut positions);
    positions
}

fn sort_position_views(positions: &mut [ConcentratedLiquidityPositionView]) {
    positions.sort_by(|left, right| {
        right
            .position_share_pct
            .partial_cmp(&left.position_share_pct)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.position_id.cmp(&right.position_id))
    });
}

fn position_share_pct(position_liquidity: u128, pool_liquidity: f64) -> Option<f64> {
    if position_liquidity == 0 || pool_liquidity <= 0.0 {
        return None;
    }
    Some(((position_liquidity as f64 / pool_liquidity) * 100.0).min(100.0))
}

fn position_token_id_for_view(position_id: &str) -> Option<String> {
    parse_position_token_id(position_id).map(|token_id| format!("{token_id:#x}"))
}

fn parse_position_token_id(value: &str) -> Option<U256> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        return U256::from_str_radix(hex, 16).ok();
    }
    value.parse::<U256>().ok()
}

fn filter_activity_by_denom(
    activity: &[TokenBlockActivity],
    denom_address: &str,
) -> Vec<TokenBlockActivity> {
    let denom = denom_address.to_lowercase();
    activity
        .iter()
        .filter(|block| {
            block
                .buy_volume_by_denom
                .keys()
                .any(|k| k.to_lowercase() == denom)
                || block
                    .sell_volume_by_denom
                    .keys()
                    .any(|k| k.to_lowercase() == denom)
                || block.total_bribe_eth > 0.0
        })
        .cloned()
        .collect()
}

pub fn denom_symbol(address: &str) -> Option<String> {
    address
        .parse::<Address>()
        .ok()
        .and_then(get_token_symbol)
        .map(str::to_string)
}

fn latest_pool_block_number(base: &BasePool) -> Option<u64> {
    base.latest_block_number
        .or_else(|| nonzero_block(base.state.last_update_block))
        .or_else(|| nonzero_block(base.state.last_sync_block))
        .or(base.can_buy_block)
        .or(base.creation_block)
}

fn nonzero_block(block: u64) -> Option<u64> {
    (block > 0).then_some(block)
}

pub(super) fn sort_pools_by_liquidity(pools: &mut [PoolView]) {
    pools.sort_by(|left, right| {
        pool_liquidity_cmp(left, right)
            .then_with(|| left.token_address.cmp(&right.token_address))
            .then_with(|| left.pool_address.cmp(&right.pool_address))
    });
}

fn pool_liquidity_cmp(left: &PoolView, right: &PoolView) -> Ordering {
    right
        .liquidity_rank
        .cmp(&left.liquidity_rank)
        .then_with(|| {
            right
                .liquidity_rank_score
                .partial_cmp(&left.liquidity_rank_score)
                .unwrap_or(Ordering::Equal)
        })
}
