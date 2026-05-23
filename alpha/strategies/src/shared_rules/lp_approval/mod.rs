//! LP approval gates shared by entry and exit rules.
//!
//! The same evidence has two valid uses:
//! - before entry, a large approval blocks a new buy;
//! - after entry, a large approval exits an open position.

pub mod entry_gate;
pub mod exit_gate;

use std::str::FromStr;

use eth_alpha_core::{
    ids::{PoolAddress, TokenAddress},
    risk::{RiskEvent, RiskKind},
};
use rust_decimal::Decimal;
use serde_json::Value;

pub const DEFAULT_GATE_MIN_APPROVED_PCT: u64 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalAgeEvidence {
    pub age_blocks: i64,
    pub basis: &'static str,
    pub reference_block: Option<u64>,
    pub observed_block: Option<u64>,
}

pub fn matches_token_pool(
    event: &RiskEvent,
    token_address: TokenAddress,
    pool_address: &PoolAddress,
) -> bool {
    event.kind == RiskKind::LpApproval
        && event.token_address == token_address
        && event
            .pool_address
            .as_ref()
            .map(|pool| pool == pool_address)
            .unwrap_or(true)
}

pub fn approval_exceeds_threshold(event: &RiskEvent, min_pct: Option<Decimal>) -> bool {
    if event.kind != RiskKind::LpApproval {
        return false;
    }

    let Some(min_pct) = min_pct else {
        return true;
    };

    approval_pct(event)
        .map(|pct| pct > min_pct)
        .unwrap_or(false)
}

pub fn approval_pct(event: &RiskEvent) -> Option<Decimal> {
    approval_pct_from_evidence(event).or_else(|| approval_pct_from_message(&event.message))
}

pub fn approval_trading_enabled_age_blocks(event: &RiskEvent) -> Option<i64> {
    approval_age_evidence(event).map(|evidence| evidence.age_blocks)
}

pub fn approval_age_evidence(event: &RiskEvent) -> Option<ApprovalAgeEvidence> {
    approval_age_evidence_from_event_payload(event)
        .or_else(|| approval_age_evidence_from_message(&event.message, event.observed_block))
}

pub fn approval_trading_enabled_age_blocks_from_message(message: &str) -> Option<i64> {
    approval_age_evidence_from_message(message, None).map(|evidence| evidence.age_blocks)
}

fn approval_age_evidence_from_event_payload(event: &RiskEvent) -> Option<ApprovalAgeEvidence> {
    let evidence = event.evidence.as_ref()?;
    if let Some(age_blocks) = evidence_i64(evidence, "trading_enabled_age_blocks_at_signal") {
        return Some(ApprovalAgeEvidence {
            age_blocks,
            basis: "trading_enabled_block",
            reference_block: evidence_u64(evidence, "trading_enabled_block"),
            observed_block: evidence_u64(evidence, "observed_block").or(event.observed_block),
        });
    }
    evidence_i64(evidence, "pool_age_blocks_at_signal").map(|age_blocks| ApprovalAgeEvidence {
        age_blocks,
        basis: "pool_creation_block",
        reference_block: evidence_u64(evidence, "pool_creation_block"),
        observed_block: evidence_u64(evidence, "observed_block").or(event.observed_block),
    })
}

fn approval_age_evidence_from_message(
    message: &str,
    observed_block: Option<u64>,
) -> Option<ApprovalAgeEvidence> {
    [
        "trading_enabled_to_last_lp_approval_chain_block_delta=",
        "trading_enabled_to_lp_approval_chain_block_delta=",
        "approval_age_chain_block_delta=",
    ]
    .into_iter()
    .find_map(|key| parse_i64_after_key(message, key))
    .map(|age_blocks| ApprovalAgeEvidence {
        age_blocks,
        basis: "trading_enabled_block",
        reference_block: None,
        observed_block,
    })
    .or_else(|| {
        parse_i64_after_key(message, "pool_age_at_lp_approval_chain_block_delta=").map(
            |age_blocks| ApprovalAgeEvidence {
                age_blocks,
                basis: "pool_creation_block",
                reference_block: None,
                observed_block,
            },
        )
    })
}

pub fn approval_pct_from_message(message: &str) -> Option<Decimal> {
    [
        "generic_approved_pct=",
        "approved_pct=",
        "lp_approved_pct=",
        "lp_approved_pct_as_of=",
        "lp_router_approved_pct=",
    ]
    .into_iter()
    .find_map(|key| parse_pct_after_key(message, key))
}

