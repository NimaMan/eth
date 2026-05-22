use std::collections::HashMap;

use eth_live_trading::{GasRankProfile, StrategyGasRankPolicy};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;

const LIVE_GAS_REQUIRED_SOURCE_CONFIG: &str = "ALPHA_LIVE_GAS_RANK_REQUIRED_SOURCE";
const LIVE_GAS_LOOKBACK_BLOCKS_CONFIG: &str = "ALPHA_LIVE_GAS_RANK_LOOKBACK_BLOCKS";
const LIVE_GAS_SIMULATED_BUFFER_BPS_CONFIG: &str = "ALPHA_LIVE_GAS_SIMULATED_GAS_BUFFER_BPS";
const LIVE_GAS_MAX_PRIORITY_FEE_GWEI_CONFIG: &str = "ALPHA_LIVE_GAS_MAX_PRIORITY_FEE_GWEI";
const LIVE_ENTRY_MAX_GAS_FEE_ETH_CONFIG: &str = "ALPHA_LIVE_ENTRY_MAX_ESTIMATED_GAS_FEE_ETH";
const LIVE_EXIT_MAX_GAS_FEE_ETH_CONFIG: &str = "ALPHA_LIVE_EXIT_MAX_ESTIMATED_GAS_FEE_ETH";
const LIVE_GAS_SAFETY_BUFFER_ETH_CONFIG: &str = "ALPHA_LIVE_GAS_SAFETY_BUFFER_ETH";
const LIVE_V2_VAULT_BUY_GAS_LIMIT_CONFIG: &str = "ALPHA_LIVE_UNISWAP_V2_VAULT_BUY_GAS_LIMIT";
const LIVE_V2_VAULT_SELL_GAS_LIMIT_CONFIG: &str = "ALPHA_LIVE_UNISWAP_V2_VAULT_SELL_GAS_LIMIT";
const LIVE_ENTRY_BUY_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_ENTRY_BUY_GAS_PROFILES";
const LIVE_NORMAL_EXIT_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_NORMAL_EXIT_GAS_PROFILES";
const LIVE_MEMPOOL_RACE_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_MEMPOOL_RACE_EXIT_GAS_PROFILES";
const LIVE_MINED_APPROVAL_RACE_GAS_PROFILES_CONFIG: &str =
    "ALPHA_LIVE_MINED_APPROVAL_RACE_GAS_PROFILES";
const LIVE_BUY_CONFIRM_APPROVAL_GAS_PROFILES_CONFIG: &str =
    "ALPHA_LIVE_BUY_CONFIRM_APPROVAL_GAS_PROFILES";

#[derive(Clone, Debug)]
pub(super) struct LiveRealGasPolicy {
    pub(super) required_gas_rank_source: String,
    pub(super) gas_rank_lookback_blocks: u64,
    pub(super) simulated_gas_buffer_bps: u64,
    pub(super) max_priority_fee_gwei: Decimal,
    pub(super) entry_max_estimated_gas_fee_eth: Decimal,
    pub(super) exit_max_estimated_gas_fee_eth: Decimal,
    pub(super) safety_buffer_eth: Decimal,
    pub(super) v2_vault_buy_gas_limit: u64,
    pub(super) v2_vault_sell_gas_limit: u64,
    pub(super) entry_buy_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) normal_exit_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) mined_approval_race_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) buy_confirm_block_approval_gas_rank_policy: StrategyGasRankPolicy,
}

