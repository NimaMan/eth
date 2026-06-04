use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use serde_json::{json, Value};

use super::super::{
    known_infrastructure, PnlAddressPositionExport, PnlCustodyFindingMeta, PnlPoolExport,
    PnlPoolMeta,
};

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const ACCOUNTING_VERSION: &str = "eth_token_pnl_accounting_v1";

impl PnlPoolExport {
    pub fn reconcile_accounting(&mut self, mark_price_denom_per_token: Option<f64>) {
        let retained_counts = retained_movement_counts(self);
        let context = PoolAccountingContext::from_export(self, mark_price_denom_per_token);

        for position in &mut self.address_positions {
            let retained = retained_counts
                .get(&normalize_address(&position.address))
                .copied()
                .unwrap_or(0);
            reconcile_position(position, &context, retained);
        }
    }

    pub fn mark_aggregate_only_accounting(&mut self) {
        for position in &mut self.address_positions {
            position.movement_rows_retained = 0;
            position.movement_rows_backed = position.movement_count == 0;
            position.reconciliation_status = if position.movement_count == 0 {
                "no_movements".to_string()
            } else {
                "aggregate_only".to_string()
            };
            merge_accounting_context(
                &mut position.accounting_context,
                json!({
                    "movement_count": position.movement_count,
                    "movement_rows_retained": 0,
                    "movement_rows_backed": position.movement_rows_backed,
                    "reconciliation_reason": "movement rows intentionally not persisted for aggregate export"
                }),
            );
        }
    }
}

struct PoolAccountingContext {
    pool_id: String,
    token_address: String,
    denom_address: String,
    denom_tracks_native_eth: bool,
    mark_price_denom_per_token: Option<f64>,
    meta: PnlPoolMeta,
    labels: Vec<String>,
    terminal_position_risk: bool,
    terminal_reason: Option<String>,
    dust_or_illiquid: bool,
    dust_reason: Option<String>,
    custody_realized: bool,
    custody_by_victim: BTreeMap<String, CustodyAddressContext>,
}

