use std::collections::HashMap;

use eth_live_trading::{GasRankProfile, StrategyGasRankPolicy, TxSubmissionRoute};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;

const LIVE_GAS_REQUIRED_SOURCE_CONFIG: &str = "ALPHA_LIVE_GAS_RANK_REQUIRED_SOURCE";
const LIVE_GAS_LOOKBACK_BLOCKS_CONFIG: &str = "ALPHA_LIVE_GAS_RANK_LOOKBACK_BLOCKS";
const LIVE_GAS_PRIORITY_TIE_BREAKER_GWEI_CONFIG: &str = "ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI";
const LIVE_GAS_SIMULATED_BUFFER_BPS_CONFIG: &str = "ALPHA_LIVE_GAS_SIMULATED_GAS_BUFFER_BPS";
const LIVE_GAS_MIN_PRIORITY_FEE_GWEI_CONFIG: &str = "ALPHA_LIVE_GAS_MIN_PRIORITY_FEE_GWEI";
const LIVE_GAS_MAX_PRIORITY_FEE_GWEI_CONFIG: &str = "ALPHA_LIVE_GAS_MAX_PRIORITY_FEE_GWEI";
const LIVE_ENTRY_MAX_GAS_FEE_ETH_CONFIG: &str = "ALPHA_LIVE_ENTRY_MAX_ESTIMATED_GAS_FEE_ETH";
const LIVE_EXIT_MAX_GAS_FEE_ETH_CONFIG: &str = "ALPHA_LIVE_EXIT_MAX_ESTIMATED_GAS_FEE_ETH";
const LIVE_GAS_SAFETY_BUFFER_ETH_CONFIG: &str = "ALPHA_LIVE_GAS_SAFETY_BUFFER_ETH";
const LIVE_V2_VAULT_BUY_GAS_LIMIT_CONFIG: &str = "ALPHA_LIVE_UNISWAP_V2_VAULT_BUY_GAS_LIMIT";
const LIVE_V2_VAULT_SELL_GAS_LIMIT_CONFIG: &str = "ALPHA_LIVE_UNISWAP_V2_VAULT_SELL_GAS_LIMIT";
const LIVE_ENTRY_BUY_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_ENTRY_BUY_GAS_PROFILES";
const LIVE_TAIL_ENTRY_BUY_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_TAIL_ENTRY_BUY_GAS_PROFILES";
const LIVE_NORMAL_EXIT_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_NORMAL_EXIT_GAS_PROFILES";
const LIVE_MEMPOOL_RACE_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_MEMPOOL_RACE_EXIT_GAS_PROFILES";
const LIVE_LP_APPROVAL_EXIT_GAS_PROFILES_CONFIG: &str = "ALPHA_LIVE_LP_APPROVAL_EXIT_GAS_PROFILES";
const LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI_CONFIG: &str =
    "ALPHA_LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI";
const LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI_CONFIG: &str =
    "ALPHA_LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI";

