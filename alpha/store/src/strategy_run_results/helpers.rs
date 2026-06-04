use super::*;

pub(super) fn apply_elapsed_time(
    point: &mut ResultSetPerformancePoint,
    start: TimeAxisStart,
    current_epoch_seconds: Option<i64>,
) {
    if let Some((elapsed_seconds, source)) =
        elapsed_time_fields(start, point.block_number, current_epoch_seconds)
    {
        point.elapsed_seconds = Some(elapsed_seconds);
        point.time_axis_source = Some(source);
    }
}

pub(super) fn elapsed_time_fields(
    start: TimeAxisStart,
    block_number: i64,
    current_epoch_seconds: Option<i64>,
) -> Option<(i64, String)> {
    if let (Some(start_epoch), Some(current_epoch)) = (start.epoch_seconds, current_epoch_seconds) {
        return Some((
            current_epoch.saturating_sub(start_epoch),
            "recorded_elapsed".to_string(),
        ));
    }

    let start_block = start.block_number?;
    let elapsed_blocks = block_number.saturating_sub(start_block);
    Some((
        elapsed_blocks * ESTIMATED_ETH_BLOCK_SECONDS,
        "estimated_elapsed_from_blocks".to_string(),
    ))
}

pub(super) fn row_to_result_set(
    row: &sqlx::postgres::PgRow,
    strategy_name: Option<&str>,
) -> Result<ResultSetView> {
    let run_ids = string_vec(row, "run_ids")?;
    let start_block = optional_int(row, "start_block")?;
    let end_block = optional_int(row, "end_block")?;
    let block_count = match (start_block, end_block) {
        (Some(start), Some(end)) if end >= start => Some(end - start + 1),
        _ => None,
    };
    let config = json_value(row, "config")?;
    let strategy_name = strategy_name
        .map(ToOwned::to_owned)
        .or_else(|| config_string(&config, "strategy_name"));
    let strategy_suite = optional_text(row, "strategy_suite")?;
    Ok(ResultSetView {
        result_set_id: text(row, "result_set_id")?,
        run_id: run_ids.first().cloned(),
        run_ids,
        mode: text(row, "mode")?,
        status: text(row, "status")?,
        strategy_suite: strategy_suite.clone(),
        strategy_id: strategy_name.clone().or(strategy_suite.clone()),
        strategy_label: config_string(&config, "strategy_label").or(strategy_name.clone()),
        strategy_name,
        start_block,
        end_block,
        from_block: start_block,
        to_block: end_block,
        block_count,
        metadata: json_value(row, "metadata")?,
        config,
        created_at: text(row, "created_at")?,
        updated_at: text(row, "updated_at")?,
        stopped_at: optional_text(row, "stopped_at")?,
        trade_count: int(row, "trades")?,
        positions: int(row, "trades")?,
        open_trades: int(row, "open_trades")?,
        open_positions: int(row, "open_trades")?,
        closed_trades: int(row, "closed_trades")?,
        closed_positions: int(row, "closed_trades")?,
        failed_trades: int(row, "failed_trades")?,
        failed_positions: int(row, "failed_trades")?,
        exposure_trades: int(row, "exposure_trades")?,
        exposure_positions: int(row, "exposure_trades")?,
        failed_exit_exposure_trades: int(row, "failed_exit_exposure_trades")?,
        failed_exit_exposure_positions: int(row, "failed_exit_exposure_trades")?,
        strategy_count: int(row, "strategies")?,
        buy_volume_eth: optional_float(row, "buy_volume_eth")?,
        sell_volume_eth: optional_float(row, "sell_volume_eth")?,
        current_value_eth: optional_float(row, "current_value_eth")?,
        exposure_entry_cost_eth: optional_float(row, "exposure_entry_cost_eth")?,
        exposure_current_value_eth: optional_float(row, "exposure_current_value_eth")?,
        exposure_unrealized_pnl_eth: optional_float(row, "exposure_unrealized_pnl_eth")?,
        exposure_drawdown_eth: optional_float(row, "exposure_drawdown_eth")?,
        exposure_roi_percent: optional_float(row, "exposure_roi_percent")?,
        exposure_to_capital_percent: optional_float(row, "exposure_to_capital_percent")?,
        realized_pnl_eth: optional_float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: optional_float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: optional_float(row, "total_pnl_eth")?,
        roi_percent: optional_float(row, "roi_percent")?,
    })
}

