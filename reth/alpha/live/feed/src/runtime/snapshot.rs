use eth_token::erc20::ERC20Token;
use eth_token::pools::{PoolLifecycle, UniswapV2Pool};
use serde::{Deserialize, Serialize};

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
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub pools: Vec<LiveTokenPoolSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenPoolSnapshot {
    pub pool_address: String,
    pub token_address: String,
    pub protocol: String,
    pub denom_address: String,
    pub denom_symbol: String,
    pub token_reserve: f64,
    pub denom_reserve: f64,
    pub price: f64,
    pub total_liquidity: f64,
    pub can_buy: bool,
    pub can_sell: bool,
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<u64>,
    pub trading_enabled_tx: Option<String>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub creation_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub lifecycle: String,
    pub control_addresses: Vec<String>,
    pub lp_tokens_approved_percentage: Option<f64>,
}

impl LiveTokenSnapshot {
    pub fn from_token(token: &ERC20Token) -> Self {
        let mut pools = token
            .v2_pools
            .values()
            .map(|pool| LiveTokenPoolSnapshot::from_pool(&token.contract_address, pool))
            .collect::<Vec<_>>();
        pools.sort_by(|left, right| left.pool_address.cmp(&right.pool_address));

        let buy_tax = pools.iter().find_map(|pool| pool.buy_tax);
        let sell_tax = pools.iter().find_map(|pool| pool.sell_tax);

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
            is_scam: token.is_scam(),
            scam_label: token.scam_label(),
            buy_tax,
            sell_tax,
            pools,
        }
    }
}

impl LiveTokenPoolSnapshot {
    pub fn from_pool(token_address: &str, pool: &UniswapV2Pool) -> Self {
        Self {
            pool_address: pool.base.identity.pool_address.clone(),
            token_address: token_address.to_string(),
            protocol: pool.base.identity.protocol.clone(),
            denom_address: pool.base.identity.denom_address.clone(),
            denom_symbol: pool.base.identity.denom_address.clone(),
            token_reserve: pool.base.token_reserve(),
            denom_reserve: pool.base.denom_reserve(),
            price: pool.base.price(),
            total_liquidity: pool.base.state.total_liquidity,
            can_buy: pool.base.state.can_buy,
            can_sell: pool.base.state.can_sell,
            trading_enabled: pool.base.trading_enabled(),
            trading_enabled_block: pool.base.can_buy_block,
            trading_enabled_tx: pool.base.can_buy_tx.clone(),
            buy_tax: pool.base.buy_tax,
            sell_tax: pool.base.sell_tax,
            is_scam: pool.base.is_scam(),
            scam_label: pool.base.scam_label.clone(),
            creation_block: pool.base.creation_block,
            latest_block_number: pool.base.latest_block_number,
            lifecycle: lifecycle_label(pool.base.state.lifecycle),
            control_addresses: sorted_strings(pool.base.token_control_addresses.iter().cloned()),
            lp_tokens_approved_percentage: Some(pool.lp_approved_percentage()),
        }
    }
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
        PoolLifecycle::Active => "ACTIVE",
        PoolLifecycle::Scam => "SCAM",
        PoolLifecycle::Evicted => "EVICTED",
    }
    .to_string()
}
