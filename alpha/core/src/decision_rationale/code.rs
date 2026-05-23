use serde_json::{json, Map, Value};

use super::category::ReasonCategory;

pub const NO_REASON: &str = "decision.no_reason";

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedReason {
    pub code: String,
    pub category: ReasonCategory,
    pub label: String,
    pub details: Value,
}

pub fn normalize_reason(raw: &str, action: Option<&str>) -> NormalizedReason {
    let raw = raw.trim();
    if raw.is_empty() {
        return NormalizedReason {
            code: NO_REASON.to_string(),
            category: ReasonCategory::Unknown,
            label: "No reason recorded".to_string(),
            details: json!({}),
        };
    }

    let parts = raw.split(':').map(str::trim).collect::<Vec<_>>();
    let base = parts.first().copied().unwrap_or(raw);
    let detail = parts.get(1).copied().filter(|value| !value.is_empty());
    let code = canonical_code(base, detail, parts.get(2).copied());
    let category = category_for(&code, action);
    let label = label_for(&code, raw);
    let details = details_for(raw, base, detail, parts.get(2).copied());

    NormalizedReason {
        code,
        category,
        label,
        details,
    }
}

fn canonical_code(base: &str, detail: Option<&str>, value: Option<&str>) -> String {
    if detail == Some("pool_denom_reserve_below_min_sell_threshold") {
        return format!("{base}.pool_denom_reserve_below_min_sell_threshold");
    }

    match (base, detail) {
        ("entry.buy_eligible_pool_once", Some("pool already bought")) => {
            "entry.buy_eligible_pool_once.already_bought".to_string()
        }
        ("entry.eligibility", Some(detail)) => {
            format!("entry.eligibility.{}", sanitize_code_part(detail))
        }
        ("entry.lp_approval_gate", Some(detail)) => {
            format!("entry.lp_approval_gate.{}", sanitize_code_part(detail))
        }
        ("entry.init_policy", Some(detail)) => {
            format!("entry.init_policy.{}", sanitize_code_part(detail))
        }
        ("exit.lp_approval", Some(detail)) => {
            format!("exit.lp_approval.{}", sanitize_code_part(detail))
        }
        ("entry.protocol_not_allowed", Some(_)) => "entry.protocol_not_allowed".to_string(),
        (_, Some(detail)) if value.is_none() && base.starts_with("risk_policy.") => {
            format!("{base}.{}", sanitize_code_part(detail))
        }
        _ => base.to_string(),
    }
}

fn category_for(code: &str, action: Option<&str>) -> ReasonCategory {
    if code.starts_with("entry.") {
        ReasonCategory::Entry
    } else if code.starts_with("exit.") {
        ReasonCategory::Exit
    } else if code.starts_with("risk_policy.") {
        ReasonCategory::RiskPolicy
    } else if code.starts_with("execution.") {
        ReasonCategory::Execution
    } else if action == Some("hold") || code == "risk.no_exit_rule_matched" {
        ReasonCategory::Hold
    } else {
        ReasonCategory::Unknown
    }
}

