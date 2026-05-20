use crate::render;

use super::model::{PositionRank, StrategyReport};

pub fn print_strategy_report(report: &StrategyReport) {
    println!("# Strategy Lab: {}", report.run.run_id);
    println!();
    println!(
        "- mode/status: `{}` / `{}`",
        report.run.mode, report.run.status
    );
    println!(
        "- replay run: `{}`",
        report
            .run
            .replay_run_id
            .as_deref()
            .unwrap_or("<none recorded>")
    );
    println!();

    println!("## Summary");
    render::print_table(
        &["metric", "value"],
        &[
            vec![
                "positions".to_string(),
                report.summary.positions.to_string(),
            ],
            vec![
                "open positions".to_string(),
                report.summary.open_positions.to_string(),
            ],
            vec![
                "failed positions".to_string(),
                report.summary.failed_positions.to_string(),
            ],
            vec![
                "buy failed positions".to_string(),
                report.summary.buy_failed_positions.to_string(),
            ],
            vec![
                "sell failed positions".to_string(),
                report.summary.sell_failed_positions.to_string(),
            ],
            vec![
                "entry cost ETH".to_string(),
                report.summary.entry_cost_eth.clone(),
            ],
            vec![
                "gas cost ETH".to_string(),
                report.summary.gas_cost_eth.clone(),
            ],
            vec![
                "latest current value ETH".to_string(),
                report.summary.latest_current_value_eth.clone(),
            ],
            vec![
                "realized PnL ETH".to_string(),
                report.summary.realized_pnl_eth.clone(),
            ],
            vec![
                "unrealized PnL ETH".to_string(),
                report.summary.unrealized_pnl_eth.clone(),
            ],
            vec![
                "total PnL ETH".to_string(),
                report.summary.total_pnl_eth.clone(),
            ],
            vec![
                "ROI on open cost".to_string(),
                report
                    .summary
                    .total_roi_on_open_cost
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "snapshots".to_string(),
                report.summary.snapshots.to_string(),
            ],
            vec![
                "positions with snapshots".to_string(),
                report.summary.snapshot_positions.to_string(),
            ],
            vec![
                "open without snapshot".to_string(),
                report.summary.open_without_snapshot.to_string(),
            ],
            vec![
                "zero decimal / nonzero raw".to_string(),
                report.summary.zero_decimal_nonzero_raw.to_string(),
            ],
        ],
    );
    println!();

    println!("## PnL Concentration");
    render::print_table(
        &["metric", "value"],
        &[
            vec![
                "snapshot positions".to_string(),
                report.concentration.snapshot_positions.to_string(),
            ],
            vec![
                "total PnL ETH".to_string(),
                report.concentration.total_pnl_eth.clone(),
            ],
            vec![
                "excluding top 1".to_string(),
                report.concentration.pnl_ex_top1_eth.clone(),
            ],
            vec![
                "excluding top 2".to_string(),
                report.concentration.pnl_ex_top2_eth.clone(),
            ],
            vec![
                "excluding top 5".to_string(),
                report.concentration.pnl_ex_top5_eth.clone(),
            ],
            vec![
                "excluding top 10".to_string(),
                report.concentration.pnl_ex_top10_eth.clone(),
            ],
        ],
    );
    println!();

    if !report.issue_flags.is_empty() {
        println!("## Flags");
        render::print_table(
            &["severity", "code", "message"],
            &report
                .issue_flags
                .iter()
                .map(|flag| {
                    vec![
                        flag.severity.clone(),
                        flag.code.clone(),
                        flag.message.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    println!("## Protocols");
    render::print_table(
        &["protocol", "denom", "confirmed buys"],
        &report
            .protocols
            .iter()
            .map(|bucket| {
                vec![
                    bucket.protocol.clone(),
                    bucket.denom_symbol.clone(),
                    bucket.confirmed_buys.to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    );
    println!();

    if !report.failures.is_empty() {
        println!("## Failure Buckets");
        render::print_table(
            &["error class", "reports"],
            &report
                .failures
                .iter()
                .map(|bucket| vec![bucket.error_class.clone(), bucket.reports.to_string()])
                .collect::<Vec<_>>(),
        );
        println!();
    }

    if !report.buy_failed_entries.is_empty() {
        println!("## Buy Failed Entries");
        render::print_table(
            &[
                "token", "block", "protocol", "denom", "obs buy", "obs sell", "error",
            ],
            &report
                .buy_failed_entries
                .iter()
                .map(|row| {
                    vec![
                        short(&row.token_address),
                        row.failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.protocol.clone(),
                        row.denom_symbol.clone(),
                        render_bool(row.observed_can_buy),
                        render_bool(row.observed_can_sell),
                        row.error_class.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    if !report.open_failed_exits.is_empty() {
        println!("## Open Failed Exits");
        render::print_table(
            &[
                "token",
                "entry",
                "first fail",
                "last fail",
                "reports",
                "value",
                "pnl",
                "error",
            ],
            &report
                .open_failed_exits
                .iter()
                .map(|row| {
                    vec![
                        short(&row.token_address),
                        row.entry_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.first_failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.last_failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.failed_reports.to_string(),
                        render::fmt_opt(&row.current_value_eth),
                        render::fmt_opt(&row.pnl_eth),
                        row.error_class.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    println!("## Top Winners");
    print_rank_table(&report.top_winners);
    println!();

    println!("## Worst Losers");
    print_rank_table(&report.worst_losers);
}

fn print_rank_table(rows: &[PositionRank]) {
    render::print_table(
        &["token", "entry", "snapshot", "cost", "value", "pnl", "roi"],
        &rows
            .iter()
            .map(|row| {
                vec![
                    short(&row.token_address),
                    row.entry_block
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    row.latest_snapshot_block
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    render::fmt_opt(&row.entry_cost_eth),
                    render::fmt_opt(&row.current_value_eth),
                    render::fmt_opt(&row.pnl_eth),
                    render::fmt_opt(&row.roi),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn short(value: &str) -> String {
    if value.len() <= 14 {
        return value.to_string();
    }
    format!("{}...{}", &value[..8], &value[value.len() - 6..])
}

fn render_bool(value: Option<bool>) -> String {
    match value {
        Some(true) => "yes".to_string(),
        Some(false) => "no".to_string(),
        None => "-".to_string(),
    }
}
