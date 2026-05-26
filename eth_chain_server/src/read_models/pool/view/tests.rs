use eth_pool_classification::{EligiblePoolOutcome, PoolCohort};
use eth_price::liquidity::{USDC_ADDRESS, WETH_ADDRESS};
use eth_token::erc20::{ERC20Token, ERC20TokenMetadata};
use eth_token::pools::BasePoolConfig;

use super::display::DisplayReserveQuality;
use super::*;

#[test]
fn display_tax_hides_failed_simulation_sentinel() {
    assert_eq!(display_tax(Some(-1.0)), None);
    assert_eq!(display_tax(Some(2.99)), Some(2.99));
}

#[test]
fn display_price_ratio_zeroes_dust_or_drained_liquidity() {
    let ok_reserve_quality = DisplayReserveQuality {
        status: "ok",
        label: None,
        price_ratio_trustworthy: true,
    };
    assert_eq!(
        display_price_ratio(Some(1000.0), PoolLiquidityLevel::Dust, &ok_reserve_quality),
        Some(0.0)
    );
    assert_eq!(
        display_price_ratio(
            Some(1000.0),
            PoolLiquidityLevel::Drained,
            &ok_reserve_quality
        ),
        Some(0.0)
    );
    assert_eq!(
        display_price_ratio(
            Some(1000.0),
            PoolLiquidityLevel::Unknown,
            &ok_reserve_quality
        ),
        Some(1000.0)
    );
    assert_eq!(
        display_price_ratio(Some(10.0), PoolLiquidityLevel::Liquid, &ok_reserve_quality),
        Some(10.0)
    );
    assert_eq!(
        display_price_ratio(None, PoolLiquidityLevel::Liquid, &ok_reserve_quality),
        None
    );
    assert_eq!(
        display_price_ratio(Some(-5.0), PoolLiquidityLevel::Liquid, &ok_reserve_quality),
        None
    );
}

#[test]
fn display_price_ratio_zeroes_token_reserve_dust_even_with_liquid_denom() {
    let supply_ratio = display_supply_ratio(Some(6.9e-7));
    let reserve_quality = display_reserve_quality(&supply_ratio, PoolLiquidityLevel::Liquid);

    assert_eq!(reserve_quality.status, "token_reserve_dust");
    assert_eq!(
        display_price_ratio(
            Some(2_128_397.0),
            PoolLiquidityLevel::Liquid,
            &reserve_quality
        ),
        Some(0.0)
    );
    assert!(display_price_ratio_history(
        &[(1, 1.0), (2, 2_128_397.0)],
        PoolLiquidityLevel::Liquid,
        &reserve_quality,
    )
    .is_empty());
}

#[test]
fn display_reserve_quality_accepts_meaningful_token_reserve() {
    let supply_ratio = display_supply_ratio(Some(0.01));
    let reserve_quality = display_reserve_quality(&supply_ratio, PoolLiquidityLevel::Liquid);

    assert_eq!(reserve_quality.status, "ok");
    assert!(reserve_quality.price_ratio_trustworthy);
    assert_eq!(
        display_price_ratio(Some(3.0), PoolLiquidityLevel::Liquid, &reserve_quality),
        Some(3.0)
    );
}

#[test]
fn liquidity_level_marks_unknown_quote_assets() {
    assert_eq!(
        assess_denom_liquidity(1_000.0, Some("WETH"), None, LiquidityReference::default()).level,
        PoolLiquidityLevel::Liquid
    );
    assert_eq!(
        assess_denom_liquidity(
            1_000_000.0,
            None,
            Some("0xunknown"),
            LiquidityReference::default()
        )
        .level,
        PoolLiquidityLevel::Unknown
    );
}