pub(super) fn row_to_result_set_summary(row: &sqlx::postgres::PgRow) -> Result<ResultSetSummary> {
    Ok(ResultSetSummary {
        positions: int(row, "trades")?,
        open_positions: int(row, "open_trades")?,
        closed_positions: int(row, "closed_trades")?,
        failed_positions: int(row, "failed_trades")?,
        exposure_positions: int(row, "exposure_trades")?,
        failed_exit_exposure_positions: int(row, "failed_exit_exposure_trades")?,
        buy_volume_eth: optional_float(row, "buy_volume_eth")?,
        sell_volume_eth: optional_float(row, "sell_volume_eth")?,
        current_value_eth: optional_float(row, "current_value_eth")?,
        exposure_entry_cost_eth: optional_float(row, "exposure_entry_cost_eth")?,
        exposure_current_value_eth: optional_float(row, "exposure_current_value_eth")?,
        exposure_unrealized_pnl_eth: optional_float(row, "exposure_unrealized_pnl_eth")?,
        exposure_drawdown_eth: optional_float(row, "exposure_drawdown_eth")?,
        exposure_roi_percent: optional_float(row, "exposure_roi_percent")?,
        exposure_to_capital_percent: optional_float(row, "exposure_to_capital_percent")?,
        realized_pnl_eth: optional_float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: optional_float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: optional_float(row, "total_pnl_eth")?,
        roi_percent: optional_float(row, "roi_percent")?,
        reports: int(row, "reports")?,
        failed: int(row, "failed")?,
        risks: int(row, "risks")?,
    })
}

