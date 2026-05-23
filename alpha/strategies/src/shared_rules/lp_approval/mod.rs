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

pub const DEFAULT_GATE_MIN_APPROVED_PCT: u64 = 30;

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
    approval_pct_from_message(&event.message)
}

pub fn approval_trading_enabled_age_blocks(event: &RiskEvent) -> Option<i64> {
    approval_trading_enabled_age_blocks_from_message(&event.message)
}

pub fn approval_trading_enabled_age_blocks_from_message(message: &str) -> Option<i64> {
    [
        "trading_enabled_to_last_lp_approval_chain_block_delta=",
        "trading_enabled_to_lp_approval_chain_block_delta=",
        "pool_age_at_lp_approval_chain_block_delta=",
        "approval_age_chain_block_delta=",
    ]
    .into_iter()
    .find_map(|key| parse_i64_after_key(message, key))
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
}
