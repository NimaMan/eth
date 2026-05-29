use eth_pool_classification::{
    classify_pool_with_config, EligiblePoolOutcome, PoolClassificationConfig,
};

use eth_token::erc20::ERC20Token;
use eth_token::pools::{
    BasePool, PoolLifecycle, PoolRuntimeState, UniswapV2Pool, UniswapV3Pool, UniswapV4Pool,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenSnapshot {
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
    pub creation_block: Option<u64>,
    pub creation_timestamp: Option<u64>,
    pub creation_tx: Option<String>,
    pub creator_address: Option<String>,
    pub current_owner: Option<String>,
    pub tax_setter_addresses: Vec<String>,
    pub ownership_renounced: bool,
    pub latest_activity_block: Option<u64>,
    pub latest_activity_timestamp: Option<u64>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub hidden_mint_detected: bool,
    pub hidden_mint_block: Option<u64>,
    pub hidden_mint_tx: Option<String>,
    pub liquidity_removal_pool_count: usize,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub pools: Vec<LiveTokenPoolSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenPoolSnapshot {
    pub pool_address: String,
    pub token_address: String,
    pub protocol: String,
    pub pool_id: Option<String>,
    pub pool_manager_address: Option<String>,
    pub currency0: Option<String>,
    pub currency1: Option<String>,
    pub fee_tier: Option<u32>,
    pub tick_spacing: Option<i32>,
    pub hooks: Option<String>,
    pub denom_address: String,
    pub denom_symbol: String,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub price: f64,
    pub initial_price: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub total_liquidity: f64,
    pub can_buy: bool,
    pub can_sell: bool,
    #[serde(default)]
    pub has_observed_buy: bool,
    #[serde(default)]
    pub has_observed_sell: bool,
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<u64>,
    pub trading_enabled_tx: Option<String>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub scam_mechanism_label: Option<String>,
    pub scam_mechanism_evidence: Option<Value>,
    pub liquidity_removal: bool,
    pub liquidity_removal_label: Option<String>,
    pub liquidity_removal_block: Option<u64>,
    pub liquidity_removal_tx_hash: Option<String>,
    pub creation_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub lifecycle: String,
    pub runtime_state: PoolRuntimeState,
    pub control_addresses: Vec<String>,
    pub lp_tokens_approved_percentage: Option<f64>,
    pub lp_last_approval_block: Option<u64>,
    pub lp_last_approval: Option<Value>,
    pub lp_approval_count: u64,
}

impl LiveTokenSnapshot {
    pub fn from_token(token: &ERC20Token) -> Self {
        let mut pools = Vec::with_capacity(token.pool_count());
        pools.extend(
            token
                .v2_pools
                .values()
                .map(|pool| LiveTokenPoolSnapshot::from_pool(&token.contract_address, pool)),
        );
        pools.extend(
            token
                .v3_pools
                .values()
                .map(|pool| LiveTokenPoolSnapshot::from_v3_pool(&token.contract_address, pool)),
        );
        pools.extend(
            token
                .v4_pools
                .values()
                .map(|pool| LiveTokenPoolSnapshot::from_v4_pool(&token.contract_address, pool)),
        );
        pools.extend(token.curve_pools.values().map(|pool| {
            LiveTokenPoolSnapshot::from_base(
                &token.contract_address,
                &pool.base,
                LiveTokenPoolLpSnapshot::default(),
            )
        }));
        pools.extend(token.balancer_pools.values().map(|pool| {
            LiveTokenPoolSnapshot::from_base(
                &token.contract_address,
                &pool.base,
                LiveTokenPoolLpSnapshot::default(),
            )
        }));
        pools.sort_by(|left, right| left.pool_address.cmp(&right.pool_address));

        let buy_tax = pools.iter().find_map(|pool| pool.buy_tax);
        let sell_tax = pools.iter().find_map(|pool| pool.sell_tax);
        let liquidity_removal_pool_count =
            pools.iter().filter(|pool| pool.liquidity_removal).count();
        let is_scam = token.is_scam() || liquidity_removal_pool_count > 0;
        let scam_label = token.scam_label().or_else(|| {
            (liquidity_removal_pool_count > 0).then(|| "liquidity_removal".to_string())
        });
        let scam_mechanism = token
            .scam_mechanism()
            .or_else(|| pools.iter().find_map(|pool| pool.scam_mechanism.clone()));
        let scam_mechanism_label = token.scam_mechanism_label().or_else(|| {
            pools
                .iter()
                .find_map(|pool| pool.scam_mechanism_label.clone())
        });

        Self {
            contract_address: token.contract_address.clone(),
            name: token.name.clone(),
            symbol: token.symbol.clone(),
            decimals: token.decimals,
            total_supply: token.total_supply.clone(),
            creation_block: token.creation_block,
            creation_timestamp: token.creation_timestamp,
            creation_tx: token.creation_tx.clone(),
            creator_address: token.creator_address.clone(),
            current_owner: token.current_owner(),
            tax_setter_addresses: sorted_strings(token.token_control_addresses.iter().cloned()),
            ownership_renounced: token.ownership_renounced(),
            latest_activity_block: token.latest_block_number,
            latest_activity_timestamp: token.latest_block_timestamp,
            is_scam,
            scam_label,
            scam_mechanism,
            scam_mechanism_label,
            hidden_mint_detected: token.hidden_mint_detected(),
            hidden_mint_block: token.hidden_mint_block(),
            hidden_mint_tx: token.hidden_mint_tx(),
            liquidity_removal_pool_count,
            buy_tax,
            sell_tax,
            pools,
        }
    }
}

impl LiveTokenPoolSnapshot {
    pub fn from_pool(token_address: &str, pool: &UniswapV2Pool) -> Self {
        Self::from_base(
            token_address,
            &pool.base,
            LiveTokenPoolLpSnapshot {
                approved_percentage: Some(pool.lp_approved_percentage()),
                last_approval_block: pool.last_lp_approval_block(),
                last_approval: pool.last_lp_approval_event(),
                approval_count: pool.lp_tracker.approval_events.len() as u64,
            },
        )
    }

    pub fn from_v4_pool(token_address: &str, pool: &UniswapV4Pool) -> Self {
        let mut snapshot = Self::from_base(
            token_address,
            &pool.base,
            LiveTokenPoolLpSnapshot {
                approved_percentage: Some(pool.lp_approved_percentage()),
                last_approval_block: pool.last_lp_approval_block(),
                last_approval: pool.last_lp_approval_event(),
                approval_count: pool.lp_approval_events.len() as u64,
            },
        );
        snapshot.pool_id = Some(pool.pool_id.clone());
        snapshot.pool_manager_address = Some(pool.pool_manager_address.clone());
        snapshot.currency0 = Some(pool.pool_key.currency0.clone());
        snapshot.currency1 = Some(pool.pool_key.currency1.clone());
        snapshot.fee_tier = Some(pool.pool_key.fee);
        snapshot.tick_spacing = Some(pool.pool_key.tick_spacing);
        snapshot.hooks = Some(pool.pool_key.hooks.clone());
        snapshot
    }

    pub fn from_v3_pool(token_address: &str, pool: &UniswapV3Pool) -> Self {
        let mut snapshot = Self::from_base(
            token_address,
            &pool.base,
            LiveTokenPoolLpSnapshot::default(),
        );
        snapshot.currency0 = Some(pool.token0.clone());
        snapshot.currency1 = Some(pool.token1.clone());
        snapshot.fee_tier = Some(pool.fee_tier);
        snapshot.tick_spacing = Some(pool.tick_spacing);
        snapshot
    }

    pub fn from_base(token_address: &str, pool: &BasePool, lp: LiveTokenPoolLpSnapshot) -> Self {
        let explicit_liquidity_removal = pool.has_liquidity_removal();
        let classification = classify_pool_with_config(
            &pool.classification_input(),
            &PoolClassificationConfig::default(),
        );
        let derived_liquidity_removal = matches!(
            classification.eligible_outcome,
            Some(EligiblePoolOutcome::LiquidityRemoval)
        );
        let inferred_mechanism = pool.inferred_scam_mechanism();
        let scam_mechanism = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.mechanism.clone());
        let scam_mechanism_label = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.label.clone());
        let scam_mechanism_evidence = inferred_mechanism
            .as_ref()
            .map(|mechanism| mechanism.evidence.clone());
        let liquidity_removal =
            explicit_liquidity_removal || derived_liquidity_removal || inferred_mechanism.is_some();
        let liquidity_removal_label = if explicit_liquidity_removal {
            pool.scam_label
                .clone()
                .or_else(|| scam_mechanism_label.clone())
        } else if derived_liquidity_removal {
            scam_mechanism_label
                .clone()
                .or_else(|| Some("liquidity_removal (derived from reserve drop)".to_string()))
        } else if inferred_mechanism.is_some() {
            scam_mechanism_label.clone()
        } else {
            None
        };
        let lifecycle = if liquidity_removal && !explicit_liquidity_removal {
            PoolLifecycle::LiquidityRemoved
        } else {
            pool.state.lifecycle
        };

        Self {
            pool_address: pool.identity.pool_address.clone(),
            token_address: token_address.to_string(),
            protocol: pool.identity.protocol.clone(),
            pool_id: None,
            pool_manager_address: None,
            currency0: None,
            currency1: None,
            fee_tier: None,
            tick_spacing: None,
            hooks: None,
            denom_address: pool.identity.denom_address.clone(),
            denom_symbol: eth_token::pools::classification::quote_symbol_for_denom_address(
                &pool.identity.denom_address,
            )
            .unwrap_or(pool.identity.denom_address.as_str())
            .to_string(),
            token_reserve: pool.token_reserve(),
            denom_reserve: pool.denom_reserve(),
            price: pool.price(),
            initial_price: pool.initial_price(),
            price_ratio_to_initial: pool.price_ratio_to_initial(),
            total_liquidity: pool.state.total_liquidity,
            can_buy: pool.state.can_buy,
            can_sell: pool.state.can_sell,
            has_observed_buy: pool.has_observed_buy(),
            has_observed_sell: pool.has_observed_sell(),
            trading_enabled: pool.trading_enabled(),
            trading_enabled_block: pool.can_buy_block,
            trading_enabled_tx: pool.can_buy_tx.clone(),
            buy_tax: pool.buy_tax,
            sell_tax: pool.sell_tax,
            is_scam: liquidity_removal,
            scam_label: liquidity_removal_label.clone(),
            scam_mechanism,
            scam_mechanism_label,
            scam_mechanism_evidence,
            liquidity_removal,
            liquidity_removal_label,
            liquidity_removal_block: pool.scam_block.or_else(|| {
                inferred_mechanism
                    .as_ref()
                    .and_then(|mechanism| mechanism.block_number)
            }),
            liquidity_removal_tx_hash: pool.scam_tx_hash.clone().or_else(|| {
                inferred_mechanism
                    .as_ref()
                    .and_then(|mechanism| mechanism.tx_hash.clone())
            }),
            creation_block: pool.creation_block,
            latest_block_number: pool.latest_block_number,
            lifecycle: lifecycle_label(lifecycle),
            runtime_state: pool.state.clone(),
            control_addresses: sorted_strings(pool.token_control_addresses.iter().cloned()),
            lp_tokens_approved_percentage: lp.approved_percentage,
            lp_last_approval_block: lp.last_approval_block,
            lp_last_approval: lp.last_approval,
            lp_approval_count: lp.approval_count,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LiveTokenPoolLpSnapshot {
    pub approved_percentage: Option<f64>,
    pub last_approval_block: Option<u64>,
    pub last_approval: Option<Value>,
    pub approval_count: u64,
}


fn sorted_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn lifecycle_label(lifecycle: PoolLifecycle) -> String {
    match lifecycle {
        PoolLifecycle::Discovered => "DISCOVERED",
        PoolLifecycle::LiquidityDeposited => "LIQUIDITY_DEPOSITED",
        PoolLifecycle::Trading => "TRADING",
        PoolLifecycle::CannotSell => "CANNOT_SELL",
        PoolLifecycle::Dust => "DUST",
        PoolLifecycle::Drained => "DRAINED",
        PoolLifecycle::Active => "ACTIVE",
        PoolLifecycle::LiquidityRemoved => "LIQUIDITY_REMOVED",
        PoolLifecycle::Evicted => "EVICTED",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth_token::erc20::ERC20TokenMetadata;
    use eth_token::pools::base::BasePoolConfig;

    const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

    fn token_with_drained_pool() -> ERC20Token {
        let mut token = ERC20Token::new(ERC20TokenMetadata::new(
            "0x1000000000000000000000000000000000000001",
            "Example",
            "EX",
            18,
            "1000000000000000000000000",
        ));
        let pool = token.create_uniswap_v2_pool(
            "0x2000000000000000000000000000000000000002",
            WETH_ADDRESS,
            BasePoolConfig {
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                ..BasePoolConfig::new(18)
            },
            [] as [&str; 0],
        );
        pool.base.creation_block = Some(100);
        pool.base.creation_timestamp = Some(1_700);
        pool.base
            .update_reserves(1_000.0, 1.2, 100, 1_700, "0xSYNC1");
        pool.base.state.record_swap(0.1, 25.0, 0.1, 25.0);
        pool.base
            .set_simulated_buy_status(true, 101, "0xBUY", 1_710);
        pool.base
            .set_simulated_sell_status(true, None, None, 102, "0xSELL");
        pool.base
            .update_reserves(1_000.0, 0.03, 120, 1_940, "0xDRAIN");
        token
    }

    #[test]
    fn live_snapshot_derives_liquidity_removal_from_reserve_drop() {
        let snapshot = LiveTokenSnapshot::from_token(&token_with_drained_pool());
        let pool = snapshot
            .pools
            .iter()
            .find(|pool| pool.pool_address == "0x2000000000000000000000000000000000000002")
            .expect("pool snapshot");

        assert!(pool.liquidity_removal);
        assert!(pool.is_scam);
        assert_eq!(pool.lifecycle, "LIQUIDITY_REMOVED");
        assert_eq!(snapshot.liquidity_removal_pool_count, 1);
        assert!(snapshot.is_scam);
        assert_eq!(
            snapshot.scam_label.as_deref(),
            Some("Unknown Reserve Drain")
        );
        assert_eq!(
            snapshot.scam_mechanism.as_deref(),
            Some("unknown_reserve_drain")
        );
    }
}