impl PoolAccountingContext {
    fn from_export(export: &PnlPoolExport, mark_price_denom_per_token: Option<f64>) -> Self {
        let labels = export.meta.pool_labels.clone();
        let labels_lower = labels
            .iter()
            .map(|label| label.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let lifecycle = export
            .meta
            .lifecycle
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let terminal_reason = terminal_reason(&export.meta, &labels_lower, lifecycle.as_str());
        let dust_reason = dust_reason(&labels_lower, lifecycle.as_str());
        let custody_realized = labels_lower.iter().any(|label| {
            label == "custody:realized"
                || label.starts_with("custody:realized:")
                || label == "risk:holder_balance_backdoor_drain"
                || label == "risk:custody_buyer_token_confiscation"
        });
        let custody_by_victim = custody_by_victim(&export.meta.custody_findings);

        Self {
            pool_id: normalize_address(&export.pool_id),
            token_address: normalize_address(&export.token_address),
            denom_address: normalize_address(&export.denom_address),
            denom_tracks_native_eth: normalize_address(&export.denom_address) == WETH_ADDRESS,
            mark_price_denom_per_token,
            meta: export.meta.clone(),
            labels,
            terminal_position_risk: terminal_reason.is_some(),
            terminal_reason,
            dust_or_illiquid: dust_reason.is_some(),
            dust_reason,
            custody_realized: custody_realized || !custody_by_victim.is_empty(),
            custody_by_victim,
        }
    }

    fn custody_for(&self, address: &str) -> Option<&CustodyAddressContext> {
        self.custody_by_victim.get(&normalize_address(address))
    }
}

#[derive(Clone, Debug, Default)]
struct CustodyAddressContext {
    victim_address: String,
    finding_count: u64,
    first_block: Option<u64>,
    drained_amount_scaled: Option<f64>,
    missing_balance: Option<f64>,
    expected_balance: Option<f64>,
    actual_balance: Option<f64>,
    max_drained_fraction: Option<f64>,
    tx_hashes: BTreeSet<String>,
    capabilities: BTreeSet<String>,
    sources: BTreeSet<String>,
    details: BTreeSet<String>,
}

fn reconcile_position(
    position: &mut PnlAddressPositionExport,
    context: &PoolAccountingContext,
    retained_movements: u64,
) {
    let native_costs = native_costs_denom(position, context);
    let realized_cash_pnl = position.denom_cashflow - native_costs;
    let has_token_balance = position.token_balance != Decimal::ZERO;
    let positive_token_balance = position.token_balance > Decimal::ZERO;
    let negative_token_balance = position.token_balance < Decimal::ZERO;
    let custody = context.custody_for(&position.address).cloned();
    let exact_custody_victim = custody.is_some();
    let terminal_zero = has_token_balance && context.terminal_position_risk;

    position.movement_rows_retained = retained_movements;
    position.movement_rows_backed = retained_movements == position.movement_count;
    position.reconciliation_status =
        reconciliation_status(position.movement_count, retained_movements);

    position.position_status = if exact_custody_victim && has_token_balance {
        "confiscated".to_string()
    } else if !has_token_balance {
        "closed".to_string()
    } else if terminal_zero {
        "terminal_zero".to_string()
    } else if context.dust_or_illiquid {
        "dust".to_string()
    } else if negative_token_balance {
        "external_token_source".to_string()
    } else {
        "open".to_string()
    };

    position.valuation_status = if !has_token_balance {
        "not_required".to_string()
    } else if terminal_zero {
        "terminal_zero".to_string()
    } else if context.dust_or_illiquid {
        "dust_illiquid".to_string()
    } else if position.marked_token_value_denom.is_some() {
        "priced".to_string()
    } else {
        "no_mark".to_string()
    };

    position.unrealized_value_denom = if !has_token_balance || terminal_zero {
        Some(Decimal::ZERO)
    } else if context.dust_or_illiquid {
        None
    } else {
        position.marked_token_value_denom
    };

    position.realized_pnl_denom = if !has_token_balance || terminal_zero {
        Some(realized_cash_pnl)
    } else {
        None
    };

    position.total_pnl_denom = if !has_token_balance || terminal_zero {
        Some(realized_cash_pnl)
    } else if context.dust_or_illiquid {
        None
    } else {
        position.pnl_proxy_denom
    };

    let actor_roles = actor_roles(
        position,
        context,
        exact_custody_victim,
        positive_token_balance,
        negative_token_balance,
    );
    position.is_user_candidate = is_user_candidate(&actor_roles);
    let mut actor_roles = actor_roles;
    if position.is_user_candidate {
        actor_roles.insert("user_candidate".to_string());
    }
    position.actor_roles = actor_roles.into_iter().collect();

    let notes = accounting_notes(position, context);
    let custody_context = custody
        .as_ref()
        .map(|custody| custody_accounting_context(custody, position))
        .unwrap_or_else(|| {
            json!({
                "status": if positive_token_balance && context.custody_realized {
                    "candidate"
                } else {
                    "none"
                },
                "confiscated": false
            })
        });
    position.accounting_context = json!({
        "version": ACCOUNTING_VERSION,
        "position_status": position.position_status,
        "valuation_status": position.valuation_status,
        "reconciliation_status": position.reconciliation_status,
        "terminal_reason": context.terminal_reason,
        "dust_reason": context.dust_reason,
        "custody_realized": context.custody_realized,
        "custody": custody_context,
        "denom_tracks_native_eth": context.denom_tracks_native_eth,
        "native_costs_denom": native_costs.to_string(),
        "mark_price_denom_per_token": context.mark_price_denom_per_token,
        "movement_count": position.movement_count,
        "movement_rows_retained": retained_movements,
        "movement_rows_backed": position.movement_rows_backed,
        "pool_labels": context.labels,
        "scam_mechanism": context.meta.scam_mechanism,
        "notes": notes,
    });
}

fn retained_movement_counts(export: &PnlPoolExport) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for movement in &export.movements {
        *counts
            .entry(normalize_address(&movement.address))
            .or_insert(0) += 1;
    }
    counts
}

