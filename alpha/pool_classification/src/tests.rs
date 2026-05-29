use super::*;

fn input() -> PoolClassificationInput {
    PoolClassificationInput::new(Some("WETH".to_string()), Some(1.0), true, true, false)
        .with_creation_data(Some(100), Some(1_700_000_000))
        .with_price_history(true)
}

#[test]
fn accepts_weth_pool_at_default_floor() {
    let decision = classify_pool(&input());

    assert!(decision.eligible);
    assert_eq!(decision.reason, None);
    assert_eq!(decision.min_liquidity, Some(ETH_ELIGIBLE_LIQUIDITY));
    assert_eq!(decision.liquidity_class, LiquidityClass::Eligible);
}

#[test]
fn rejects_weth_pool_below_default_floor() {
    let mut input = input();
    input.denom_reserve = Some(0.49);
    input.max_denom_reserve = input.denom_reserve;

    let decision = classify_pool(&input);

    assert!(!decision.eligible);
    assert_eq!(decision.reason, Some(NonEligibleReason::LowLiquidity));
    assert_eq!(decision.liquidity_class, LiquidityClass::Low);
}

#[test]
fn rejects_unsupported_quote_before_liquidity_checks() {
    let mut input = input();
    input.quote_symbol = Some("WBTC".to_string());
    input.denom_reserve = Some(10_000.0);

    let decision = classify_pool(&input);

    assert_eq!(
        decision.reason,
        Some(NonEligibleReason::UnsupportedCurrency)
    );
    assert_eq!(
        decision.liquidity_class,
        LiquidityClass::UnsupportedCurrency
    );
}

#[test]
fn uses_stable_floor_for_usdc_usdt_and_dai() {
    let mut input = input();
    input.quote_symbol = Some("USDC".to_string());
    input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY);
    input.max_denom_reserve = input.denom_reserve;
    assert!(classify_pool(&input).eligible);

    input.quote_symbol = Some("USDT".to_string());
    input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY - 0.01);
    input.max_denom_reserve = input.denom_reserve;
    assert_eq!(
        classify_pool(&input).reason,
        Some(NonEligibleReason::LowLiquidity)
    );

    input.quote_symbol = Some("DAI".to_string());
    input.denom_reserve = Some(STABLE_ELIGIBLE_LIQUIDITY);
    input.max_denom_reserve = input.denom_reserve;
    assert!(classify_pool(&input).eligible);
}

#[test]
fn strict_config_requires_creation_and_price_history() {
    let strict = PoolClassificationConfig {
        require_creation_data: true,
        require_price_history: true,
        ..PoolClassificationConfig::default()
    };

    let mut input = input();
    input.creation_timestamp = None;
    input.has_price_history = false;

    let decision = classify_pool_with_config(&input, &strict);
    assert_eq!(
        decision.reason,
        Some(NonEligibleReason::MissingCreationData)
    );

    input.creation_timestamp = Some(1_700_000_000);
    let decision = classify_pool_with_config(&input, &strict);
    assert_eq!(decision.reason, Some(NonEligibleReason::MissingPriceData));
}

#[test]
fn current_liquidity_drop_stays_eligible_as_liquidity_removal() {
    let mut input = input();
    input.denom_reserve = Some(0.03);
    input.max_denom_reserve = Some(1.2);

    let decision = classify_pool(&input);

    assert!(decision.eligible);
    assert_eq!(decision.reason, None);
    assert_eq!(decision.cohort, PoolCohort::Eligible);
    assert_eq!(decision.category, PoolCategory::EligibleRisk);
    assert_eq!(
        decision.eligible_outcome,
        Some(EligiblePoolOutcome::LiquidityRemoval)
    );
    assert_eq!(decision.eligibility_liquidity, Some(1.2));
    assert_eq!(decision.liquidity_class, LiquidityClass::Low);
}

#[test]
fn current_liquidity_minor_dip_stays_eligible_as_current_low_liquidity() {
    let mut input = input();
    input.denom_reserve = Some(0.49);
    input.max_denom_reserve = Some(0.51);

    let decision = classify_pool(&input);

    assert!(decision.eligible);
    assert_eq!(decision.reason, None);
    assert_eq!(
        decision.eligible_outcome,
        Some(EligiblePoolOutcome::CurrentLowLiquidity)
    );
    assert!(!decision.tradable_now);
}

#[test]
fn cohort_uses_ever_tradable_flags_but_current_sell_failure_is_outcome() {
    let mut input = input();
    input.can_buy = true;
    input.can_sell = false;
    input.cohort_can_buy = Some(true);
    input.cohort_can_sell = Some(true);

    let decision = classify_pool(&input);

    assert!(decision.eligible);
    assert_eq!(decision.reason, None);
    assert_eq!(
        decision.eligible_outcome,
        Some(EligiblePoolOutcome::Honeypot)
    );
    assert!(!decision.tradable_now);
}

#[test]
fn first_eligible_observation_is_not_pool_creation() {
    let observations = vec![
        PoolEligibilityObservation::new(
            100,
            PoolClassificationInput::new(Some("WETH"), Some(0.1), false, false, false),
        ),
        PoolEligibilityObservation::new(
            101,
            PoolClassificationInput::new(Some("WETH"), Some(0.8), true, false, false),
        ),
        PoolEligibilityObservation::new(
            102,
            PoolClassificationInput::new(Some("WETH"), Some(0.8), true, true, false),
        ),
    ];

    let entry = first_eligible_observation(observations).unwrap();

    assert_eq!(entry.block_number, 102);
    assert!(entry.classification.eligible);
    assert_eq!(
        entry.classification.eligible_outcome,
        Some(EligiblePoolOutcome::Active)
    );
}

#[test]
fn first_eligible_observation_uses_current_entry_state_not_lifetime_state() {
    let mut low_current_after_prior_liquidity =
        PoolClassificationInput::new(Some("WETH"), Some(0.03), true, true, false);
    low_current_after_prior_liquidity.max_denom_reserve = Some(1.2);
    low_current_after_prior_liquidity.cohort_can_buy = Some(true);
    low_current_after_prior_liquidity.cohort_can_sell = Some(true);

    let observations = vec![
        PoolEligibilityObservation::new(100, low_current_after_prior_liquidity),
        PoolEligibilityObservation::new(
            101,
            PoolClassificationInput::new(Some("WETH"), Some(0.6), true, true, false),
        ),
    ];

    let entry = first_eligible_observation(observations).unwrap();

    assert_eq!(entry.block_number, 101);
    assert_eq!(entry.input.max_denom_reserve, Some(0.6));
    assert_eq!(entry.input.cohort_can_buy, None);
    assert_eq!(entry.input.cohort_can_sell, None);
}
