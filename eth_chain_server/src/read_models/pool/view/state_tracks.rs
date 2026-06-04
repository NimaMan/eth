use eth_token::pools::classification::PoolClassification;
use eth_price::liquidity::PoolLiquidityLevel;
use eth_token::custody::CustodyState;
use eth_token::erc20::ERC20Token;
use eth_token::pools::{BasePool, PoolLifecycle, TaxBucket, SCAM_DIRECT_LP_LIQUIDITY_REMOVAL};
use serde::Serialize;

use super::{display, CurrentTradingView, LpPoolViewFields, PoolRiskLevel, PoolRiskView};

#[derive(Clone, Debug, Default, Serialize)]
pub struct PoolStateTracksView {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reserve_state: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub route_state: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub valuation: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lp_state: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lp_holders: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custody: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub risk_mechanism: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transfer_policy: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sell_state: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tax_policy: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contract_posture: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub supply_control: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub behavioral_outcomes: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eligibility: Vec<PoolStateBadgeView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence_event: Vec<PoolStateBadgeView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolStateBadgeView {
    pub key: String,
    pub label: String,
    pub tone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn pool_state_tracks(
    token: &ERC20Token,
    base: &BasePool,
    lp_fields: &LpPoolViewFields,
    stage: PoolLifecycle,
    liquidity_level: PoolLiquidityLevel,
    reserve_quality: &display::DisplayReserveQuality,
    current_trading: CurrentTradingView,
    tax_bucket: TaxBucket,
    liquidity_removal: bool,
    liquidity_removal_label: Option<&str>,
    risk: &PoolRiskView,
    scam_mechanism: Option<&str>,
    scam_mechanism_label: Option<&str>,
    classification: &PoolClassification,
) -> PoolStateTracksView {
    let mut tracks = PoolStateTracksView::default();

    tracks.lifecycle.push(badge(
        stage.as_str(),
        lifecycle_label(stage),
        lifecycle_tone(stage),
    ));

    tracks.reserve_state.push(badge(
        liquidity_level.label(),
        reserve_label(liquidity_level),
        reserve_tone(liquidity_level),
    ));

    tracks.route_state.push(badge(
        route_key(current_trading),
        route_label(current_trading),
        route_tone(current_trading),
    ));

    let terminal_zero = liquidity_removal || !reserve_quality.price_ratio_trustworthy;
    tracks.valuation.push(badge(
        if terminal_zero {
            "terminal_zero"
        } else {
            "priced"
        },
        if terminal_zero {
            "Terminal zero"
        } else {
            "Priced"
        },
        if terminal_zero { "bad" } else { "good" },
    ));

    if scam_mechanism == Some(SCAM_DIRECT_LP_LIQUIDITY_REMOVAL) {
        tracks
            .lp_state
            .push(badge("lp_removed", "LP removed", "bad"));
    } else if lp_fields.lp_supply_known {
        tracks
            .lp_state
            .push(badge("lp_observed", "LP observed", "neutral"));
    } else {
        tracks
            .lp_state
            .push(badge("lp_unknown", "LP unknown", "muted"));
    }
    tracks.lp_holders.push(badge(
        &format!("lp_holders_{}", lp_fields.lp_holder_count),
        &format!("{} LP holders", lp_fields.lp_holder_count),
        if lp_fields.lp_holder_count == 0 {
            "warn"
        } else {
            "neutral"
        },
    ));

    let custody_findings = token.custody_findings();
    if !custody_findings.is_empty() {
        let realized = custody_findings
            .iter()
            .any(|finding| finding.state == CustodyState::Realized);
        tracks.custody.push(badge(
            if realized {
                "custody_realized"
            } else {
                "custody_latent"
            },
            if realized {
                "Custody realized"
            } else {
                "Custody latent"
            },
            if realized { "bad" } else { "warn" },
        ));
        for finding in custody_findings {
            tracks.custody.push(badge(
                &format!(
                    "custody_{}_{}",
                    finding.state.as_str(),
                    finding.capability.as_str()
                ),
                finding.capability.label(),
                if finding.state == CustodyState::Realized {
                    "bad"
                } else {
                    "warn"
                },
            ));
        }
    }

    if let Some(label) = scam_mechanism_label
        .or(liquidity_removal_label)
        .or(risk.label.as_deref())
    {
        tracks.risk_mechanism.push(badge(
            scam_mechanism.unwrap_or("risk_mechanism"),
            label,
            "bad",
        ));
    } else if matches!(risk.level, PoolRiskLevel::Honeypot) {
        tracks
            .risk_mechanism
            .push(badge("cannot_sell", "Cannot sell", "bad"));
    } else if matches!(
        risk.level,
        PoolRiskLevel::HighTax | PoolRiskLevel::ExtremeTax
    ) {
        tracks
            .risk_mechanism
            .push(badge(tax_bucket.key(), tax_bucket.key(), "bad"));
    }

    tracks.sell_state.push(badge(
        if current_trading.can_sell {
            "sellable"
        } else {
            "no_sell"
        },
        if current_trading.can_sell {
            "Sellable"
        } else {
            "No sell"
        },
        if current_trading.can_sell {
            "good"
        } else {
            "bad"
        },
    ));

    tracks.tax_policy.push(badge(
        tax_bucket.key(),
        tax_bucket.key(),
        tax_tone(tax_bucket),
    ));

    if classification.eligible {
        tracks.eligibility.push(badge(
            classification
                .eligible_outcome
                .map(|outcome| outcome.key())
                .unwrap_or("eligible"),
            classification
                .eligible_outcome
                .map(|outcome| outcome.label())
                .unwrap_or("Eligible"),
            "good",
        ));
    } else {
        tracks.eligibility.push(badge(
            classification.reason_key.unwrap_or("ineligible"),
            classification.reason_label.unwrap_or("Ineligible"),
            "warn",
        ));
    }

    if liquidity_removal {
        let event_label = if scam_mechanism == Some(SCAM_DIRECT_LP_LIQUIDITY_REMOVAL) {
            "LP removal"
        } else {
            "Liquidity removal"
        };
        tracks
            .evidence_event
            .push(badge("liquidity_removal_evidence", event_label, "bad"));
    }
    if base.has_observed_buy() {
        tracks
            .behavioral_outcomes
            .push(badge("observed_buy", "Observed buy", "neutral"));
    }
    if base.has_observed_sell() {
        tracks
            .behavioral_outcomes
            .push(badge("observed_sell", "Observed sell", "neutral"));
    }

    tracks
}

fn badge(key: &str, label: &str, tone: &str) -> PoolStateBadgeView {
    PoolStateBadgeView {
        key: key.to_string(),
        label: label.to_string(),
        tone: tone.to_string(),
        title: None,
    }
}

fn lifecycle_label(stage: PoolLifecycle) -> &'static str {
    match stage {
        PoolLifecycle::Discovered => "Discovered",
        PoolLifecycle::LiquidityDeposited => "Liquidity deposited",
        PoolLifecycle::Trading => "Trading",
        PoolLifecycle::CannotSell => "Cannot sell",
        PoolLifecycle::Dust => "Dust",
        PoolLifecycle::Drained => "Drained",
        PoolLifecycle::Active => "Active",
        PoolLifecycle::LiquidityRemoved => "Liquidity removed",
        PoolLifecycle::Evicted => "Evicted",
    }
}

fn lifecycle_tone(stage: PoolLifecycle) -> &'static str {
    match stage {
        PoolLifecycle::Trading | PoolLifecycle::Active => "good",
        PoolLifecycle::LiquidityDeposited | PoolLifecycle::Discovered => "neutral",
        PoolLifecycle::Dust | PoolLifecycle::CannotSell => "warn",
        PoolLifecycle::Drained | PoolLifecycle::LiquidityRemoved | PoolLifecycle::Evicted => "bad",
    }
}