fn reconciliation_status(movement_count: u64, retained_movements: u64) -> String {
    if movement_count == 0 {
        "no_movements".to_string()
    } else if retained_movements == movement_count {
        "movement_backed".to_string()
    } else if retained_movements == 0 {
        "aggregate_only".to_string()
    } else if retained_movements < movement_count {
        "partial_movements".to_string()
    } else {
        "mismatch".to_string()
    }
}

fn native_costs_denom(
    position: &PnlAddressPositionExport,
    context: &PoolAccountingContext,
) -> Decimal {
    if context.denom_tracks_native_eth {
        position.native_fee + position.native_priority_fee
    } else {
        Decimal::ZERO
    }
}

fn actor_roles(
    position: &PnlAddressPositionExport,
    context: &PoolAccountingContext,
    exact_custody_victim: bool,
    positive_token_balance: bool,
    negative_token_balance: bool,
) -> BTreeSet<String> {
    let mut roles = BTreeSet::new();
    let address = normalize_address(&position.address);

    if address == ZERO_ADDRESS {
        roles.insert("zero_address".to_string());
    }
    if address == context.pool_id {
        roles.insert("pool".to_string());
    }
    if address == context.token_address {
        roles.insert("token_contract".to_string());
    }
    if address == context.denom_address {
        roles.insert("denom_contract".to_string());
    }
    if known_infrastructure().contains(&address) {
        roles.insert("infrastructure".to_string());
    }
    if same_optional_address(&address, context.meta.token_creator_address.as_deref()) {
        roles.insert("token_creator".to_string());
    }
    if same_optional_address(&address, context.meta.pool_creator_address.as_deref()) {
        roles.insert("pool_creator".to_string());
    }
    if exact_custody_victim {
        roles.insert("custody_victim".to_string());
    }
    if positive_token_balance && context.custody_realized {
        roles.insert("custody_victim_candidate".to_string());
    }
    if negative_token_balance {
        roles.insert("external_token_source".to_string());
    }
    if position.denom_cashflow < Decimal::ZERO && positive_token_balance {
        roles.insert("buyer".to_string());
    }
    if position.denom_cashflow > Decimal::ZERO && negative_token_balance {
        roles.insert("seller".to_string());
    }
    roles
}

fn is_user_candidate(roles: &BTreeSet<String>) -> bool {
    !roles.iter().any(|role| {
        matches!(
            role.as_str(),
            "zero_address"
                | "pool"
                | "token_contract"
                | "denom_contract"
                | "infrastructure"
                | "token_creator"
                | "pool_creator"
        )
    })
}

fn accounting_notes(
    position: &PnlAddressPositionExport,
    context: &PoolAccountingContext,
) -> Vec<String> {
    let mut notes = Vec::new();
    if context.custody_for(&position.address).is_some() {
        notes.push(
            "address has trace-backed or reconciled custody confiscation evidence".to_string(),
        );
    }
    if context.terminal_position_risk {
        notes.push(
            "pool has terminal position risk; open token balance is valued at zero".to_string(),
        );
    }
    if context.dust_or_illiquid {
        notes.push("pool is dust or illiquid; mark value is not trusted".to_string());
    }
    if position.reconciliation_status == "partial_movements" {
        notes.push("retained movement rows do not fully cover aggregate counters".to_string());
    } else if position.reconciliation_status == "aggregate_only" {
        notes.push("aggregate counters are present without persisted movement rows".to_string());
    }
    if !context.denom_tracks_native_eth
        && (position.native_fee != Decimal::ZERO || position.native_priority_fee != Decimal::ZERO)
    {
        notes.push(
            "native gas costs are tracked but not subtracted from non-ETH denom PnL".to_string(),
        );
    }
    notes
}

