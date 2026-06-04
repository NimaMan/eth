use super::*;

pub(super) fn build_pool_rollups(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    risks: &[RiskRecord],
    report_by_order: &HashMap<String, Vec<ExecutionReportRecord>>,
    snapshot_by_position: &HashMap<String, SnapshotRecord>,
    generated_at_unix_secs: u64,
) -> Vec<PoolRollup> {
    let mut pools: HashMap<(String, String), PoolRollup> = HashMap::new();
    let mut token_to_pool_keys: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for position in positions {
        let key = pool_key(&position.token_address, &position.pool_address);
        let position_has_exposure = is_exposure_state(&position.state);
        token_to_pool_keys
            .entry(key.0.clone())
            .or_default()
            .push(key.clone());
        let rollup = pools
            .entry(key)
            .or_insert_with(|| PoolRollup::position(position));
        if is_terminal_state(&position.state) {
            rollup.updated_epoch = Some(position.updated_epoch);
        } else {
            rollup.updated_epoch = Some(generated_at_unix_secs as i64);
        }
        rollup.has_exposure |= position_has_exposure;
        let mut realized_from_reports = false;
        if let Some(order_id) = position.entry_order_id.as_deref() {
            if let Some(reports) = report_by_order.get(order_id) {
                let entry_value_eth = rollup.add_reports(reports, ReportSide::Buy);
                if position_has_exposure {
                    rollup.exposure_entry_cost_eth += entry_value_eth;
                }
                if let Some(exit_order_id) = position.exit_order_id.as_deref() {
                    if let Some(exit_reports) = report_by_order.get(exit_order_id) {
                        let exit_value_eth = confirmed_filled_eth(exit_reports);
                        if entry_value_eth > 0.0 && exit_value_eth > 0.0 {
                            rollup.add_realized_pnl(entry_value_eth, exit_value_eth);
                            realized_from_reports = true;
                        }
                    }
                }
            }
        }
        if let Some(order_id) = position.exit_order_id.as_deref() {
            if let Some(reports) = report_by_order.get(order_id) {
                rollup.add_reports(reports, ReportSide::Sell);
            }
        }
        if let Some(snapshot) = snapshot_by_position.get(&position.position_id) {
            if realized_from_reports {
                // Snapshot realized PnL already captured from execution reports;
                // only use snapshot for current value and unrealized PnL.
                rollup.snapshot_count += 1;
                rollup.current_value_eth += snapshot.current_value_eth.unwrap_or(0.0);
                rollup.unrealized_pnl_eth += snapshot.unrealized_profit_eth.unwrap_or(0.0);
            } else {
                rollup.add_snapshot(snapshot);
            }
            if position_has_exposure {
                rollup.exposure_current_value_eth += snapshot.current_value_eth.unwrap_or(0.0);
                rollup.exposure_unrealized_pnl_eth += snapshot.unrealized_profit_eth.unwrap_or(0.0);
            }
        }
    }

    for order in orders {
        let key = pool_key(&order.token_address, &order.pool_address);
        pools
            .entry(key)
            .or_insert_with(|| PoolRollup::order_only(order))
            .add_order(order);
    }

    for risk in risks {
        let keys = if let Some(pool_address) = risk.pool_address.as_deref() {
            vec![pool_key(&risk.token_address, pool_address)]
        } else {
            token_to_pool_keys
                .get(&normalize_key(&risk.token_address))
                .cloned()
                .filter(|keys| !keys.is_empty())
                .unwrap_or_else(|| vec![pool_key(&risk.token_address, "")])
        };
        for key in keys {
            pools
                .entry(key)
                .or_insert_with(|| PoolRollup::risk_only(risk))
                .add_risk(risk);
        }
    }

    pools.into_values().collect()
}