fn label_for(code: &str, raw: &str) -> String {
    match code {
        "entry.buy_eligible_pool_once" => "Entry: eligible pool".to_string(),
        "entry.buy_eligible_pool_once.already_bought" => {
            "Entry hold: pool already bought".to_string()
        }
        "entry.blocked_by_active_risk" => "Entry hold: blocked by active risk".to_string(),
        "entry.protocol_not_allowed" => "Entry hold: protocol not allowed".to_string(),
        "entry.eligibility.low_liquidity" => "Entry hold: low liquidity".to_string(),
        "entry.eligibility.cannot_buy" => "Entry hold: cannot buy".to_string(),
        "entry.eligibility.cannot_sell" => "Entry hold: cannot sell".to_string(),
        "entry.eligibility.unsupported_execution_denom" => {
            "Entry hold: unsupported execution denom".to_string()
        }
        "entry.eligibility.unsupported_execution_missing_denom" => {
            "Entry hold: missing execution denom".to_string()
        }
        "entry.eligibility.unsupported_v4_hooks" => "Entry hold: unsupported V4 hooks".to_string(),
        "entry.eligibility.unsupported_v4_missing_pool_key" => {
            "Entry hold: missing V4 pool key".to_string()
        }
        "entry.lp_approval_gate.approved_pct_gt_min" => {
            "Entry hold: LP approval above threshold".to_string()
        }
        "entry.init_policy.missing_creation_block" => {
            "Entry hold: missing pool creation block".to_string()
        }
        "entry.init_policy.creation_block_after_entry" => {
            "Entry hold: pool creation block after entry".to_string()
        }
        "entry.init_policy.pool_age_gt_max" => {
            "Entry hold: pool age above init-policy threshold".to_string()
        }
        "entry.init_policy.price_to_initial_ratio_missing" => {
            "Entry hold: missing price/initial ratio".to_string()
        }
        "entry.init_policy.price_to_initial_ratio_gt_max" => {
            "Entry hold: price/initial above init-policy threshold".to_string()
        }
        "exit.lp_approval" => "Exit: LP approval".to_string(),
        "exit.lp_approval_buy_confirm_block" => "Exit: buy-confirm block LP approval".to_string(),
        "exit.lp_approval_mined_race" => "Exit: mined LP approval race".to_string(),
        "exit.liquidity_removal" => "Exit: liquidity removal".to_string(),
        "exit.mempool_liquidity_removal_signal" => {
            "Exit: mempool liquidity removal signal".to_string()
        }
        "exit.max_hold_active_blocks" => "Exit: max hold".to_string(),
        "exit.take_profit" => "Exit: take profit".to_string(),
        "exit.stop_loss" => "Exit: stop loss".to_string(),
        "exit.tax" => "Exit: tax risk".to_string(),
        "exit.scam" => "Exit: scam risk".to_string(),
        "exit.failed_retry" => "Exit: retry failed exit".to_string(),
        "exit.no_sellable_position" => "Exit hold: no sellable position".to_string(),
        "exit.no_token_amount" => "Exit hold: no token amount".to_string(),
        "exit.lp_approval_not_critical" => "Exit hold: LP approval not critical".to_string(),
        "exit.lp_approval_buy_confirm_block_deferred_to_max_hold" => {
            "Exit hold: buy-confirm block LP approval deferred".to_string()
        }
        "exit.lp_approval.early_approval_deferred_to_max_hold" => {
            "Exit hold: early LP approval deferred to max hold".to_string()
        }
        "exit.lp_approval.approved_pct_unknown_or_not_gt_min" => {
            "Exit hold: LP approval below threshold or unknown".to_string()
        }
        "exit.lp_approval.no_open_matching_position" => {
            "Exit hold: no open matching position".to_string()
        }
        "exit.liquidity_removal.pool_denom_reserve_below_min_sell_threshold"
        | "exit.max_hold_active_blocks.pool_denom_reserve_below_min_sell_threshold" => {
            "Exit hold: pool reserve below sell threshold".to_string()
        }
        "risk.no_exit_rule_matched" => "Risk hold: no exit rule matched".to_string(),
        "position_open_no_exit" => "Hold: position open, no exit".to_string(),
        "position_exit_failed_no_strategy_retry" => {
            "Hold: exit failed; strategy retry disabled".to_string()
        }
        "position_exit_pending" => "Hold: exit pending".to_string(),
        "market_event_not_pool_update" => "Hold: market event is not pool update".to_string(),
        _ => humanize_reason(raw),
    }
}

fn details_for(raw: &str, base: &str, detail: Option<&str>, value: Option<&str>) -> Value {
    let mut details = Map::new();
    details.insert("raw".to_string(), Value::String(raw.to_string()));
    details.insert("rule".to_string(), Value::String(base.to_string()));
    if let Some(detail) = detail {
        details.insert("detail".to_string(), Value::String(detail.to_string()));
    }
    if let Some(value) = value {
        details.insert("value".to_string(), Value::String(value.to_string()));
    }
    if base == "entry.protocol_not_allowed" {
        if let Some(protocol) = detail {
            details.insert("protocol".to_string(), Value::String(protocol.to_string()));
        }
    }
    if detail == Some("pool_denom_reserve_below_min_sell_threshold") {
        if let Some(value) = value {
            if let Some((reserve, threshold)) = value.split_once('<') {
                details.insert(
                    "pool_denom_reserve".to_string(),
                    Value::String(reserve.trim().to_string()),
                );
                details.insert(
                    "min_sell_pool_denom_reserve".to_string(),
                    Value::String(threshold.trim().to_string()),
                );
            }
        }
    }
    if base == "exit.lp_approval" && detail == Some("early_approval_deferred_to_max_hold") {
        if let Some(value) = value {
            for part in value.split_whitespace() {
                let Some((key, raw_value)) = part.split_once('=') else {
                    continue;
                };
                if key.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
                    details.insert(key.to_string(), Value::String(raw_value.to_string()));
                }
            }
        }
    }
    Value::Object(details)
}

fn sanitize_code_part(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    sanitized
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn humanize_reason(value: &str) -> String {
    let text = value.replace('.', ": ").replace('_', " ");
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dust_sell_guard_without_numeric_bucket_fragmentation() {
        let reason = normalize_reason(
            "exit.max_hold_active_blocks:pool_denom_reserve_below_min_sell_threshold:0.001<0.01",
            Some("hold"),
        );

        assert_eq!(
            reason.code,
            "exit.max_hold_active_blocks.pool_denom_reserve_below_min_sell_threshold"
        );
        assert_eq!(reason.category, ReasonCategory::Exit);
        assert_eq!(
            reason.details["pool_denom_reserve"],
            Value::String("0.001".to_string())
        );
        assert_eq!(
            reason.details["min_sell_pool_denom_reserve"],
            Value::String("0.01".to_string())
        );
    }

    #[test]
    fn normalizes_entry_eligibility_detail() {
        let reason = normalize_reason("entry.eligibility:low_liquidity", Some("hold"));

        assert_eq!(reason.code, "entry.eligibility.low_liquidity");
        assert_eq!(reason.category, ReasonCategory::Entry);
        assert_eq!(reason.label, "Entry hold: low liquidity");
    }
}