pub(super) fn row_to_carried_snapshot(
    row: &sqlx::postgres::PgRow,
    protocols_by_trade: &HashMap<String, String>,
) -> Result<CarriedSnapshot> {
    let trade_id = text(row, "trade_id")?;
    let protocol = protocols_by_trade
        .get(&trade_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    Ok(CarriedSnapshot {
        trade_id,
        protocol,
        entry_cost_eth: float(row, "entry_cost_eth")?,
        snapshot_block: int(row, "snapshot_block")?,
        state: text(row, "state")?,
        current_value_eth: float(row, "current_value_eth")?,
        realized_pnl_eth: float(row, "realized_pnl_eth")?,
        unrealized_pnl_eth: float(row, "unrealized_pnl_eth")?,
        total_pnl_eth: float(row, "total_pnl_eth")?,
        created_epoch_seconds: optional_int(row, "created_epoch_seconds")?,
        created_at: optional_text(row, "created_at")?,
    })
}

pub(super) fn normalize_protocol_label(protocol: &str) -> String {
    let protocol = protocol.trim();
    if protocol.is_empty() {
        "unknown".to_string()
    } else {
        protocol.to_string()
    }
}

pub(super) fn build_performance_points(
    blocks: &[i64],
    snapshots: &[CarriedSnapshot],
    timeline_start: TimeAxisStart,
) -> Vec<ResultSetPerformancePoint> {
    let mut latest_by_trade: HashMap<String, CarriedSnapshot> = HashMap::new();
    let mut next_snapshot = 0_usize;
    let mut points = Vec::with_capacity(blocks.len());

    for block_number in blocks {
        while let Some(snapshot) = snapshots.get(next_snapshot) {
            if snapshot.snapshot_block > *block_number {
                break;
            }
            latest_by_trade.insert(snapshot.trade_id.clone(), snapshot.clone());
            next_snapshot += 1;
        }

        points.push(build_performance_point(
            *block_number,
            latest_by_trade.values(),
            timeline_start,
            false,
        ));
    }

    points
}

pub(super) fn build_protocol_performance_series(
    blocks: &[i64],
    snapshots: &[CarriedSnapshot],
    timeline_start: TimeAxisStart,
) -> Vec<ResultSetProtocolPerformanceSeries> {
    let protocols = snapshots
        .iter()
        .map(|snapshot| normalize_protocol_label(&snapshot.protocol))
        .collect::<BTreeSet<_>>();
    if protocols.is_empty() {
        return Vec::new();
    }

    let mut latest_by_trade: HashMap<String, CarriedSnapshot> = HashMap::new();
    let mut next_snapshot = 0_usize;
    let mut points_by_protocol = protocols
        .iter()
        .map(|protocol| (protocol.clone(), Vec::with_capacity(blocks.len())))
        .collect::<BTreeMap<_, _>>();

    for block_number in blocks {
        while let Some(snapshot) = snapshots.get(next_snapshot) {
            if snapshot.snapshot_block > *block_number {
                break;
            }
            latest_by_trade.insert(snapshot.trade_id.clone(), snapshot.clone());
            next_snapshot += 1;
        }

        for protocol in &protocols {
            if let Some(points) = points_by_protocol.get_mut(protocol) {
                points.push(build_performance_point(
                    *block_number,
                    latest_by_trade
                        .values()
                        .filter(|snapshot| snapshot.protocol.as_str() == protocol.as_str()),
                    timeline_start,
                    true,
                ));
            }
        }
    }

    points_by_protocol
        .into_iter()
        .map(|(protocol, points)| ResultSetProtocolPerformanceSeries { protocol, points })
        .collect()
}

pub(super) fn insert_protocol_baseline_points(
    protocol_series: &mut [ResultSetProtocolPerformanceSeries],
    baseline: &ResultSetPerformancePoint,
) {
    for series in protocol_series {
        let should_insert = series.points.first().map_or(true, |point| {
            point.block_number > baseline.block_number
                || (point.block_number == baseline.block_number
                    && point.total_pnl_eth.unwrap_or(0.0).abs() > f64::EPSILON)
        });
        if should_insert {
            series.points.insert(0, zero_performance_point(baseline));
        }
    }
}

pub(super) fn build_protocol_performance_summary(
    protocol_series: &[ResultSetProtocolPerformanceSeries],
) -> ResultSetProtocolPerformanceSummary {
    let mut latest = protocol_series
        .iter()
        .filter_map(|series| {
            let value = series.points.last()?.total_pnl_eth?;
            value.is_finite().then(|| (series.protocol.clone(), value))
        })
        .collect::<Vec<_>>();

    if latest.is_empty() {
        return ResultSetProtocolPerformanceSummary {
            protocol_count: protocol_series.len(),
            ..Default::default()
        };
    }

    let latest_total_pnl_eth = latest.iter().map(|(_, value)| *value).sum();
    latest.sort_by(|left, right| right.1.total_cmp(&left.1));
    let (best_protocol, best_total_pnl_eth) = latest
        .first()
        .map(|(protocol, value)| (Some(protocol.clone()), Some(*value)))
        .unwrap_or((None, None));
    let (worst_protocol, worst_total_pnl_eth) = latest
        .last()
        .map(|(protocol, value)| (Some(protocol.clone()), Some(*value)))
        .unwrap_or((None, None));

    ResultSetProtocolPerformanceSummary {
        protocol_count: protocol_series.len(),
        latest_total_pnl_eth: Some(latest_total_pnl_eth),
        best_protocol,
        best_total_pnl_eth,
        worst_protocol,
        worst_total_pnl_eth,
    }
}

pub(super) fn build_performance_point<'a>(
    block_number: i64,
    snapshots: impl IntoIterator<Item = &'a CarriedSnapshot>,
    timeline_start: TimeAxisStart,
    force_zero_values: bool,
) -> ResultSetPerformancePoint {
    let mut trade_count = 0_i64;
    let mut open_trades = 0_i64;
    let mut closed_trades = 0_i64;
    let mut exposure_trades = 0_i64;
    let mut failed_exit_exposure_trades = 0_i64;
    let mut entry_cost_eth = 0.0_f64;
    let mut current_value_eth = 0.0_f64;
    let mut exposure_entry_cost_eth = 0.0_f64;
    let mut exposure_current_value_eth = 0.0_f64;
    let mut exposure_unrealized_pnl_eth = 0.0_f64;
    let mut exposure_drawdown_eth = 0.0_f64;
    let mut realized_pnl_eth = 0.0_f64;
    let mut unrealized_pnl_eth = 0.0_f64;
    let mut total_pnl_eth = 0.0_f64;
    let mut recorded_at: Option<String> = None;
    let mut recorded_epoch_seconds: Option<i64> = None;

    for snapshot in snapshots {
        trade_count += 1;
        if is_closed_trade_state(&snapshot.state) {
            closed_trades += 1;
        } else if is_open_trade_state(&snapshot.state) {
            open_trades += 1;
        }
        if is_exposure_trade_state(&snapshot.state) {
            exposure_trades += 1;
            exposure_entry_cost_eth += snapshot.entry_cost_eth;
            exposure_current_value_eth += snapshot.current_value_eth;
            exposure_unrealized_pnl_eth += snapshot.unrealized_pnl_eth;
            exposure_drawdown_eth +=
                (snapshot.entry_cost_eth - snapshot.current_value_eth).max(0.0);
        }
        if is_failed_exit_exposure_trade_state(&snapshot.state) {
            failed_exit_exposure_trades += 1;
        }
        entry_cost_eth += snapshot.entry_cost_eth;
        current_value_eth += snapshot.current_value_eth;
        realized_pnl_eth += snapshot.realized_pnl_eth;
        unrealized_pnl_eth += snapshot.unrealized_pnl_eth;
        total_pnl_eth += snapshot.total_pnl_eth;
        if let Some(created_epoch) = snapshot.created_epoch_seconds {
            if recorded_epoch_seconds
                .map(|current| created_epoch > current)
                .unwrap_or(true)
            {
                recorded_epoch_seconds = Some(created_epoch);
                recorded_at = snapshot.created_at.clone();
            }
        } else if let Some(created_at) = snapshot.created_at.as_deref() {
            if recorded_at
                .as_deref()
                .map(|current| created_at > current)
                .unwrap_or(true)
            {
                recorded_at = Some(created_at.to_owned());
            }
        }
    }

    let portfolio_value_eth = entry_cost_eth + total_pnl_eth;
    let roi_percent = if entry_cost_eth > 0.0 {
        Some(total_pnl_eth / entry_cost_eth * 100.0)
    } else if force_zero_values {
        Some(0.0)
    } else {
        None
    };
    let exposure_roi_percent = if exposure_entry_cost_eth > 0.0 {
        Some(exposure_unrealized_pnl_eth / exposure_entry_cost_eth * 100.0)
    } else if force_zero_values {
        Some(0.0)
    } else {
        None
    };
    let exposure_to_entry_percent = if entry_cost_eth > 0.0 {
        Some(exposure_entry_cost_eth / entry_cost_eth * 100.0)
    } else if force_zero_values {
        Some(0.0)
    } else {
        None
    };
    let (elapsed_seconds, time_axis_source) =
        elapsed_time_fields(timeline_start, block_number, recorded_epoch_seconds)
            .map(|(elapsed_seconds, source)| (Some(elapsed_seconds), Some(source)))
            .unwrap_or((None, None));
    ResultSetPerformancePoint {
        block_number,
        elapsed_seconds,
        time_axis_source,
        trade_count,
        open_trades,
        closed_trades,
        exposure_trades,
        failed_exit_exposure_trades,
        entry_cost_eth: some_if_any_or_forced(trade_count, entry_cost_eth, force_zero_values),
        current_value_eth: some_if_any_or_forced(trade_count, current_value_eth, force_zero_values),
        portfolio_value_eth: some_if_any_or_forced(
            trade_count,
            portfolio_value_eth,
            force_zero_values,
        ),
        exposure_entry_cost_eth: some_if_any_or_forced(
            exposure_trades,
            exposure_entry_cost_eth,
            force_zero_values,
        ),
        exposure_current_value_eth: some_if_any_or_forced(
            exposure_trades,
            exposure_current_value_eth,
            force_zero_values,
        ),
        exposure_unrealized_pnl_eth: some_if_any_or_forced(
            exposure_trades,
            exposure_unrealized_pnl_eth,
            force_zero_values,
        ),
        exposure_drawdown_eth: some_if_any_or_forced(
            exposure_trades,
            exposure_drawdown_eth,
            force_zero_values,
        ),
        exposure_roi_percent,
        exposure_to_entry_percent,
        realized_pnl_eth: some_if_any_or_forced(trade_count, realized_pnl_eth, force_zero_values),
        unrealized_pnl_eth: some_if_any_or_forced(
            trade_count,
            unrealized_pnl_eth,
            force_zero_values,
        ),
        total_pnl_eth: some_if_any_or_forced(trade_count, total_pnl_eth, force_zero_values),
        roi_percent,
        recorded_at,
    }
}

