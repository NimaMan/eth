use crate::render;

use super::model::{LossScanReport, TradeEventTrace};

pub fn print_trade_trace(trace: &TradeEventTrace) {
    println!("# Trade Event Trace: {}", trace.trade.trade_id);
    println!();
    println!("- result set: `{}`", trace.result_set_id);
    println!("- run: `{}`", trace.run_id);
    println!("- strategy: `{}`", trace.strategy_name);
    println!("- token: `{}`", trace.trade.token_address);
    println!("- pool: `{}`", trace.trade.pool_address);
    println!();

    println!("## Trade");
    render::print_table(
        &["field", "value"],
        &[
            vec!["state".to_string(), trace.trade.state.clone()],
            vec![
                "entry block".to_string(),
                fmt_block(trace.trade.entry_block),
            ],
            vec!["exit block".to_string(), fmt_block(trace.trade.exit_block)],
            vec![
                "entry cost ETH".to_string(),
                fmt_opt(&trace.trade.entry_cost_eth),
            ],
            vec![
                "exit value ETH".to_string(),
                fmt_opt(&trace.trade.exit_value_eth),
            ],
            vec!["gas ETH".to_string(), fmt_opt(&trace.trade.gas_cost_eth)],
            vec![
                "realized PnL ETH".to_string(),
                fmt_opt(&trace.trade.realized_pnl_eth),
            ],
            vec![
                "unrealized PnL ETH".to_string(),
                fmt_opt(&trace.trade.unrealized_pnl_eth),
            ],
            vec![
                "total PnL ETH".to_string(),
                fmt_opt(&trace.trade.total_pnl_eth),
            ],
            vec!["ROI".to_string(), fmt_opt(&trace.trade.roi)],
            vec![
                "exit reason".to_string(),
                trace.exit_reason.clone().unwrap_or_else(|| "-".to_string()),
            ],
        ],
    );
    println!();

    println!("## Timing");
    render::print_table(
        &["event", "block"],
        &[
            vec![
                "buy submitted".to_string(),
                fmt_block(trace.timing.buy_submitted_block),
            ],
            vec![
                "buy confirmed".to_string(),
                fmt_block(trace.timing.buy_confirmed_block),
            ],
            vec![
                "first LP approval".to_string(),
                fmt_block(trace.timing.first_lp_approval_block),
            ],
            vec![
                "first liquidity removal".to_string(),
                fmt_block(trace.timing.first_liquidity_removal_block),
            ],
            vec![
                "sell submitted".to_string(),
                fmt_block(trace.timing.sell_submitted_block),
            ],
            vec![
                "sell confirmed".to_string(),
                fmt_block(trace.timing.sell_confirmed_block),
            ],
        ],
    );
    println!();

    println!("## Candidate Signals");
    if trace.signal_candidates.is_empty() {
        println!("No candidate signals.");
    } else {
        let rows = trace
            .signal_candidates
            .iter()
            .map(|signal| {
                vec![
                    signal.severity.clone(),
                    signal.code.clone(),
                    fmt_block(signal.block_number),
                    signal.message.clone(),
                ]
            })
            .collect::<Vec<_>>();
        render::print_table(&["severity", "code", "block", "message"], &rows);
    }
    println!();

    println!("## Event Timeline");
    let rows = trace
        .events
        .iter()
        .map(|event| {
            vec![
                fmt_block(event.block_number),
                event.source.clone(),
                event.kind.clone(),
                event.label.clone(),
                event.detail.clone(),
                event.value_eth.clone().unwrap_or_else(|| "-".to_string()),
                event.pnl_eth.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    render::print_table(
        &["block", "source", "kind", "label", "detail", "value", "pnl"],
        &rows,
    );
}

pub fn print_loss_scan(report: &LossScanReport) {
    println!("# Losing Trade Event Scan");
    println!();
    println!("- result set: `{}`", report.result_set_id);
    println!("- strategy: `{}`", report.strategy_name);
    println!(
        "- run filter: `{}`",
        report.run_id.as_deref().unwrap_or("<none>")
    );
    println!();

    println!("## Summary");
    render::print_table(
        &["metric", "value"],
        &[
            vec![
                "losing trades".to_string(),
                report.losing_trades.to_string(),
            ],
            vec!["total loss ETH".to_string(), report.total_loss_eth.clone()],
            vec![
                "sample traces".to_string(),
                report.sample_traces.len().to_string(),
            ],
        ],
    );
    println!();

    println!("## Signal Buckets");
    if report.signal_buckets.is_empty() {
        println!("No signal buckets.");
    } else {
        let rows = report
            .signal_buckets
            .iter()
            .map(|bucket| {
                vec![
                    bucket.severity.clone(),
                    bucket.code.clone(),
                    bucket.trades.to_string(),
                    bucket.total_pnl_eth.clone(),
                ]
            })
            .collect::<Vec<_>>();
        render::print_table(&["severity", "code", "trades", "pnl ETH"], &rows);
    }
    println!();

    println!("## Exit Reasons");
    if report.exit_reason_buckets.is_empty() {
        println!("No exit reason buckets.");
    } else {
        let rows = report
            .exit_reason_buckets
            .iter()
            .map(|bucket| {
                vec![
                    bucket.reason.clone(),
                    bucket.trades.to_string(),
                    bucket.total_pnl_eth.clone(),
                ]
            })
            .collect::<Vec<_>>();
        render::print_table(&["reason", "trades", "pnl ETH"], &rows);
    }
    println!();

    println!("## Worst Losing Samples");
    if report.sample_traces.is_empty() {
        println!("No losing samples.");
    } else {
        let rows = report
            .sample_traces
            .iter()
            .map(|trace| {
                vec![
                    trace.trade.trade_id.clone(),
                    short_address(&trace.trade.token_address),
                    fmt_block(trace.timing.buy_confirmed_block),
                    fmt_block(trace.timing.first_lp_approval_block),
                    fmt_block(trace.timing.first_liquidity_removal_block),
                    fmt_block(trace.timing.sell_submitted_block),
                    fmt_block(trace.timing.sell_confirmed_block),
                    trace.exit_reason.clone().unwrap_or_else(|| "-".to_string()),
                    fmt_opt(&trace.trade.total_pnl_eth),
                    trace
                        .signal_candidates
                        .iter()
                        .map(|signal| signal.code.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ]
            })
            .collect::<Vec<_>>();
        render::print_table(
            &[
                "trade", "token", "buy C", "LP app", "liq rm", "sell S", "sell C", "exit",
                "pnl ETH", "signals",
            ],
            &rows,
        );
    }
}

fn fmt_opt(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "-".to_string())
}

fn fmt_block(block: Option<i64>) -> String {
    block
        .map(|block| block.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn short_address(address: &str) -> String {
    if address.len() <= 14 {
        return address.to_string();
    }
    format!("{}...{}", &address[..8], &address[address.len() - 6..])
}
