use super::pool_metadata_lookup::is_optional_uniswap_v2_pool_metadata_miss;
use super::trading_status_update::{
    current_block_simulation_pool_addresses, is_optional_direct_live_pool_simulation_error,
    should_simulate_at_current_block,
};
use super::{pool_metadata_timeout_for_trading_simulation, PoolTradingSimulationMode};

#[test]
fn discovered_current_block_pool_uses_current_block_state() {
    let pool = "0xb80b6c2453b82996950a81c42db446e95a3fa2d5";

    assert!(should_simulate_at_current_block(
        pool,
        &[pool.to_string()],
        PoolTradingSimulationMode::Noop
    ));
}

#[test]
fn existing_pool_without_current_discovery_uses_parent_replay() {
    assert!(!should_simulate_at_current_block(
        "0xb80b6c2453b82996950a81c42db446e95a3fa2d5",
        &[],
        PoolTradingSimulationMode::Noop
    ));
}

#[test]
fn non_direct_live_modes_do_not_timeout_v2_pool_metadata() {
    assert!(
        pool_metadata_timeout_for_trading_simulation(PoolTradingSimulationMode::Noop).is_none()
    );
}

#[test]
fn updated_pool_uses_current_block_state_without_setup_replay() {
    let updated = vec!["0x0000000000000000000000000000000000000001".to_string()];
    let current = current_block_simulation_pool_addresses(&[], &updated, &[], false);

    assert_eq!(current, updated);
}

#[test]
fn forced_token_control_simulates_all_pools_at_current_block() {
    let pools = vec![
        "0x0000000000000000000000000000000000000002".to_string(),
        "0x0000000000000000000000000000000000000001".to_string(),
    ];
    let current = current_block_simulation_pool_addresses(&pools, &[], &[], true);

    assert_eq!(
        current,
        vec![
            "0x0000000000000000000000000000000000000001".to_string(),
            "0x0000000000000000000000000000000000000002".to_string()
        ]
    );
}

#[test]
fn live_header_gap_is_optional_pool_metadata_miss() {
    assert!(is_optional_uniswap_v2_pool_metadata_miss(
        "missing live block header for 25050934"
    ));
    assert!(is_optional_uniswap_v2_pool_metadata_miss(
        "live chain cache not configured"
    ));
}

#[test]
fn direct_live_context_misses_are_optional_pool_simulation_errors() {
    assert!(is_optional_direct_live_pool_simulation_error(
        true,
        "while executing Uniswap V4 simulation step weth_deposit: live chain cache not configured"
    ));
    assert!(is_optional_direct_live_pool_simulation_error(
        true,
        "missing live block header for 25092345"
    ));
    assert!(!is_optional_direct_live_pool_simulation_error(
        false,
        "live chain cache not configured"
    ));
    assert!(!is_optional_direct_live_pool_simulation_error(
        true,
        "transaction validation error: lack of funds"
    ));
}

#[test]
fn token_decimals_failures_are_optional_pool_metadata_misses() {
    assert!(is_optional_uniswap_v2_pool_metadata_miss(
        "Failed to get token decimals for 0x38c6a68304cdefb9bec48bbfaaba5c5b47818bb2"
    ));
    assert!(is_optional_uniswap_v2_pool_metadata_miss(
            "Token decimals call for 0xe0b7927c4af23765cb51314a0e0521a9645f0e2a returned 0 bytes (expected >= 32)"
        ));
    assert!(is_optional_uniswap_v2_pool_metadata_miss(
        "factory() view call failed or empty output"
    ));
}