pub(super) fn zero_performance_point(
    baseline: &ResultSetPerformancePoint,
) -> ResultSetPerformancePoint {
    ResultSetPerformancePoint {
        block_number: baseline.block_number,
        elapsed_seconds: baseline.elapsed_seconds,
        time_axis_source: baseline.time_axis_source.clone(),
        trade_count: 0,
        open_trades: 0,
        closed_trades: 0,
        exposure_trades: 0,
        failed_exit_exposure_trades: 0,
        entry_cost_eth: Some(0.0),
        current_value_eth: Some(0.0),
        portfolio_value_eth: Some(0.0),
        exposure_entry_cost_eth: Some(0.0),
        exposure_current_value_eth: Some(0.0),
        exposure_unrealized_pnl_eth: Some(0.0),
        exposure_drawdown_eth: Some(0.0),
        exposure_roi_percent: Some(0.0),
        exposure_to_entry_percent: Some(0.0),
        realized_pnl_eth: Some(0.0),
        unrealized_pnl_eth: Some(0.0),
        total_pnl_eth: Some(0.0),
        roi_percent: Some(0.0),
        recorded_at: baseline.recorded_at.clone(),
    }
}

pub(super) fn some_if_any_or_forced(count: i64, value: f64, force: bool) -> Option<f64> {
    if force {
        Some(value)
    } else {
        some_if_any(count, value)
    }
}