pub(super) fn build_totals(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    reports: &[ExecutionReportRecord],
    risks: &[RiskRecord],
    pools: &[PoolRollup],
) -> StrategyPerformanceTotals {
    let open_positions = positions
        .iter()
        .filter(|position| !is_terminal_state(&position.state))
        .count() as u64;
    let exposure_positions = positions
        .iter()
        .filter(|position| is_exposure_state(&position.state))
        .count() as u64;
    let failed_exit_exposure_positions = positions
        .iter()
        .filter(|position| is_failed_exit_exposure_state(&position.state))
        .count() as u64;
    let buy_orders = orders.iter().filter(|order| order.side == "buy").count() as u64;
    let sell_orders = orders.iter().filter(|order| order.side == "sell").count() as u64;
    let confirmed_reports = reports
        .iter()
        .filter(|report| report.status == "confirmed")
        .count() as u64;
    let failed_reports = reports
        .iter()
        .filter(|report| is_failed_report_status(&report.status))
        .count() as u64;
    let critical_risk_events = risks
        .iter()
        .filter(|risk| risk.severity == "critical")
        .count() as u64;

    let capital_deployed_eth = pools
        .iter()
        .map(|pool| pool.capital_deployed_eth)
        .sum::<f64>();
    let buy_volume_eth = pools.iter().map(|pool| pool.buy_volume_eth).sum::<f64>();
    let sell_volume_eth = pools.iter().map(|pool| pool.sell_volume_eth).sum::<f64>();
    let pools_with_snapshots = pools.iter().filter(|pool| pool.snapshot_count > 0).count() as u64;
    let pools_with_pnl = pools
        .iter()
        .filter(|pool| pool.pnl_observation_count > 0)
        .count() as u64;
    let has_snapshots = pools_with_snapshots > 0;
    let has_pnl = pools_with_pnl > 0;
    let current_value_eth = pools.iter().map(|pool| pool.current_value_eth).sum::<f64>();
    let exposure_entry_cost_eth = pools
        .iter()
        .map(|pool| pool.exposure_entry_cost_eth)
        .sum::<f64>();
    let exposure_current_value_eth = pools
        .iter()
        .map(|pool| pool.exposure_current_value_eth)
        .sum::<f64>();
    let exposure_unrealized_pnl_eth = pools
        .iter()
        .map(|pool| pool.exposure_unrealized_pnl_eth)
        .sum::<f64>();
    let exposure_drawdown_eth = pools
        .iter()
        .filter_map(PoolRollup::exposure_drawdown_eth)
        .sum::<f64>();
    let realized_pnl_eth = pools.iter().map(|pool| pool.realized_pnl_eth).sum::<f64>();
    let unrealized_pnl_eth = pools
        .iter()
        .map(|pool| pool.unrealized_pnl_eth)
        .sum::<f64>();
    let total_pnl_eth = realized_pnl_eth + unrealized_pnl_eth;

    StrategyPerformanceTotals {
        positions: positions.len() as u64,
        open_positions,
        closed_positions: positions.len() as u64 - open_positions,
        buy_orders,
        sell_orders,
        execution_reports: reports.len() as u64,
        confirmed_reports,
        failed_reports,
        confirmed_buys: pools.iter().map(|pool| pool.confirmed_buy_count).sum(),
        confirmed_sells: pools.iter().map(|pool| pool.confirmed_sell_count).sum(),
        risk_events: risks.len() as u64,
        critical_risk_events,
        exposure_positions,
        failed_exit_exposure_positions,
        capital_deployed_eth: positive_or_none(capital_deployed_eth),
        buy_volume_eth: positive_or_none(buy_volume_eth),
        sell_volume_eth: positive_or_none(sell_volume_eth),
        current_value_eth: has_snapshots.then_some(current_value_eth),
        exposure_entry_cost_eth: positive_or_none(exposure_entry_cost_eth),
        exposure_current_value_eth: if exposure_positions > 0 {
            Some(exposure_current_value_eth)
        } else {
            None
        },
        exposure_unrealized_pnl_eth: if exposure_positions > 0 {
            Some(exposure_unrealized_pnl_eth)
        } else {
            None
        },
        exposure_drawdown_eth: if exposure_positions > 0 {
            Some(exposure_drawdown_eth)
        } else {
            None
        },
        exposure_roi_percent: if exposure_entry_cost_eth > 0.0 {
            Some((exposure_unrealized_pnl_eth / exposure_entry_cost_eth) * 100.0)
        } else {
            None
        },
        exposure_to_capital_percent: if capital_deployed_eth > 0.0 {
            Some((exposure_entry_cost_eth / capital_deployed_eth) * 100.0)
        } else {
            None
        },
        realized_pnl_eth: has_pnl.then_some(realized_pnl_eth),
        unrealized_pnl_eth: has_snapshots.then_some(unrealized_pnl_eth),
        total_pnl_eth: has_pnl.then_some(total_pnl_eth),
        roi_percent: if has_pnl && capital_deployed_eth > 0.0 {
            Some((total_pnl_eth / capital_deployed_eth) * 100.0)
        } else {
            None
        },
        pools_with_snapshots,
        pools_with_pnl,
    }
}