#[derive(Clone, Debug)]
pub(super) struct LiveRealGasPolicy {
    pub(super) required_gas_rank_source: String,
    pub(super) gas_rank_lookback_blocks: u64,
    pub(super) gas_rank_priority_tie_breaker_gwei: Decimal,
    pub(super) simulated_gas_buffer_bps: u64,
    pub(super) min_priority_fee_gwei: Decimal,
    pub(super) max_priority_fee_gwei: Decimal,
    pub(super) entry_max_estimated_gas_fee_eth: Decimal,
    pub(super) exit_max_estimated_gas_fee_eth: Decimal,
    pub(super) safety_buffer_eth: Decimal,
    pub(super) v2_vault_buy_gas_limit: u64,
    pub(super) v2_vault_sell_gas_limit: u64,
    pub(super) mempool_race_priority_buffer_min_gwei: Decimal,
    pub(super) mempool_race_priority_buffer_max_gwei: Decimal,
    pub(super) entry_buy_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) tail_entry_buy_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) normal_exit_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy,
    pub(super) lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy,
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
    LiveRealGasPolicy {
        required_gas_rank_source: required_config_string(config, LIVE_GAS_REQUIRED_SOURCE_CONFIG)?,
        gas_rank_lookback_blocks,
        gas_rank_priority_tie_breaker_gwei: required_config_decimal(
            config,
            LIVE_GAS_PRIORITY_TIE_BREAKER_GWEI_CONFIG,
        )?,
        simulated_gas_buffer_bps: required_config_u64(
            config,
            LIVE_GAS_SIMULATED_BUFFER_BPS_CONFIG,
        )?,
        min_priority_fee_gwei: required_config_decimal(
            config,
            LIVE_GAS_MIN_PRIORITY_FEE_GWEI_CONFIG,
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
        mempool_race_priority_buffer_min_gwei: required_config_decimal(
            config,
            LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI_CONFIG,
        )?,
        mempool_race_priority_buffer_max_gwei: required_config_decimal(
            config,
            LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI_CONFIG,
        )?,
        entry_buy_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_ENTRY_BUY_GAS_PROFILES_CONFIG,
        )?
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast),
        tail_entry_buy_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_TAIL_ENTRY_BUY_GAS_PROFILES_CONFIG,
        )?
        .with_submission_route(TxSubmissionRoute::FlashbotsMevShareTail),
        normal_exit_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_NORMAL_EXIT_GAS_PROFILES_CONFIG,
        )?
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast),
        mempool_pre_mine_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_MEMPOOL_RACE_GAS_PROFILES_CONFIG,
        )?
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast),
        lp_approval_exit_gas_rank_policy: required_config_gas_policy(
            config,
            LIVE_LP_APPROVAL_EXIT_GAS_PROFILES_CONFIG,
        )?
        .with_submission_route(TxSubmissionRoute::PublicRpcBroadcast),
    }
    .validated()
}

pub(in crate::live_trader) struct BuyGasPolicyContext<'a> {
    pub(in crate::live_trader) policy: &'a StrategyGasRankPolicy,
    pub(in crate::live_trader) action: &'static str,
    pub(in crate::live_trader) signal: String,
    pub(in crate::live_trader) guard: &'static str,
}

impl LiveRealGasPolicy {
    fn validated(self) -> Result<Self> {
        if self.min_priority_fee_gwei < Decimal::ZERO {
            return Err(eyre!(
                "{LIVE_GAS_MIN_PRIORITY_FEE_GWEI_CONFIG} must be non-negative"
            ));
        }
        if self.min_priority_fee_gwei > self.max_priority_fee_gwei {
            return Err(eyre!(
                "{LIVE_GAS_MIN_PRIORITY_FEE_GWEI_CONFIG} ({}) must be <= {LIVE_GAS_MAX_PRIORITY_FEE_GWEI_CONFIG} ({})",
                self.min_priority_fee_gwei,
                self.max_priority_fee_gwei
            ));
        }
        if self.mempool_race_priority_buffer_min_gwei < Decimal::ZERO {
            return Err(eyre!(
                "{LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI_CONFIG} must be non-negative"
            ));
        }
        if self.gas_rank_priority_tie_breaker_gwei < Decimal::ZERO {
            return Err(eyre!(
                "{LIVE_GAS_PRIORITY_TIE_BREAKER_GWEI_CONFIG} must be non-negative"
            ));
        }
        if self.mempool_race_priority_buffer_max_gwei < self.mempool_race_priority_buffer_min_gwei {
            return Err(eyre!(
                "{LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI_CONFIG} must be >= {LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI_CONFIG}"
            ));
        }
        Ok(self)
    }