pub(super) fn some_if_any(count: i64, value: f64) -> Option<f64> {
    if count > 0 {
        Some(value)
    } else {
        None
    }
}

pub(super) fn is_open_trade_state(state: &str) -> bool {
    !matches!(
        state.to_ascii_lowercase().as_str(),
        "sell_confirmed"
            | "buy_deferred"
            | "buy_failed"
            | "buy_cancelled"
            | "cancelled"
            | "terminal_zero"
            | "scammed"
            | "failed"
    )
}

pub(super) fn is_exposure_trade_state(state: &str) -> bool {
    matches!(
        state.to_ascii_lowercase().as_str(),
        "buy_confirmed"
            | "sell_intent_created"
            | "sell_submitted"
            | "sell_failed"
            | "sell_cancelled"
    )
}

pub(super) fn is_failed_exit_exposure_trade_state(state: &str) -> bool {
    matches!(
        state.to_ascii_lowercase().as_str(),
        "sell_failed" | "sell_cancelled"
    )
}

pub(super) fn is_closed_trade_state(state: &str) -> bool {
    state.eq_ignore_ascii_case("sell_confirmed")
}

pub(super) fn config_string(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(super) fn json_value(row: &sqlx::postgres::PgRow, column: &str) -> Result<Value> {
    serde_json::from_str(&text(row, column)?)
        .map_err(|error| AlphaCoreError::Store(format!("failed to parse {column} json: {error}")))
}

pub(super) fn text(row: &sqlx::postgres::PgRow, column: &str) -> Result<String> {
    row.try_get::<String, _>(column).map_err(store_error)
}

pub(super) fn optional_text(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<String>> {
    row.try_get::<Option<String>, _>(column)
        .map_err(store_error)
}

pub(super) fn int(row: &sqlx::postgres::PgRow, column: &str) -> Result<i64> {
    row.try_get::<i64, _>(column).map_err(store_error)
}

pub(super) fn optional_int(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<i64>> {
    row.try_get::<Option<i64>, _>(column).map_err(store_error)
}

pub(super) fn optional_float(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<f64>> {
    row.try_get::<Option<f64>, _>(column).map_err(store_error)
}

pub(super) fn bool_value(row: &sqlx::postgres::PgRow, column: &str) -> Result<bool> {
    row.try_get::<bool, _>(column).map_err(store_error)
}

pub(super) fn float(row: &sqlx::postgres::PgRow, column: &str) -> Result<f64> {
    row.try_get::<f64, _>(column).map_err(store_error)
}

pub(super) fn string_vec(row: &sqlx::postgres::PgRow, column: &str) -> Result<Vec<String>> {
    row.try_get::<Vec<String>, _>(column).map_err(store_error)
}

pub(super) fn store_error(error: impl std::fmt::Display) -> AlphaCoreError {
    AlphaCoreError::Store(error.to_string())
}
