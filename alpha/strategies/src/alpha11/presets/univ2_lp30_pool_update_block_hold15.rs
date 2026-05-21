use alloy_primitives::U256;
use eth_alpha_core::{amount::Amount, ids::StrategyName};
use rust_decimal::Decimal;

use crate::{
    alpha11::{initial_entry_bankroll_wei, Alpha11Config, HOLD15_STRATEGY_NAME},
    baseline::snipe_all::SnipeAllConfig,
};

pub const HOLD_BLOCKS: u64 = 15;
pub const BUY_WEI: u64 = 10_000_000_000_000_000;

pub fn config() -> Alpha11Config {
    Alpha11Config::new(snipe_all_config())
}

pub fn snipe_all_config() -> SnipeAllConfig {
    SnipeAllConfig {
        strategy_name: StrategyName(HOLD15_STRATEGY_NAME.to_string()),
        buy_amount: Amount {
            raw: U256::from(BUY_WEI),
            decimals: 18,
        },
        min_sell_pool_denom_reserve: Decimal::ZERO,
        entry_bankroll_wei: Some(initial_entry_bankroll_wei()),
        exit_on_liquidity_removal: true,
        exit_on_tax: true,
        exit_on_lp_approval: true,
        exit_on_critical_lp_approval_only: false,
        exit_on_scam: true,
        allowed_protocols: vec!["UNISWAP-V2".to_string()],
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(
            crate::shared_rules::lp_approval::DEFAULT_GATE_MIN_APPROVED_PCT.into(),
        ),
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        max_hold_blocks: Some(HOLD_BLOCKS),
        ..SnipeAllConfig::default()
    }
}