pub(super) fn build_timeline(
    positions: &[PositionRecord],
    orders: &[OrderRecord],
    reports: &[ExecutionReportRecord],
    risks: &[RiskRecord],
    report_by_order: &HashMap<String, Vec<ExecutionReportRecord>>,
    bucket_secs: u64,
    bucket_limit: u64,
) -> Vec<StrategyPerformancePoint> {
    let mut buckets: BTreeMap<u64, TimelineBucket> = BTreeMap::new();

    for position in positions {
        bucket_mut(&mut buckets, position.created_epoch, bucket_secs).opened_positions += 1;
        if is_terminal_state(&position.state) {
            bucket_mut(&mut buckets, position.updated_epoch, bucket_secs).closed_positions += 1;
        }
        if let Some(order_id) = position.entry_order_id.as_deref() {
            if let Some(entry_reports) = report_by_order.get(order_id) {
                for report in entry_reports {
                    if report.status == "confirmed" {
                        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
                        bucket.confirmed_buys += 1;
                        bucket.buy_volume_eth += report.filled_amount_eth().unwrap_or(0.0);
                    }
                }
            }
        }
        if let Some(order_id) = position.exit_order_id.as_deref() {
            if let Some(exit_reports) = report_by_order.get(order_id) {
                for report in exit_reports {
                    if report.status == "confirmed" {
                        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
                        bucket.confirmed_sells += 1;
                        bucket.sell_volume_eth += report.filled_amount_eth().unwrap_or(0.0);
                    }
                }
            }
        }
    }

    for order in orders {
        let bucket = bucket_mut(&mut buckets, order.created_epoch, bucket_secs);
        match order.side.as_str() {
            "buy" => {
                bucket.buy_orders += 1;
            }
            "sell" => {
                bucket.sell_orders += 1;
            }
            _ => {}
        }
    }

    for report in reports {
        let bucket = bucket_mut(&mut buckets, report.created_epoch, bucket_secs);
        bucket.execution_reports += 1;
        if report.status == "confirmed" {
            bucket.confirmed_reports += 1;
        } else if is_failed_report_status(&report.status) {
            bucket.failed_reports += 1;
        }
    }

    for risk in risks {
        let bucket = bucket_mut(&mut buckets, risk.created_epoch, bucket_secs);
        bucket.risk_events += 1;
        if risk.severity == "critical" {
            bucket.critical_risk_events += 1;
        }
    }

    let mut points = buckets
        .into_iter()
        .map(
            |(bucket_start_unix_secs, bucket)| StrategyPerformancePoint {
                bucket_start_unix_secs,
                buy_orders: bucket.buy_orders,
                sell_orders: bucket.sell_orders,
                buy_volume_eth: positive_or_none(bucket.buy_volume_eth),
                sell_volume_eth: positive_or_none(bucket.sell_volume_eth),
                execution_reports: bucket.execution_reports,
                confirmed_reports: bucket.confirmed_reports,
                failed_reports: bucket.failed_reports,
                confirmed_buys: bucket.confirmed_buys,
                confirmed_sells: bucket.confirmed_sells,
                opened_positions: bucket.opened_positions,
                closed_positions: bucket.closed_positions,
                risk_events: bucket.risk_events,
                critical_risk_events: bucket.critical_risk_events,
                realized_pnl_eth: None,
                unrealized_pnl_eth: None,
                total_pnl_eth: None,
            },
        )
        .collect::<Vec<_>>();
    if points.len() > bucket_limit as usize {
        points.drain(0..points.len() - bucket_limit as usize);
    }
    points
}

