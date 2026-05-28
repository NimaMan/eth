use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::Amount,
    decision_rationale::source,
    execution::{ExecutionReport, ExecutionStatus},
    order::{OrderIntent, OrderSide},
    position::Position,
    store::TradingStore,
    strategy::StrategyDecision,
};
use eth_alpha_store::{ManualCloseRequest, PostgresTradingStore};
use eth_strategies::SnipeAllConfig;
use eyre::{eyre, Result, WrapErr};
use serde_json::json;

use crate::{
    decision::strategy_decision_record, AlphaEngine, BlockCriticalRiskPolicy,
    EngineExecutionAdapter,
};

const DEFAULT_MANUAL_CLOSE_LIMIT: usize = 5;
const MANUAL_CLOSE_REASON: &str = "manual.close_position";

#[derive(Clone, Debug, Default)]
pub(super) struct ManualCloseProcessSummary {
    pub claimed: usize,
    pub processed: usize,
    pub failed: usize,
    pub reports: usize,
}

pub(super) const fn default_manual_close_limit() -> usize {
    DEFAULT_MANUAL_CLOSE_LIMIT
}

pub(super) async fn process_manual_close_requests<E>(
    store: &PostgresTradingStore,
    engine: &mut AlphaEngine<E, BlockCriticalRiskPolicy, PostgresTradingStore>,
    vault_address: Option<Address>,
    current_block: Option<u64>,
    limit: usize,
) -> Result<ManualCloseProcessSummary>
where
    E: EngineExecutionAdapter,
{
    let requests = store
        .claim_pending_manual_close_requests(limit)
        .await
        .wrap_err("failed to claim pending manual close requests")?;
    let mut summary = ManualCloseProcessSummary {
        claimed: requests.len(),
        ..ManualCloseProcessSummary::default()
    };

    for request in requests {
        match process_one_manual_close(store, engine, vault_address, current_block, &request).await
        {
            Ok(reports) => {
                summary.reports += reports.len();
                if let Some(error) = report_failure(&reports) {
                    store
                        .mark_manual_close_request_failed(&request.request_id, &error)
                        .await?;
                    summary.failed += 1;
                } else if reports.is_empty() {
                    store
                        .mark_manual_close_request_failed(
                            &request.request_id,
                            "manual close decision produced no execution report",
                        )
                        .await?;
                    summary.failed += 1;
                } else {
                    store
                        .mark_manual_close_request_processed(&request.request_id)
                        .await?;
                    summary.processed += 1;
                }
            }
            Err(error) => {
                store
                    .mark_manual_close_request_failed(&request.request_id, &error.to_string())
                    .await?;
                summary.failed += 1;
            }
        }
    }

    Ok(summary)
}

async fn process_one_manual_close<E>(
    store: &PostgresTradingStore,
    engine: &mut AlphaEngine<E, BlockCriticalRiskPolicy, PostgresTradingStore>,
    vault_address: Option<Address>,
    current_block: Option<u64>,
    request: &ManualCloseRequest,
) -> Result<Vec<ExecutionReport>>
where
    E: EngineExecutionAdapter,
{
    let position = engine
        .portfolio()
        .positions
        .values()
        .find(|position| {
            position.key.strategy_name.0 == request.strategy_name
                && position.trade_id.0 == request.trade_id
                && request
                    .position_id
                    .as_ref()
                    .map(|position_id| &position.id.0 == position_id)
                    .unwrap_or(true)
                && position
                    .key
                    .token_address
                    .to_string()
                    .eq_ignore_ascii_case(&request.token_address)
                && position.key.pool_address.0 == request.pool_address
                && position.can_submit_exit()
        })
        .cloned()
        .ok_or_else(|| {
            eyre!(
                "no active sellable position found for manual close request {}",
                request.request_id
            )
        })?;

    let (available_tokens, decimals, balance_block, balance_source) = match vault_address {
        Some(_) => {
            return Err(eyre!(
                "manual close vault balance lookup is not supported in chain-sim execution mode"
            ));
        }
        None => chain_sim_position_token_balance(&position, request)?,
    };
    let amount_raw = requested_close_amount(request, available_tokens)?;
    let defaults = SnipeAllConfig::default();
    let intent = OrderIntent {
        trade_id: Some(position.trade_id.clone()),
        portfolio_id: position.key.portfolio_id.clone(),
        wallet_id: position.key.wallet_id.clone(),
        strategy_name: position.key.strategy_name.clone(),
        side: OrderSide::Sell,
        token_address: position.key.token_address,
        pool_address: position.key.pool_address.clone(),
        protocol: position.key.protocol.clone(),
        amount: Amount {
            raw: amount_raw,
            decimals,
        },
        route: None,
        max_slippage_bps: defaults.max_slippage_bps,
        deadline_secs: defaults.deadline_secs,
        decision_reason: None,
    };
    let decision = StrategyDecision::submit_order(intent, MANUAL_CLOSE_REASON);
    let decision_block = current_block.or(Some(balance_block));
    let mut decision_record = strategy_decision_record(
        &request.strategy_name,
        source::EVENT_SOURCE_MANUAL_CLOSE,
        format!("manual_close:{}", request.request_id),
        decision_block,
        Some(request.token_address.clone()),
        Some(request.pool_address.clone()),
        &decision,
    );
    decision_record.reason_details = Some(json!({
        "manual_close_request_id": request.request_id,
        "requested_percent": request.requested_percent,
        "requested_raw_amount": request.requested_raw_amount,
        "available_token_balance_raw": available_tokens.to_string(),
        "resolved_amount_raw": amount_raw.to_string(),
        "balance_block": balance_block,
        "balance_source": balance_source,
    }));
    store.record_strategy_decision(&decision_record).await?;

    engine
        .apply_decisions(vec![decision], source::EVENT_SOURCE_MANUAL_CLOSE, None)
        .await
        .wrap_err_with(|| {
            format!(
                "manual close request {} failed during normal order execution",
                request.request_id
            )
        })
}