fn custody_by_victim(
    findings: &[PnlCustodyFindingMeta],
) -> BTreeMap<String, CustodyAddressContext> {
    let mut by_victim = BTreeMap::<String, CustodyAddressContext>::new();
    for finding in findings {
        let victim = normalize_address(&finding.victim_address);
        if victim.is_empty() {
            continue;
        }
        let entry = by_victim
            .entry(victim.clone())
            .or_insert_with(|| CustodyAddressContext {
                victim_address: victim,
                ..Default::default()
            });
        entry.finding_count = entry.finding_count.saturating_add(1);
        if let Some(block) = finding.block_number {
            entry.first_block = Some(
                entry
                    .first_block
                    .map_or(block, |existing| existing.min(block)),
            );
        }
        entry.drained_amount_scaled =
            sum_optional_f64(entry.drained_amount_scaled, finding.amount_scaled);
        entry.missing_balance = sum_optional_f64(entry.missing_balance, finding.missing_balance);
        entry.expected_balance = max_optional_f64(entry.expected_balance, finding.expected_balance);
        entry.actual_balance = min_optional_f64(entry.actual_balance, finding.actual_balance);
        entry.max_drained_fraction =
            max_optional_f64(entry.max_drained_fraction, finding.drained_fraction);
        if let Some(tx_hash) = finding.tx_hash.as_deref() {
            if !tx_hash.trim().is_empty() {
                entry.tx_hashes.insert(tx_hash.to_ascii_lowercase());
            }
        }
        if !finding.capability.trim().is_empty() {
            entry.capabilities.insert(finding.capability.clone());
        }
        if let Some(source) = finding.source.as_deref() {
            if !source.trim().is_empty() {
                entry.sources.insert(source.to_string());
            }
        }
        if let Some(detail) = finding.detail.as_deref() {
            if !detail.trim().is_empty() {
                entry.details.insert(detail.to_string());
            }
        }
    }
    by_victim
}

fn custody_accounting_context(
    custody: &CustodyAddressContext,
    position: &PnlAddressPositionExport,
) -> Value {
    let position_token_balance = decimal_to_positive_f64(position.token_balance);
    let expected_balance = custody.expected_balance.or(position_token_balance);
    let missing_balance = custody.missing_balance.or(custody.drained_amount_scaled);
    let actual_balance = custody.actual_balance.or_else(|| {
        expected_balance
            .zip(missing_balance)
            .map(|(expected, missing)| (expected - missing).max(0.0))
    });
    let max_drained_fraction = custody.max_drained_fraction.or_else(|| {
        expected_balance
            .zip(missing_balance)
            .and_then(|(expected, missing)| {
                if expected > 0.0 {
                    Some((missing / expected).clamp(0.0, 1.0))
                } else {
                    None
                }
            })
    });

    json!({
        "status": "confiscated",
        "confiscated": true,
        "victim_address": custody.victim_address,
        "finding_count": custody.finding_count,
        "first_block": custody.first_block,
        "drained_amount_scaled": custody.drained_amount_scaled,
        "missing_balance": missing_balance,
        "expected_balance": expected_balance,
        "actual_balance": actual_balance,
        "max_drained_fraction": max_drained_fraction,
        "tx_hashes": custody.tx_hashes.iter().cloned().collect::<Vec<_>>(),
        "capabilities": custody.capabilities.iter().cloned().collect::<Vec<_>>(),
        "sources": custody.sources.iter().cloned().collect::<Vec<_>>(),
        "details": custody.details.iter().cloned().collect::<Vec<_>>(),
    })
}

fn decimal_to_positive_f64(value: Decimal) -> Option<f64> {
    if value <= Decimal::ZERO {
        return None;
    }
    value.to_string().parse::<f64>().ok()
}

fn sum_optional_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn max_optional_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn min_optional_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn terminal_reason(meta: &PnlPoolMeta, labels_lower: &[String], lifecycle: &str) -> Option<String> {
    if labels_lower
        .iter()
        .any(|label| label == "risk:terminal_position")
    {
        return Some("pool_label:risk:terminal_position".to_string());
    }
    if let Some(mechanism) = meta.scam_mechanism.as_deref() {
        if !mechanism.trim().is_empty() {
            return Some(format!("scam_mechanism:{mechanism}"));
        }
    }
    if meta.is_scam {
        return Some("pool_meta:is_scam".to_string());
    }
    if matches!(
        lifecycle,
        "liquidity_removed" | "drained" | "evicted" | "cannot_sell"
    ) {
        return Some(format!("lifecycle:{lifecycle}"));
    }
    if meta.can_buy && !meta.can_sell {
        return Some("route:cannot_sell".to_string());
    }
    if labels_lower.iter().any(|label| {
        label == "route:not_economic_sellable"
            || label == "liquidity:reserve_removed"
            || label == "liquidity:legacy_terminal_flag"
            || label == "risk:holder_balance_backdoor_drain"
            || label == "risk:custody_buyer_token_confiscation"
            || label == "risk:pair_balance_backdoor_drain"
    }) {
        return Some("pool_label:terminal_or_illiquid".to_string());
    }
    None
}