fn approval_pct_from_evidence(event: &RiskEvent) -> Option<Decimal> {
    let evidence = event.evidence.as_ref()?;
    [
        "approved_share_pct",
        "approval_percentage",
        "position_share_pct",
    ]
    .into_iter()
    .find_map(|key| evidence_decimal(evidence, key))
}

fn evidence_decimal(evidence: &Value, key: &str) -> Option<Decimal> {
    match evidence.get(key)? {
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        Value::String(text) => parse_decimal_prefix(text),
        _ => None,
    }
}

fn evidence_i64(evidence: &Value, key: &str) -> Option<i64> {
    match evidence.get(key)? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => parse_i64_prefix(text),
        _ => None,
    }
}

fn evidence_u64(evidence: &Value, key: &str) -> Option<u64> {
    match evidence.get(key)? {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => parse_i64_prefix(text).and_then(|value| u64::try_from(value).ok()),
        _ => None,
    }
}

fn parse_pct_after_key(message: &str, key: &str) -> Option<Decimal> {
    let (_, suffix) = message.split_once(key)?;
    parse_decimal_prefix(suffix)
}

fn parse_decimal_prefix(value: &str) -> Option<Decimal> {
    let trimmed = value.trim_start();
    if trimmed
        .get(..7)
        .map(|prefix| prefix.eq_ignore_ascii_case("unknown"))
        .unwrap_or(false)
    {
        return None;
    }

    let end = trimmed
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+'))
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }

    Decimal::from_str(&trimmed[..end]).ok()
}

fn parse_i64_after_key(message: &str, key: &str) -> Option<i64> {
    let (_, suffix) = message.split_once(key)?;
    parse_i64_prefix(suffix)
}

fn parse_i64_prefix(value: &str) -> Option<i64> {
    let trimmed = value.trim_start();
    let end = trimmed
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '-' || ch == '+'))
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }

    i64::from_str(&trimmed[..end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generic_approved_pct_from_risk_message() {
        let pct = approval_pct_from_message(
            "risk atlas mined-chain LP approval: count=1, generic_approved_pct=45.25%, owner_is_creator=true",
        );

        assert_eq!(pct, Some(Decimal::new(4525, 2)));
    }

    #[test]
    fn parses_plain_approved_pct_from_risk_message() {
        let pct = approval_pct_from_message("lp approval approved_pct=31.0%, source=mempool");

        assert_eq!(pct, Some(Decimal::new(310, 1)));
    }

    #[test]
    fn unknown_pct_is_not_threshold_evidence() {
        let pct = approval_pct_from_message(
            "risk atlas mined-chain LP approval: count=1, generic_approved_pct=unknown",
        );

        assert_eq!(pct, None);
    }

    #[test]
    fn parses_trading_enabled_to_last_lp_approval_delta() {
        let age = approval_trading_enabled_age_blocks_from_message(
            "risk atlas mined-chain LP approval: count=1, generic_approved_pct=100.00%, trading_enabled_to_last_lp_approval_chain_block_delta=2",
        );

        assert_eq!(age, Some(2));
    }

    #[test]
    fn reads_lp_approval_pct_and_age_from_structured_evidence() {
        let event = RiskEvent {
            kind: RiskKind::LpApproval,
            severity: eth_alpha_core::risk::RiskSeverity::Critical,
            source: Some("mempool_signal".to_string()),
            token_address: TokenAddress::repeat_byte(0x11),
            pool_address: None,
            pending_tx_hash: None,
            observed_block: Some(120),
            message: "lp approval".to_string(),
            evidence: Some(serde_json::json!({
                "approved_share_pct": 100.0,
                "trading_enabled_block": 118,
                "trading_enabled_age_blocks_at_signal": 2,
                "observed_block": 120,
            })),
        };

        assert_eq!(approval_pct(&event), Some(Decimal::from(100)));
        assert_eq!(approval_trading_enabled_age_blocks(&event), Some(2));
        assert_eq!(
            approval_age_evidence(&event),
            Some(ApprovalAgeEvidence {
                age_blocks: 2,
                basis: "trading_enabled_block",
                reference_block: Some(118),
                observed_block: Some(120),
            })
        );
    }
}
