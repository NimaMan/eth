use alloy_primitives::{Address, U256};
use eth_live_trading::{
    EthTxExecutorBroadcastMode, EthTxExecutorClient, EthTxExecutorClientConfig, EthTxExecutorStatus,
};
use eth_strategies::{
    alpha11::{HOLD16_STRATEGY_NAME, INITIAL_ENTRY_BANKROLL_ETH},
    shared_rules::live::LiveStrategySpec,
};
use eyre::{eyre, Result, WrapErr};

use super::super::cli::{Args, RealExecutionArgs};
pub(super) const HOLD16_DEPLOY_BUY_WEI: &str = "5000000000000000";

pub(in crate::live_trader) struct EthTxExecutorRealPreflight {
    pub(in crate::live_trader) token: String,
    pub(in crate::live_trader) status: EthTxExecutorStatus,
}

pub(in crate::live_trader) async fn preflight_eth_tx_executor_real(
    args: &RealExecutionArgs,
    live_args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<EthTxExecutorRealPreflight> {
    let token = load_eth_tx_executor_bearer_token(&args.eth_tx_executor_token_env)?;
    let status = EthTxExecutorClient::new(EthTxExecutorClientConfig::new(
        &args.eth_tx_executor_url,
        token.clone(),
    ))
    .eth_tx_status()
    .await
    .wrap_err("failed to read ETH tx executor status")?;
    validate_eth_tx_executor_real_status(&status, args, live_args, strategy_specs)?;
    Ok(EthTxExecutorRealPreflight { token, status })
}

pub(super) fn validate_eth_tx_executor_real_status(
    status: &EthTxExecutorStatus,
    args: &RealExecutionArgs,
    live_args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<()> {
    if status.execution_disabled {
        return Err(eyre!("ETH tx executor kill switch is active"));
    }
    if !status.enabled {
        return Err(eyre!("ETH tx executor is disabled"));
    }
    if !status.signer_available {
        return Err(eyre!("ETH tx executor signer is not available"));
    }
    if status.policy.allowed_target_count == 0 || status.policy.allowed_selector_count == 0 {
        return Err(eyre!(
            "ETH tx executor policy must have non-empty target and selector allowlists"
        ));
    }
    if status.submit_endpoint.is_none() {
        return Err(eyre!(
            "eth-tx-real trader requires ETH tx executor /eth/tx/submit support"
        ));
    }
    match status.broadcast_mode {
        EthTxExecutorBroadcastMode::DryRun => Ok(()),
        EthTxExecutorBroadcastMode::Broadcast if args.allow_broadcast_live_validation => {
            validate_broadcast_hold16_deploy(status, live_args, strategy_specs)
        }
        EthTxExecutorBroadcastMode::Broadcast => Err(eyre!(
            "eth-tx-real trader requires broadcast_mode=dry_run unless --allow-broadcast-live-validation is set for the hold16 deploy strategy"
        )),
        EthTxExecutorBroadcastMode::Unknown => Err(eyre!(
            "eth-tx-real trader cannot run with unknown ETH tx executor broadcast_mode"
        )),
    }
}

fn validate_broadcast_hold16_deploy(
    status: &EthTxExecutorStatus,
    args: &Args,
    strategy_specs: &[LiveStrategySpec],
) -> Result<()> {
    if args.strategy_set.as_deref() != Some(HOLD16_STRATEGY_NAME) {
        return Err(eyre!(
            "broadcast hold16 deploy requires --strategy-set {HOLD16_STRATEGY_NAME}"
        ));
    }
    if strategy_specs.len() != 1 || strategy_specs[0].strategy_name != HOLD16_STRATEGY_NAME {
        return Err(eyre!(
            "broadcast hold16 deploy requires exactly one resolved strategy spec named {HOLD16_STRATEGY_NAME}"
        ));
    }
    let spec = &strategy_specs[0];
    if spec.max_entry_pools.is_some() {
        return Err(eyre!(
            "broadcast hold16 deploy requires strategy spec max_entry_pools unset; bankroll governs entry capacity"
        ));
    }
    if args.disable_entry {
        return Err(eyre!("broadcast hold16 deploy requires entries enabled"));
    }
    if args.once {
        return Err(eyre!(
            "broadcast hold16 deploy must keep running so receipt reconciliation and hold16 exits can complete"
        ));
    }
    if args.replay_current {
        return Err(eyre!(
            "broadcast hold16 deploy must not use --replay-current; start from fresh live observations only"
        ));
    }

    let buy_wei = parse_policy_wei(&spec.buy_wei, "strategy buy_wei")?;
    let max_buy_wei = parse_policy_wei(HOLD16_DEPLOY_BUY_WEI, "hold16 deploy buy cap")?;
    if buy_wei.is_zero() || buy_wei > max_buy_wei {
        return Err(eyre!(
            "broadcast hold16 deploy requires 0 < strategy buy_wei <= {HOLD16_DEPLOY_BUY_WEI}; got {}",
            spec.buy_wei
        ));
    }

    let entry_bankroll_eth = spec
        .entry_bankroll_eth
        .as_deref()
        .ok_or_else(|| eyre!("broadcast hold16 deploy requires strategy entry_bankroll_eth"))?;
    let entry_bankroll_wei = super::super::support::parse_eth_decimal_to_wei(
        entry_bankroll_eth,
        "strategy entry_bankroll_eth",
    )?;
    let max_entry_bankroll_wei = super::super::support::parse_eth_decimal_to_wei(
        INITIAL_ENTRY_BANKROLL_ETH,
        "hold16 deploy entry bankroll cap",
    )?;
    if entry_bankroll_wei.is_zero() || entry_bankroll_wei > max_entry_bankroll_wei {
        return Err(eyre!(
            "broadcast hold16 deploy requires entry bankroll in (0, {INITIAL_ENTRY_BANKROLL_ETH}] ETH; got {entry_bankroll_eth}"
        ));
    }

    let max_value_wei = parse_policy_wei(&status.policy.max_value_wei, "policy max_value_wei")?;
    if max_value_wei < buy_wei {
        return Err(eyre!(
            "ETH tx executor max_value_wei {} is below hold16 deploy buy value {buy_wei}",
            status.policy.max_value_wei
        ));
    }
    let max_transaction_cost_wei = parse_policy_wei(
        &status.policy.max_transaction_cost_wei,
        "policy max_transaction_cost_wei",
    )?;
    if max_transaction_cost_wei.is_zero() {
        return Err(eyre!(
            "ETH tx executor max_transaction_cost_wei must be nonzero for broadcast hold16 deploy"
        ));
    }
    let max_daily_cost_wei = parse_policy_wei(
        &status.policy.max_daily_cost_wei,
        "policy max_daily_cost_wei",
    )?;
    let daily_spend_cap_enabled = status
        .policy
        .daily_spend_cap_enabled
        .unwrap_or_else(|| !max_daily_cost_wei.is_zero());
    if daily_spend_cap_enabled && max_daily_cost_wei < max_transaction_cost_wei {
        return Err(eyre!(
            "ETH tx executor max_daily_cost_wei {} is below max_transaction_cost_wei {}",
            status.policy.max_daily_cost_wei,
            status.policy.max_transaction_cost_wei
        ));
    }
    if !status.policy.require_simulation || status.policy.max_simulation_age_blocks > 2 {
        return Err(eyre!(
            "broadcast hold16 deploy requires fresh simulation policy: require_simulation=true and max_simulation_age_blocks <= 2"
        ));
    }
    Ok(())
}

fn parse_policy_wei(value: &str, label: &str) -> Result<U256> {
    U256::from_str_radix(value.trim(), 10)
        .wrap_err_with(|| format!("invalid {label} decimal wei value {value:?}"))
}

fn load_eth_tx_executor_bearer_token(token_env: &str) -> Result<String> {
    let token = std::env::var(token_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| eyre!("missing ETH tx executor bearer token in {token_env}"))?;
    Ok(token)
}

pub(super) fn parse_live_real_address(value: &str, label: &str) -> Result<Address> {
    value
        .parse::<Address>()
        .wrap_err_with(|| format!("invalid {label} address {value:?}"))
}