fn dust_reason(labels_lower: &[String], lifecycle: &str) -> Option<String> {
    if lifecycle == "dust" {
        return Some("lifecycle:dust".to_string());
    }
    if labels_lower.iter().any(|label| label == "liquidity:dust") {
        return Some("pool_label:liquidity:dust".to_string());
    }
    None
}

fn same_optional_address(address: &str, other: Option<&str>) -> bool {
    other
        .map(|other| address == normalize_address(other))
        .unwrap_or(false)
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn merge_accounting_context(target: &mut Value, patch: Value) {
    if !target.is_object() {
        *target = json!({});
    }
    let Some(target) = target.as_object_mut() else {
        return;
    };
    let Some(patch) = patch.as_object() else {
        return;
    };
    for (key, value) in patch {
        target.insert(key.clone(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    use super::*;
    use crate::pnl::{PnlConservationExport, PnlPoolMeta};

    #[test]
    fn terminal_pool_zeroes_open_unrealized_value() {
        let mut export = pool_export(position("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        export.meta = PnlPoolMeta {
            is_scam: true,
            scam_mechanism: Some("holder_balance_backdoor_drain".to_string()),
            pool_labels: vec![
                "risk:terminal_position".to_string(),
                "custody:realized".to_string(),
            ],
            ..Default::default()
        };

        export.reconcile_accounting(Some(1.0));

        let row = &export.address_positions[0];
        assert_eq!(row.position_status, "terminal_zero");
        assert_eq!(row.valuation_status, "terminal_zero");
        assert_eq!(row.unrealized_value_denom, Some(Decimal::ZERO));
        assert_eq!(
            row.total_pnl_denom,
            Some(Decimal::from_str("-1.000000000000000010").unwrap())
        );
        assert!(row
            .actor_roles
            .contains(&"custody_victim_candidate".to_string()));
    }

    #[test]
    fn exact_custody_finding_marks_address_confiscated() {
        let victim = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut export = pool_export(position(victim));
        export.meta = PnlPoolMeta {
            is_scam: true,
            scam_mechanism: Some("holder_balance_backdoor_drain".to_string()),
            pool_labels: vec![
                "risk:terminal_position".to_string(),
                "custody:realized".to_string(),
            ],
            custody_findings: vec![crate::pnl::PnlCustodyFindingMeta {
                victim_address: victim.to_string(),
                capability: "custody_burn_drain".to_string(),
                state: "realized".to_string(),
                block_number: Some(10),
                tx_hash: Some("0xdeadbeef".to_string()),
                amount_raw: Some("1000000000".to_string()),
                amount_scaled: Some(1.0),
                expected_balance: None,
                actual_balance: None,
                missing_balance: None,
                drained_fraction: None,
                source: Some("balance_reconciliation".to_string()),
                detail: Some(
                    "holder_balance_missing_from_state_without_transfer_event".to_string(),
                ),
            }],
            ..Default::default()
        };

        export.reconcile_accounting(Some(1.0));

        let row = &export.address_positions[0];
        assert_eq!(row.position_status, "confiscated");
        assert_eq!(row.valuation_status, "terminal_zero");
        assert!(row.actor_roles.contains(&"custody_victim".to_string()));
        assert!(row
            .actor_roles
            .contains(&"custody_victim_candidate".to_string()));
        assert_eq!(
            row.accounting_context["custody"]["status"].as_str(),
            Some("confiscated")
        );
        assert_eq!(
            row.accounting_context["custody"]["first_block"].as_u64(),
            Some(10)
        );
        assert_eq!(
            row.accounting_context["custody"]["drained_amount_scaled"].as_f64(),
            Some(1.0)
        );
        assert_eq!(
            row.accounting_context["custody"]["expected_balance"].as_f64(),
            Some(1.0)
        );
        assert_eq!(
            row.accounting_context["custody"]["actual_balance"].as_f64(),
            Some(0.0)
        );
        assert_eq!(
            row.accounting_context["custody"]["missing_balance"].as_f64(),
            Some(1.0)
        );
        assert_eq!(
            row.accounting_context["custody"]["max_drained_fraction"].as_f64(),
            Some(1.0)
        );
        assert_eq!(row.unrealized_value_denom, Some(Decimal::ZERO));
    }

    #[test]
    fn aggregate_only_marker_overrides_movement_backing() {
        let mut export = pool_export(position("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        export.reconcile_accounting(Some(1.0));
        assert_eq!(
            export.address_positions[0].reconciliation_status,
            "aggregate_only"
        );

        export.movements.push(crate::pnl::PnlMovementExport {
            entry_index: 0,
            tx_hash: "0x01".to_string(),
            block_number: 1,
            block_timestamp: 1,
            tx_index: 0,
            log_index: None,
            address: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            kind: "token_in".to_string(),
            token_in_raw: "1".to_string(),
            token_out_raw: "0".to_string(),
            denom_in_raw: "0".to_string(),
            denom_out_raw: "0".to_string(),
            native_fee_raw: "0".to_string(),
            native_priority_fee_raw: "0".to_string(),
            pool_direct: true,
        });
        export.reconcile_accounting(Some(1.0));
        assert_eq!(
            export.address_positions[0].reconciliation_status,
            "partial_movements"
        );

        export.mark_aggregate_only_accounting();
        assert_eq!(
            export.address_positions[0].reconciliation_status,
            "aggregate_only"
        );
        assert_eq!(export.address_positions[0].movement_rows_retained, 0);
    }

    fn position(address: &str) -> PnlAddressPositionExport {
        PnlAddressPositionExport {
            address: address.to_string(),
            token_in_raw: "1000000000".to_string(),
            token_out_raw: "0".to_string(),
            denom_in_raw: "0".to_string(),
            denom_out_raw: "1000000000000000000".to_string(),
            native_fee_raw: "10".to_string(),
            native_priority_fee_raw: "0".to_string(),
            first_block: Some(1),
            latest_block: Some(2),
            movement_count: 2,
            token_balance_raw: "1000000000".to_string(),
            denom_cashflow_raw: "-1000000000000000000".to_string(),
            token_balance: Decimal::ONE,
            denom_cashflow: Decimal::NEGATIVE_ONE,
            native_fee: Decimal::from_str("0.000000000000000010").unwrap(),
            native_priority_fee: Decimal::ZERO,
            marked_token_value_denom: Some(Decimal::ONE),
            pnl_proxy_denom: Some(Decimal::ZERO),
            position_status: "unknown".to_string(),
            valuation_status: "unknown".to_string(),
            reconciliation_status: "unknown".to_string(),
            realized_pnl_denom: None,
            unrealized_value_denom: None,
            total_pnl_denom: None,
            movement_rows_retained: 0,
            movement_rows_backed: false,
            actor_roles: Vec::new(),
            is_user_candidate: false,
            accounting_context: json!({}),
        }
    }

    fn pool_export(position: PnlAddressPositionExport) -> PnlPoolExport {
        PnlPoolExport {
            pool_id: "0x3333333333333333333333333333333333333333".to_string(),
            token_address: "0x1111111111111111111111111111111111111111".to_string(),
            denom_address: WETH_ADDRESS.to_string(),
            protocol: Some("uniswap_v2".to_string()),
            token_decimals: 9,
            denom_decimals: 18,
            tx_count: 1,
            latest_block_number: Some(2),
            latest_block_timestamp: Some(1_700),
            conservation: PnlConservationExport::default(),
            address_positions: vec![position],
            movements: Vec::new(),
            meta: PnlPoolMeta::default(),
        }
    }
}