    pub(in crate::live_trader) fn buy_policy_context(
        &self,
        reason_code: Option<&str>,
    ) -> BuyGasPolicyContext<'_> {
        let signal = reason_code.unwrap_or("entry.buy_eligible_pool_once");
        if signal.starts_with("entry.tail_after_enabling_tx") {
            return BuyGasPolicyContext {
                policy: &self.tail_entry_buy_gas_rank_policy,
                action: "tail_entry_buy",
                signal: signal.to_string(),
                guard: "tail_entry_estimated_gas_fee_cap",
            };
        }
        BuyGasPolicyContext {
            policy: &self.entry_buy_gas_rank_policy,
            action: "entry_buy",
            signal: signal.to_string(),
            guard: "entry_estimated_gas_fee_cap",
        }
    }

    pub(in crate::live_trader) fn requires_flashbots_auth(&self) -> bool {
        [
            &self.entry_buy_gas_rank_policy,
            &self.tail_entry_buy_gas_rank_policy,
            &self.normal_exit_gas_rank_policy,
            &self.mempool_pre_mine_gas_rank_policy,
            &self.lp_approval_exit_gas_rank_policy,
        ]
        .iter()
        .any(|policy| policy.submission_route.requires_flashbots_auth())
    }
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
        "mempool_race" => Ok(GasRankProfile::MempoolRace),
        "normal" => Ok(GasRankProfile::Normal),
        "p50" => Ok(GasRankProfile::P50),
        "p55" => Ok(GasRankProfile::P55),
        "p60" => Ok(GasRankProfile::P60),
        "p65" => Ok(GasRankProfile::P65),
        "p70" => Ok(GasRankProfile::P70),
        "p75" => Ok(GasRankProfile::P75),
        "p77" => Ok(GasRankProfile::P77),
        "p85" => Ok(GasRankProfile::P85),
        "p88" => Ok(GasRankProfile::P88),
        "p90" => Ok(GasRankProfile::P90),
        "p92" => Ok(GasRankProfile::P92),
        "p94" => Ok(GasRankProfile::P94),
        "p95" => Ok(GasRankProfile::P95),
        "p96" => Ok(GasRankProfile::P96),
        "p97" => Ok(GasRankProfile::P97),
        "p99" => Ok(GasRankProfile::P99),
        other => Err(eyre!("unknown gas-rank profile {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use eth_live_trading::{StrategyGasRankPolicy, TxSubmissionRoute};
    use rust_decimal::Decimal;

    use super::*;

    fn policy() -> LiveRealGasPolicy {
        LiveRealGasPolicy {
            required_gas_rank_source: "eth_chain_server_gas_rank".to_string(),
            gas_rank_lookback_blocks: 100,
            gas_rank_priority_tie_breaker_gwei: Decimal::new(1456, 4),
            simulated_gas_buffer_bps: 2500,
            min_priority_fee_gwei: Decimal::ONE,
            max_priority_fee_gwei: Decimal::new(35, 1),
            entry_max_estimated_gas_fee_eth: Decimal::new(12, 4),
            exit_max_estimated_gas_fee_eth: Decimal::new(2, 3),
            safety_buffer_eth: Decimal::new(1, 3),
            v2_vault_buy_gas_limit: 300_000,
            v2_vault_sell_gas_limit: 300_000,
            mempool_race_priority_buffer_min_gwei: Decimal::new(1, 1),
            mempool_race_priority_buffer_max_gwei: Decimal::new(2, 1),
            entry_buy_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
            tail_entry_buy_gas_rank_policy: StrategyGasRankPolicy::p85_first()
                .with_submission_route(TxSubmissionRoute::FlashbotsMevShareTail),
            normal_exit_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
            mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy::p95_first(),
            lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy::p90_first(),
        }
    }

    fn config() -> HashMap<String, String> {
        [
            (LIVE_GAS_REQUIRED_SOURCE_CONFIG, "eth_chain_server_gas_rank"),
            (LIVE_GAS_LOOKBACK_BLOCKS_CONFIG, "100"),
            (LIVE_GAS_PRIORITY_TIE_BREAKER_GWEI_CONFIG, "0.1456"),
            (LIVE_GAS_SIMULATED_BUFFER_BPS_CONFIG, "2500"),
            (LIVE_GAS_MIN_PRIORITY_FEE_GWEI_CONFIG, "1"),
            (LIVE_GAS_MAX_PRIORITY_FEE_GWEI_CONFIG, "3.5"),
            (LIVE_ENTRY_MAX_GAS_FEE_ETH_CONFIG, "0.0012"),
            (LIVE_EXIT_MAX_GAS_FEE_ETH_CONFIG, "0.002"),
            (LIVE_GAS_SAFETY_BUFFER_ETH_CONFIG, "0.001"),
            (LIVE_V2_VAULT_BUY_GAS_LIMIT_CONFIG, "300000"),
            (LIVE_V2_VAULT_SELL_GAS_LIMIT_CONFIG, "300000"),
            (LIVE_ENTRY_BUY_GAS_PROFILES_CONFIG, "p75,p50,normal"),
            (
                LIVE_TAIL_ENTRY_BUY_GAS_PROFILES_CONFIG,
                "p85,p75,p50,normal",
            ),
            (LIVE_NORMAL_EXIT_GAS_PROFILES_CONFIG, "p75,p50,normal"),
            (LIVE_MEMPOOL_RACE_GAS_PROFILES_CONFIG, "mempool_race"),
            (
                LIVE_LP_APPROVAL_EXIT_GAS_PROFILES_CONFIG,
                "p90,p75,p50,normal",
            ),
            (LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MIN_GWEI_CONFIG, "0.1"),
            (LIVE_MEMPOOL_RACE_PRIORITY_BUFFER_MAX_GWEI_CONFIG, "0.2"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
    }

    #[test]
    fn tail_after_enabling_tx_uses_tail_entry_buy_context() {
        let policy = policy();
        let context = policy.buy_policy_context(Some("entry.tail_after_enabling_tx"));

        assert_eq!(context.action, "tail_entry_buy");
        assert_eq!(context.signal, "entry.tail_after_enabling_tx");
        assert_eq!(context.guard, "tail_entry_estimated_gas_fee_cap");
        assert_eq!(
            context.policy.preference_order,
            StrategyGasRankPolicy::p85_first().preference_order
        );
        assert_eq!(
            context.policy.submission_route,
            TxSubmissionRoute::FlashbotsMevShareTail
        );
    }

    #[test]
    fn regular_entry_uses_entry_buy_context() {
        let policy = policy();
        let context = policy.buy_policy_context(Some("entry.buy_eligible_pool_once"));

        assert_eq!(context.action, "entry_buy");
        assert_eq!(context.signal, "entry.buy_eligible_pool_once");
        assert_eq!(context.guard, "entry_estimated_gas_fee_cap");
        assert_eq!(
            context.policy.preference_order,
            StrategyGasRankPolicy::p75_first().preference_order
        );
        assert_eq!(
            context.policy.submission_route,
            TxSubmissionRoute::PublicRpcBroadcast
        );
    }

    #[test]
    fn live_real_gas_policy_owns_submission_routes() {
        let policy = load_live_real_gas_policy(&config()).unwrap();

        assert_eq!(
            policy.entry_buy_gas_rank_policy.submission_route,
            TxSubmissionRoute::PublicRpcBroadcast
        );
        assert_eq!(
            policy.tail_entry_buy_gas_rank_policy.submission_route,
            TxSubmissionRoute::FlashbotsMevShareTail
        );
        assert_eq!(
            policy.normal_exit_gas_rank_policy.submission_route,
            TxSubmissionRoute::PublicRpcBroadcast
        );
        assert_eq!(
            policy.mempool_pre_mine_gas_rank_policy.submission_route,
            TxSubmissionRoute::PublicRpcBroadcast
        );
        assert_eq!(
            policy.lp_approval_exit_gas_rank_policy.submission_route,
            TxSubmissionRoute::PublicRpcBroadcast
        );
        assert!(policy.requires_flashbots_auth());
    }

    #[test]
    fn accepts_strategy_min_priority_fee_floor() {
        let policy = policy();

        assert_eq!(policy.min_priority_fee_gwei, Decimal::ONE);
    }

    #[test]
    fn rejects_strategy_min_priority_fee_above_strategy_cap() {
        let mut policy = policy();
        policy.min_priority_fee_gwei = Decimal::from(4);

        let error = policy.validated().unwrap_err();

        assert!(error
            .to_string()
            .contains("ALPHA_LIVE_GAS_MIN_PRIORITY_FEE_GWEI"));
    }
}
