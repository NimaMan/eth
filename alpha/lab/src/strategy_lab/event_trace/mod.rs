pub mod model;
mod query;
mod render;
mod signals;
mod timeline;

use std::collections::{BTreeMap, BTreeSet};

use eyre::Result;
use sqlx::PgPool;

pub use model::{LossScanReport, TradeEventTrace};

use model::{ExitReasonBucket, SignalBucket};

#[derive(Clone, Debug)]
pub struct TraceSelector {
    pub result_set_id: String,
    pub strategy_name: String,
    pub trade_id: String,
    pub run_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LossScanOptions {
    pub result_set_id: String,
    pub strategy_name: String,
    pub run_id: Option<String>,
    pub detail_limit: usize,
}

pub async fn trace_trade(pool: &PgPool, selector: TraceSelector) -> Result<TradeEventTrace> {
    let trade = query::load_trade(
        pool,
        &selector.result_set_id,
        &selector.strategy_name,
        &selector.trade_id,
        selector.run_id.as_deref(),
    )
    .await?;
    let run_id = query::load_trade_run_id(pool, &trade.trade_id).await?;
    build_trade_trace(
        pool,
        &selector.result_set_id,
        &selector.strategy_name,
        &run_id,
        trade,
    )
    .await
}

pub async fn scan_losing_trades(pool: &PgPool, options: LossScanOptions) -> Result<LossScanReport> {
    let trade_ids = query::load_losing_trade_ids(
        pool,
        &options.result_set_id,
        &options.strategy_name,
        options.run_id.as_deref(),
    )
    .await?;

    let mut all_traces = Vec::with_capacity(trade_ids.len());
    for trade_id in trade_ids {
        let trade = query::load_trade(
            pool,
            &options.result_set_id,
            &options.strategy_name,
            &trade_id,
            options.run_id.as_deref(),
        )
        .await?;
        let run_id = query::load_trade_run_id(pool, &trade.trade_id).await?;
        all_traces.push(
            build_trade_trace(
                pool,
                &options.result_set_id,
                &options.strategy_name,
                &run_id,
                trade,
            )
            .await?,
        );
    }

    let losing_trades = all_traces.len() as i64;
    let total_loss = all_traces
        .iter()
        .filter_map(|trace| parse_f64(trace.trade.total_pnl_eth.as_deref()))
        .sum::<f64>();
    let signal_buckets = signal_buckets(&all_traces);
    let exit_reason_buckets = exit_reason_buckets(&all_traces);
    all_traces.sort_by(|left, right| {
        parse_f64(left.trade.total_pnl_eth.as_deref())
            .unwrap_or_default()
            .total_cmp(&parse_f64(right.trade.total_pnl_eth.as_deref()).unwrap_or_default())
    });
    let sample_traces = all_traces
        .into_iter()
        .take(options.detail_limit)
        .collect::<Vec<_>>();

    Ok(LossScanReport {
        result_set_id: options.result_set_id,
        strategy_name: options.strategy_name,
        run_id: options.run_id,
        losing_trades,
        total_loss_eth: format!("{total_loss:.6}"),
        signal_buckets,
        exit_reason_buckets,
        sample_traces,
    })
}

pub fn print_trade_trace(trace: &TradeEventTrace) {
    render::print_trade_trace(trace);
}

pub fn print_loss_scan(report: &LossScanReport) {
    render::print_loss_scan(report);
}

async fn build_trade_trace(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: &str,
    run_id: &str,
    trade: model::TradeRecord,
) -> Result<TradeEventTrace> {
    let trade_events = query::load_trade_events(pool, &trade.trade_id).await?;
    let risk_events =
        query::load_risk_events(pool, run_id, &trade.token_address, &trade.pool_address).await?;
    let decisions = query::load_decisions(
        pool,
        run_id,
        strategy_name,
        &trade.token_address,
        &trade.pool_address,
    )
    .await?;
    let snapshots = query::load_snapshots(pool, &trade.trade_id).await?;

    let timing = query::timing_from_rows(&trade_events, &risk_events);
    let exit_reason = signals::exit_reason(&decisions, &timing);
    let signal_candidates = signals::detect_signal_candidates(
        &trade,
        &timing,
        exit_reason.as_deref(),
        &risk_events,
        &snapshots,
    );
    let events = timeline::build_timeline(&trade_events, &risk_events, &decisions, &snapshots);

    Ok(TradeEventTrace {
        result_set_id: result_set_id.to_string(),
        run_id: run_id.to_string(),
        strategy_name: strategy_name.to_string(),
        trade,
        timing,
        exit_reason,
        signal_candidates,
        events,
    })
}

fn signal_buckets(traces: &[TradeEventTrace]) -> Vec<SignalBucket> {
    let mut buckets: BTreeMap<(String, String), (i64, f64)> = BTreeMap::new();
    for trace in traces {
        let pnl = parse_f64(trace.trade.total_pnl_eth.as_deref()).unwrap_or_default();
        let mut seen_codes = BTreeSet::new();
        for signal in &trace.signal_candidates {
            if !seen_codes.insert(signal.code.clone()) {
                continue;
            }
            let entry = buckets
                .entry((signal.code.clone(), signal.severity.clone()))
                .or_default();
            entry.0 += 1;
            entry.1 += pnl;
        }
    }
    let mut rows = buckets
        .into_iter()
        .map(|((code, severity), (trades, pnl))| SignalBucket {
            code,
            severity,
            trades,
            total_pnl_eth: format!("{pnl:.6}"),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .trades
            .cmp(&left.trades)
            .then_with(|| left.code.cmp(&right.code))
    });
    rows
}

fn exit_reason_buckets(traces: &[TradeEventTrace]) -> Vec<ExitReasonBucket> {
    let mut buckets: BTreeMap<String, (i64, f64)> = BTreeMap::new();
    for trace in traces {
        let pnl = parse_f64(trace.trade.total_pnl_eth.as_deref()).unwrap_or_default();
        let reason = trace
            .exit_reason
            .clone()
            .unwrap_or_else(|| "open/unmapped".to_string());
        let entry = buckets.entry(reason).or_default();
        entry.0 += 1;
        entry.1 += pnl;
    }
    let mut rows = buckets
        .into_iter()
        .map(|(reason, (trades, pnl))| ExitReasonBucket {
            reason,
            trades,
            total_pnl_eth: format!("{pnl:.6}"),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .trades
            .cmp(&left.trades)
            .then_with(|| left.reason.cmp(&right.reason))
    });
    rows
}

fn parse_f64(value: Option<&str>) -> Option<f64> {
    value?.parse::<f64>().ok()
}