#[derive(Default)]
struct TimelineBucket {
    buy_orders: u64,
    sell_orders: u64,
    buy_volume_eth: f64,
    sell_volume_eth: f64,
    execution_reports: u64,
    confirmed_reports: u64,
    failed_reports: u64,
    confirmed_buys: u64,
    confirmed_sells: u64,
    opened_positions: u64,
    closed_positions: u64,
    risk_events: u64,
    critical_risk_events: u64,
}

fn bucket_mut(
    buckets: &mut BTreeMap<u64, TimelineBucket>,
    epoch: i64,
    bucket_secs: u64,
) -> &mut TimelineBucket {
    let safe_epoch = u64::try_from(epoch).unwrap_or_default();
    let bucket_start = safe_epoch - safe_epoch % bucket_secs;
    buckets.entry(bucket_start).or_default()
}

pub(super) fn reports_by_order(
    reports: &[ExecutionReportRecord],
) -> HashMap<String, Vec<ExecutionReportRecord>> {
    let mut by_order: HashMap<String, Vec<ExecutionReportRecord>> = HashMap::new();
    for report in reports {
        by_order
            .entry(report.order_id.clone())
            .or_default()
            .push(report.clone());
    }
    by_order
}

pub(super) fn confirmed_filled_eth(reports: &[ExecutionReportRecord]) -> f64 {
    reports
        .iter()
        .filter(|report| report.status == "confirmed")
        .filter_map(ExecutionReportRecord::filled_amount_eth)
        .sum()
}

pub(super) fn snapshots_by_position(
    snapshots: Vec<SnapshotRecord>,
) -> HashMap<String, SnapshotRecord> {
    snapshots
        .into_iter()
        .map(|snapshot| (snapshot.position_id.clone(), snapshot))
        .collect()
}

pub(super) fn breakdown<'a>(values: impl Iterator<Item = &'a str>) -> Vec<StrategyBreakdown> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut total = 0_u64;
    for value in values {
        total += 1;
        *counts.entry(value.to_string()).or_default() += 1;
    }
    let mut rows = counts
        .into_iter()
        .map(|(key, count)| StrategyBreakdown {
            key,
            count,
            share_percent: if total > 0 {
                count as f64 / total as f64 * 100.0
            } else {
                0.0
            },
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    rows
}

pub(super) fn compare_pool_rollups(left: &PoolRollup, right: &PoolRollup) -> std::cmp::Ordering {
    right
        .total_pnl_eth()
        .unwrap_or(f64::NEG_INFINITY)
        .partial_cmp(&left.total_pnl_eth().unwrap_or(f64::NEG_INFINITY))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| right.critical_risk_count.cmp(&left.critical_risk_count))
        .then_with(|| right.risk_event_count.cmp(&left.risk_event_count))
        .then_with(|| right.buy_order_count.cmp(&left.buy_order_count))
        .then_with(|| right.updated_epoch.cmp(&left.updated_epoch))
}

pub(super) fn pool_key(token_address: &str, pool_address: &str) -> (String, String) {
    (normalize_key(token_address), normalize_key(pool_address))
}

