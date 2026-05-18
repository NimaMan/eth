use super::model::{DecisionRow, EventPoint, RiskEventRow, SnapshotRow, TradeEventRow};

pub fn build_timeline(
    trade_events: &[TradeEventRow],
    risk_events: &[RiskEventRow],
    decisions: &[DecisionRow],
    snapshots: &[SnapshotRow],
) -> Vec<EventPoint> {
    let mut points = Vec::new();

    for event in trade_events {
        points.push(EventPoint {
            block_number: event.block_number,
            source: "trade_event".to_string(),
            kind: event.event_type.clone(),
            label: format!("{} {}", event.order_side, event.status),
            detail: trade_event_detail(event),
            value_eth: event.filled_amount_raw.as_ref().map(|raw| {
                format!(
                    "{} raw / {} decimals",
                    raw,
                    event
                        .filled_amount_decimals
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string())
                )
            }),
            pnl_eth: None,
        });
    }

    for event in risk_events {
        points.push(EventPoint {
            block_number: event.observed_block,
            source: "risk".to_string(),
            kind: event.kind.clone(),
            label: event.severity.clone(),
            detail: event.message.clone(),
            value_eth: None,
            pnl_eth: None,
        });
    }

    for decision in decisions {
        points.push(EventPoint {
            block_number: decision.block_number,
            source: "decision".to_string(),
            kind: decision.action.clone(),
            label: decision.reason.clone().unwrap_or_else(|| "-".to_string()),
            detail: format!(
                "{} {} {}",
                decision.event_source,
                decision.event_key,
                decision.order_side.clone().unwrap_or_default()
            )
            .trim()
            .to_string(),
            value_eth: None,
            pnl_eth: None,
        });
    }

    for snapshot in snapshots {
        points.push(EventPoint {
            block_number: Some(snapshot.block_number),
            source: "snapshot".to_string(),
            kind: snapshot.state.clone(),
            label: snapshot_label(snapshot),
            detail: snapshot_detail(snapshot),
            value_eth: Some(snapshot.current_value_eth.clone()),
            pnl_eth: Some(snapshot.total_pnl_eth.clone()),
        });
    }

    points.sort_by(|left, right| {
        left.block_number
            .unwrap_or(i64::MAX)
            .cmp(&right.block_number.unwrap_or(i64::MAX))
            .then_with(|| source_order(&left.source).cmp(&source_order(&right.source)))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.label.cmp(&right.label))
    });
    points
}

fn trade_event_detail(event: &TradeEventRow) -> String {
    let mut parts = vec![format!("order={}", event.order_id)];
    if let Some(gas) = &event.gas_cost_eth {
        parts.push(format!("gas={gas} ETH"));
    }
    if let Some(error) = &event.error {
        parts.push(format!("error={error}"));
    }
    parts.join(", ")
}

fn snapshot_label(snapshot: &SnapshotRow) -> String {
    format!(
        "value={} pnl={} roi={}",
        snapshot.current_value_eth, snapshot.total_pnl_eth, snapshot.roi
    )
}

fn snapshot_detail(snapshot: &SnapshotRow) -> String {
    let mut parts = Vec::new();
    if let Some(block) = snapshot.observed_block_number {
        parts.push(format!("observed={block}"));
    }
    if let Some(block) = snapshot.valuation_block_number {
        parts.push(format!("valuation={block}"));
    }
    if let Some(price_ratio) = &snapshot.pool_price_to_initial_price_ratio {
        parts.push(format!("price_ratio={price_ratio}"));
    }
    if let Some(liquidity) = &snapshot.pool_liquidity_denom {
        parts.push(format!("liquidity={liquidity}"));
    }
    parts.join(", ")
}

fn source_order(source: &str) -> u8 {
    match source {
        "trade_event" => 0,
        "risk" => 1,
        "decision" => 2,
        "snapshot" => 3,
        _ => 9,
    }
}