pub(super) fn load_live_real_gas_policy(
    config: &HashMap<String, String>,
) -> Result<LiveRealGasPolicy> {
    let gas_rank_lookback_blocks = required_config_u64(config, LIVE_GAS_LOOKBACK_BLOCKS_CONFIG)?;
    if !(1..=100).contains(&gas_rank_lookback_blocks) {
        return Err(eyre!(
            "{LIVE_GAS_LOOKBACK_BLOCKS_CONFIG} must be between 1 and 100; got {gas_rank_lookback_blocks}"
        ));
    }
    Ok(LiveRealGasPolicy {
        required_gas_rank_source: required_config_string(config, LIVE_GAS_REQUIRED_SOURCE_CONFIG)?,
        gas_rank_lookback_blocks,
        simulated_gas_buffer_bps: required_config_u64(
            config,
            LIVE_GAS_SIMULATED_BUFFER_BPS_CONFIG,
        )?,
        max_priority_fee_gwei: required_config_decimal(
            config,
            LIVE_GAS_MAX_PRIORITY_FEE_GWEI_CONFIG,
        )?,
        entry_max_estimated_gas_fee_eth: required_config_decimal(
            config,
            LIVE_ENTRY_MAX_GAS_FEE_ETH_CONFIG,
        )?,
        exit_max_estimated_gas_fee_eth: required_config_decimal(
            config,
            LIVE_EXIT_MAX_GAS_FEE_ETH_CONFIG,
        )?,
        safety_buffer_eth: required_config_decimal(config, LIVE_GAS_SAFETY_BUFFER_ETH_CONFIG)?,
        v2_vault_buy_gas_limit: required_config_positive_u64(
            config,
            LIVE_V2_VAULT_BUY_GAS_LIMIT_CONFIG,
        )?,
        v2_vault_sell_gas_limit: required_config_positive_u64(
            config,
            LIVE_V2_VAULT_SELL_GAS_LIMIT_CONFIG,
        )?,
        entry_buy_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_ENTRY_BUY_GAS_PROFILES_CONFIG,
        )?,
        normal_exit_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_NORMAL_EXIT_GAS_PROFILES_CONFIG,
        )?,
        mempool_pre_mine_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_MEMPOOL_RACE_GAS_PROFILES_CONFIG,
        )?,
        mined_approval_race_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_MINED_APPROVAL_RACE_GAS_PROFILES_CONFIG,
        )?,
        buy_confirm_block_approval_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_BUY_CONFIRM_APPROVAL_GAS_PROFILES_CONFIG,
        )?,
    })
}

fn required_config_string(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| eyre!("{key} must be set in config.env"))
}

fn required_config_u64(config: &HashMap<String, String>, key: &str) -> Result<u64> {
    let value = required_config_string(config, key)?;
    value
        .parse::<u64>()
        .wrap_err_with(|| format!("invalid {key} u64 value {value:?}"))
}

fn required_config_positive_u64(config: &HashMap<String, String>, key: &str) -> Result<u64> {
    let value = required_config_u64(config, key)?;
    if value == 0 {
        return Err(eyre!("{key} must be greater than zero"));
    }
    Ok(value)
}

fn required_config_decimal(config: &HashMap<String, String>, key: &str) -> Result<Decimal> {
    let value = required_config_string(config, key)?;
    value
        .parse::<Decimal>()
        .wrap_err_with(|| format!("invalid {key} decimal value {value:?}"))
}

fn required_config_gas_policy(
    config: &HashMap<String, String>,
    key: &str,
) -> Result<StrategyGasRankPolicy> {
    let value = required_config_string(config, key)?;
    let profiles = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_gas_rank_profile)
        .collect::<Result<Vec<_>>>()?;
    if profiles.is_empty() {
        return Err(eyre!("{key} must contain at least one gas-rank profile"));
    }
    Ok(StrategyGasRankPolicy::with_preference_order(profiles))
}

fn parse_gas_rank_profile(value: &str) -> Result<GasRankProfile> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "_")
        .as_str()
    {
        "minimum" => Ok(GasRankProfile::Minimum),
        "balanced" => Ok(GasRankProfile::Balanced),
        "aggressive" => Ok(GasRankProfile::Aggressive),
        "urgent" => Ok(GasRankProfile::Urgent),
        other => Err(eyre!("unknown gas-rank profile {other:?}")),
    }
}