pub(super) fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn amount_to_f64(raw: &str, decimals: i16) -> Option<f64> {
    let value = raw.parse::<f64>().ok()?;
    let scale = 10_f64.powi(i32::from(decimals).clamp(0, 36));
    if scale.is_finite() && scale > 0.0 {
        Some(value / scale)
    } else {
        None
    }
}

pub(super) fn decimal_text(row: &sqlx::postgres::PgRow, name: &str) -> Result<Option<f64>> {
    let value: String = row.try_get(name).map_err(store_error)?;
    Ok(value.parse::<f64>().ok())
}

pub(super) fn is_terminal_state(state: &str) -> bool {
    matches!(
        state,
        "buy_deferred"
            | "buy_failed"
            | "buy_cancelled"
            | "sell_confirmed"
            | "cancelled"
            | "closed_zero_valuation"
            | "terminal_zero"
            | "scammed"
    )
}

pub(super) fn is_exposure_state(state: &str) -> bool {
    matches!(
        state,
        "buy_confirmed"
            | "sell_intent_created"
            | "sell_submitted"
            | "sell_failed"
            | "sell_cancelled"
    )
}

pub(super) fn is_failed_exit_exposure_state(state: &str) -> bool {
    matches!(state, "sell_failed" | "sell_cancelled")
}

pub(super) fn is_failed_report_status(status: &str) -> bool {
    matches!(status, "failed" | "rejected" | "reverted" | "timeout")
}

pub(super) fn positive_or_none(value: f64) -> Option<f64> {
    if value > 0.0 {
        Some(value)
    } else {
        None
    }
}

pub(super) fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(super) fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_raw_amount_to_decimal_eth() {
        assert_eq!(
            amount_to_f64("1500000000000000000", 18).map(|value| value.round() as i64),
            Some(2)
        );
        assert_eq!(amount_to_f64("2500000", 6), Some(2.5));
    }

    #[test]
    fn terminal_states_match_position_lifecycle() {
        assert!(is_terminal_state("sell_confirmed"));
        assert!(is_terminal_state("buy_deferred"));
        assert!(is_terminal_state("buy_failed"));
        assert!(is_terminal_state("buy_cancelled"));
        assert!(is_terminal_state("cancelled"));
        assert!(is_terminal_state("closed_zero_valuation"));
        assert!(is_terminal_state("terminal_zero"));
        assert!(is_terminal_state("scammed"));
        assert!(!is_terminal_state("buy_confirmed"));
        assert!(!is_terminal_state("sell_failed"));
    }

    #[test]
    fn exposure_states_match_position_lifecycle() {
        assert!(is_exposure_state("buy_confirmed"));
        assert!(is_exposure_state("sell_intent_created"));
        assert!(is_exposure_state("sell_submitted"));
        assert!(is_exposure_state("sell_failed"));
        assert!(is_exposure_state("sell_cancelled"));
        assert!(!is_exposure_state("sell_confirmed"));
        assert!(!is_exposure_state("buy_deferred"));
        assert!(!is_exposure_state("buy_failed"));

        assert!(is_failed_exit_exposure_state("sell_failed"));
        assert!(is_failed_exit_exposure_state("sell_cancelled"));
        assert!(!is_failed_exit_exposure_state("buy_confirmed"));
    }

    #[test]
    fn buckets_events_by_requested_window() {
        let mut buckets = BTreeMap::new();
        let epoch = 1_775_000_123_i64;
        let bucket_start = epoch - epoch % 300;
        bucket_mut(&mut buckets, bucket_start + 12, 300).buy_orders += 1;
        bucket_mut(&mut buckets, bucket_start + 240, 300).buy_orders += 1;
        bucket_mut(&mut buckets, bucket_start + 301, 300).sell_orders += 1;

        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets.values().next().unwrap().buy_orders, 2);
        assert_eq!(buckets.values().last().unwrap().sell_orders, 1);
    }
}