fn chain_sim_position_token_balance(
    position: &Position,
    request: &ManualCloseRequest,
) -> Result<(U256, u8, u64, &'static str)> {
    let amount = position.entry_token_raw_amount.as_ref().ok_or_else(|| {
        eyre!(
            "manual close request {} cannot resolve simulated position token amount",
            request.request_id
        )
    })?;
    if amount.raw.is_zero() {
        return Err(eyre!(
            "simulated position token balance is zero for manual close request {}",
            request.request_id
        ));
    }
    Ok((
        amount.raw,
        amount.decimals,
        position.entry_block.unwrap_or_default(),
        "chain_sim_position_entry_token_amount",
    ))
}

fn requested_close_amount(request: &ManualCloseRequest, vault_balance: U256) -> Result<U256> {
    if vault_balance.is_zero() {
        return Err(eyre!(
            "current vault token balance is zero for manual close request {}",
            request.request_id
        ));
    }
    if let Some(raw_amount) = request
        .requested_raw_amount
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let amount = parse_u256_quantity(raw_amount, "requested_raw_amount")?;
        if amount.is_zero() {
            return Err(eyre!("requested_raw_amount must be greater than zero"));
        }
        if amount > vault_balance {
            return Err(eyre!(
                "requested_raw_amount {} exceeds current vault token balance {}",
                amount,
                vault_balance
            ));
        }
        return Ok(amount);
    }

    let percent_bps = requested_percent_bps(request.requested_percent.as_deref())?;
    let amount = vault_balance.saturating_mul(U256::from(percent_bps)) / U256::from(10_000u64);
    if amount.is_zero() {
        return Err(eyre!(
            "manual close percent resolved to zero tokens from vault balance {}",
            vault_balance
        ));
    }
    Ok(amount)
}

fn requested_percent_bps(value: Option<&str>) -> Result<u64> {
    let text = value.unwrap_or("100").trim();
    if text.is_empty() {
        return Ok(10_000);
    }
    let mut parts = text.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() || whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(eyre!("requested_percent must be a decimal percentage"));
    }
    let whole_value = whole
        .parse::<u64>()
        .wrap_err("requested_percent whole part is invalid")?;
    let fraction_value = match fraction {
        None | Some("") => 0,
        Some(value) if value.len() <= 2 && value.chars().all(|ch| ch.is_ascii_digit()) => {
            let mut padded = value.to_string();
            while padded.len() < 2 {
                padded.push('0');
            }
            padded
                .parse::<u64>()
                .wrap_err("requested_percent fractional part is invalid")?
        }
        Some(_) => {
            return Err(eyre!(
                "requested_percent supports at most two decimal places"
            ))
        }
    };
    let bps = whole_value
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction_value))
        .ok_or_else(|| eyre!("requested_percent is too large"))?;
    if bps == 0 || bps > 10_000 {
        return Err(eyre!(
            "requested_percent must be greater than 0 and no more than 100"
        ));
    }
    Ok(bps)
}

fn parse_u256_quantity(value: &str, label: &str) -> Result<U256> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).wrap_err_with(|| format!("invalid {label} hex quantity"))
    } else {
        U256::from_str_radix(trimmed, 10)
            .wrap_err_with(|| format!("invalid {label} decimal quantity"))
    }
}

fn report_failure(reports: &[ExecutionReport]) -> Option<String> {
    reports
        .iter()
        .find(|report| report.status == ExecutionStatus::Failed)
        .map(|report| {
            report
                .error
                .clone()
                .unwrap_or_else(|| format!("manual close report {} failed", report.order_id.0))
        })
}

#[cfg(test)]
mod tests {
    use super::requested_percent_bps;

    #[test]
    fn manual_close_percent_defaults_to_full_balance() {
        assert_eq!(requested_percent_bps(None).unwrap(), 10_000);
        assert_eq!(requested_percent_bps(Some("")).unwrap(), 10_000);
        assert_eq!(requested_percent_bps(Some("100")).unwrap(), 10_000);
    }

    #[test]
    fn manual_close_percent_supports_basis_points() {
        assert_eq!(requested_percent_bps(Some("12.34")).unwrap(), 1_234);
        assert_eq!(requested_percent_bps(Some("0.01")).unwrap(), 1);
    }

    #[test]
    fn manual_close_percent_rejects_invalid_values() {
        assert!(requested_percent_bps(Some("0")).is_err());
        assert!(requested_percent_bps(Some("100.01")).is_err());
        assert!(requested_percent_bps(Some("1.234")).is_err());
    }
}