fn reserve_label(level: PoolLiquidityLevel) -> &'static str {
    match level {
        PoolLiquidityLevel::Liquid => "Liquid",
        PoolLiquidityLevel::Dust => "Dust",
        PoolLiquidityLevel::Drained => "Drained",
        PoolLiquidityLevel::Unknown => "Unknown quote",
    }
}

fn reserve_tone(level: PoolLiquidityLevel) -> &'static str {
    match level {
        PoolLiquidityLevel::Liquid => "good",
        PoolLiquidityLevel::Unknown => "warn",
        PoolLiquidityLevel::Dust => "warn",
        PoolLiquidityLevel::Drained => "bad",
    }
}

fn route_key(current: CurrentTradingView) -> &'static str {
    match (current.can_buy, current.can_sell) {
        (true, true) => "buy_sell",
        (true, false) => "buy_only",
        (false, true) => "sell_only",
        (false, false) => "no_route",
    }
}

fn route_label(current: CurrentTradingView) -> &'static str {
    match (current.can_buy, current.can_sell) {
        (true, true) => "Buy/Sell",
        (true, false) => "Buy only",
        (false, true) => "Sell only",
        (false, false) => "No route",
    }
}

fn route_tone(current: CurrentTradingView) -> &'static str {
    match (current.can_buy, current.can_sell) {
        (true, true) => "good",
        (false, false) => "bad",
        _ => "warn",
    }
}

fn tax_tone(bucket: TaxBucket) -> &'static str {
    match bucket {
        TaxBucket::Unknown => "muted",
        TaxBucket::NoTax | TaxBucket::LowTax => "good",
        TaxBucket::ModerateTax => "warn",
        TaxBucket::HighTax | TaxBucket::ExtremeTax => "bad",
    }
}