#[test]
fn token_pools_are_ranked_by_normalized_liquidity() {
    let token_address = "0x1111111111111111111111111111111111111111";
    let weth_pool_address = "0x2222222222222222222222222222222222222222";
    let usdc_pool_address = "0x3333333333333333333333333333333333333333";
    let mut token = ERC20Token::new(ERC20TokenMetadata::new(
        token_address,
        "Token",
        "TOK",
        18,
        "1000000000000000000000000",
    ));

    let mut usdc_pool = UniswapV2Pool::new(
        usdc_pool_address,
        token_address,
        USDC_ADDRESS,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    usdc_pool
        .base
        .update_reserves(1_000.0, 2_000.0, 10, 100, "0xtx1");
    token
        .v2_pools
        .insert(usdc_pool_address.to_string(), usdc_pool);

    let mut weth_pool = UniswapV2Pool::new(
        weth_pool_address,
        token_address,
        WETH_ADDRESS,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    weth_pool
        .base
        .update_reserves(1_000.0, 1.0, 11, 112, "0xtx2");
    token
        .v2_pools
        .insert(weth_pool_address.to_string(), weth_pool);

    let pools = PoolView::from_token_pools(&token);

    assert_eq!(pools[0].pool_address, weth_pool_address);
    assert_eq!(pools[0].liquidity_usd, Some(3_000.0));
    assert_eq!(pools[1].pool_address, usdc_pool_address);
    assert_eq!(pools[1].liquidity_usd, Some(2_000.0));
}

#[test]
fn derived_liquidity_removal_stays_in_eligible_cohort() {
    let token_address = "0x1111111111111111111111111111111111111111";
    let pool_address = "0x2222222222222222222222222222222222222222";
    let mut token = ERC20Token::new(ERC20TokenMetadata::new(
        token_address,
        "Token",
        "TOK",
        18,
        "1000000000000000000000000",
    ));
    let mut pool = UniswapV2Pool::new(
        pool_address,
        token_address,
        WETH_ADDRESS,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.creation_block = Some(10);
    pool.base.creation_timestamp = Some(100);
    pool.base.update_reserves(1_000.0, 1.2, 10, 100, "0xadd");
    pool.base.state.record_swap(0.1, 25.0, 0.1, 25.0);
    pool.base
        .update_reserves(1_000_000.0, 0.03, 11, 112, "0xremove");

    let pool_view = PoolView::from_v2_pool(&token, &pool);

    assert!(pool_view.liquidity_removal);
    assert!(pool_view.is_scam);
    assert_eq!(pool_view.risk_level, PoolRiskLevel::LiquidityRemoval);
    assert_eq!(pool_view.stage, PoolLifecycle::LiquidityRemoved);
    assert_eq!(pool_view.pool_classification.cohort, PoolCohort::Eligible);
    assert_eq!(pool_view.pool_classification.reason, None);
    assert_eq!(
        pool_view.pool_classification.eligible_outcome,
        Some(EligiblePoolOutcome::LiquidityRemoval)
    );

    token.v2_pools.insert(pool_address.to_string(), pool);
    let token_view = crate::read_models::token::TokenView::from_token(&token, None);
    assert_eq!(token_view.liquidity_removal_pool_count, 1);
    assert!(token_view.is_scam);
    assert_eq!(
        token_view
            .contract_analysis
            .pools
            .liquidity_removal_pool_count,
        1
    );
    assert!(token_view
        .contract_analysis
        .behavior_flags
        .contains(&eth_token::contract_analysis::BehaviorFlagKind::PoolLiquidityRemovalEvidence));
}

#[test]
fn v2_lp_supply_status_marks_mid_range_unknown_supply() {
    let token_address = "0x1111111111111111111111111111111111111111";
    let pool_address = "0x2222222222222222222222222222222222222222";
    let token = ERC20Token::new(ERC20TokenMetadata::new(
        token_address,
        "Token",
        "TOK",
        18,
        "1000000000000000000000000",
    ));
    let mut pool = UniswapV2Pool::new(
        pool_address,
        token_address,
        WETH_ADDRESS,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.update_reserves(1_000.0, 1.0, 10, 100, "0xtx1");

    let unknown = PoolView::from_v2_pool(&token, &pool);
    assert_eq!(unknown.lp_total_supply, 0.0);
    assert!(!unknown.lp_supply_known);
    assert_eq!(unknown.lp_supply_status, "unknown");

    pool.lp_tracker.record_transfer(
        "0x0000000000000000000000000000000000000000",
        "0xholder",
        100.0,
        11,
        "0xmint",
        Some(1),
    );

    let observed = PoolView::from_v2_pool(&token, &pool);
    assert_eq!(observed.lp_total_supply, 100.0);
    assert!(observed.lp_supply_known);
    assert_eq!(observed.lp_supply_status, "observed");
}

#[test]
fn v2_lp_supply_status_keeps_mid_range_non_mint_transfer_unknown() {
    let token_address = "0x1111111111111111111111111111111111111111";
    let pool_address = "0x2222222222222222222222222222222222222222";
    let token = ERC20Token::new(ERC20TokenMetadata::new(
        token_address,
        "Token",
        "TOK",
        18,
        "1000000000000000000000000",
    ));
    let mut pool = UniswapV2Pool::new(
        pool_address,
        token_address,
        WETH_ADDRESS,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.update_reserves(1_000.0, 1.0, 10, 100, "0xtx1");
    pool.lp_tracker.record_transfer(
        "0xholder",
        "0x000000000000000000000000000000000000dead",
        100.0,
        11,
        "0xburn",
        Some(1),
    );

    let view = PoolView::from_v2_pool(&token, &pool);
    assert_eq!(view.lp_total_supply, 0.0);
    assert!(!view.lp_supply_known);
    assert_eq!(view.lp_supply_status, "unknown");
}

#[test]
fn current_trading_view_masks_dust_or_drained_pool_flags() {
    let mut pool = UniswapV2Pool::new(
        "0xpool",
        "0xtoken",
        "0xdenom",
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.state.can_buy = true;
    pool.base.state.can_sell = true;

    let liquid = current_trading_view(&pool.base, PoolLiquidityLevel::Liquid);
    assert!(liquid.can_buy);
    assert!(liquid.can_sell);

    pool.base.state.lifecycle = PoolLifecycle::Dust;
    let dust = current_trading_view(&pool.base, PoolLiquidityLevel::Dust);
    assert!(!dust.can_buy);
    assert!(!dust.can_sell);

    pool.base.state.lifecycle = PoolLifecycle::Drained;
    let drained = current_trading_view(&pool.base, PoolLiquidityLevel::Drained);
    assert!(!drained.can_buy);
    assert!(!drained.can_sell);
}

#[test]
fn current_trading_view_keeps_observed_chain_swaps_separate() {
    let mut pool = UniswapV2Pool::new(
        "0xpool",
        "0xtoken",
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.update_reserves(100.0, 1.0, 10, 1_700, "0xSYNC");
    pool.base.state.can_buy = true;
    pool.base.state.can_sell = false;
    pool.base.state.record_swap(0.0, 25.0, 0.1, 0.0);

    let current = current_trading_view(&pool.base, PoolLiquidityLevel::Liquid);
    assert!(current.can_buy);
    assert!(!current.can_sell);
    assert!(pool.base.has_observed_sell());
    assert!(!pool.base.state.can_sell);
    assert_eq!(
        current_lifecycle_view(&pool.base, current),
        PoolLifecycle::CannotSell
    );
    assert_eq!(
        pool_risk(&pool.base, current).level,
        PoolRiskLevel::Honeypot
    );
}

#[test]
fn display_supply_ratio_rejects_reserve_above_total_supply() {
    let ratio = display_supply_ratio(Some(10_000.0));
    assert_eq!(ratio.pooled_token_supply_ratio, None);
    assert_eq!(ratio.status, "inconsistent");
    assert_eq!(
        ratio.label,
        Some("pool_reserve_exceeds_total_supply".to_string())
    );
}

#[test]
fn latest_pool_block_uses_runtime_update_when_control_block_is_empty() {
    let mut pool = UniswapV2Pool::new(
        "0xpool",
        "0xtoken",
        "0xdenom",
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.state.last_update_block = 123;

    assert_eq!(latest_pool_block_number(&pool.base), Some(123));

    pool.base.latest_block_number = Some(456);
    assert_eq!(latest_pool_block_number(&pool.base), Some(456));
}

#[test]
fn pool_list_summary_omits_heavy_detail_fields() {
    let token_address = "0x1111111111111111111111111111111111111111";
    let pool_address = "0x2222222222222222222222222222222222222222";
    let denom_address = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    let mut token = ERC20Token::new(ERC20TokenMetadata::new(
        token_address,
        "Token",
        "TOK",
        18,
        "1000000000000000000000000",
    ));
    let mut pool = UniswapV2Pool::new(
        pool_address,
        token_address,
        denom_address,
        BasePoolConfig::new(18),
        std::iter::empty::<&str>(),
    );
    pool.base.update_reserves(1_000.0, 1.0, 10, 100, "0xtx1");
    pool.base.update_reserves(900.0, 1.2, 11, 112, "0xtx2");
    token.v2_pools.insert(pool_address.to_string(), pool);

    let full = PoolView::from_token_pools(&token);
    assert!(!full[0].price_ratio_history.is_empty());
    assert!(!full[0].liquidity_history.is_empty());

    let summary = PoolView::from_token_pool_summaries(&token);
    assert!(summary[0].price_ratio_history.is_empty());
    assert!(summary[0].liquidity_history.is_empty());
    assert!(summary[0].lp_holders.is_empty());
    assert!(summary[0].lp_last_approval.is_none());
}
