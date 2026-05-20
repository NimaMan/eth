use crate::render;

use super::model::{PoolObservation, PositionReport};

pub fn print_position_report(report: &PositionReport) {
    println!("# Token Lab: {}", report.position.token_address);
    println!();
    println!("- run: `{}`", report.position.run_id);
    println!(
        "- replay run: `{}`",
        report
            .run
            .replay_run_id
            .as_deref()
            .unwrap_or("<none recorded>")
    );
    println!("- pool: `{}`", report.position.pool_address);
    println!();

    println!("## Position");
    render::print_table(
        &["field", "value"],
        &[
            vec!["state".to_string(), report.position.state.clone()],
            vec![
                "entry order".to_string(),
                report
                    .position
                    .entry_order_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "entry block".to_string(),
                report
                    .position
                    .entry_block
                    .map(|block| block.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "entry cost ETH".to_string(),
                render::fmt_opt(&report.position.entry_cost_eth),
            ],
            vec![
                "entry token amount".to_string(),
                render::fmt_opt(&report.position.entry_token_amount),
            ],
            vec![
                "entry token raw".to_string(),
                render::fmt_opt(&report.position.entry_token_raw),
            ],
        ],
    );
    println!();

    if let Some(report_row) = &report.entry_report {
        println!("## Entry Report");
        render::print_table(
            &["field", "value"],
            &[
                vec!["status".to_string(), report_row.status.clone()],
                vec![
                    "block".to_string(),
                    report_row
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "gas".to_string(),
                    report_row
                        .gas_used
                        .map(|gas| gas.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "filled raw".to_string(),
                    render::fmt_opt(&report_row.filled_amount_raw),
                ],
                vec![
                    "token raw".to_string(),
                    render::fmt_opt(&report_row.token_amount_raw),
                ],
                vec!["error".to_string(), render::fmt_opt(&report_row.error)],
            ],
        );
        println!();
    }

    println!("## Observations");
    render::print_table(
        &[
            "point",
            "block",
            "protocol",
            "denom",
            "denom reserve",
            "token reserve",
            "price",
            "can buy",
            "can sell",
        ],
        &[
            observation_row("entry", report.entry_observation.as_ref()),
            observation_row("latest", report.latest_observation.as_ref()),
        ],
    );
    println!();

    println!("## Latest Snapshot");
    if let Some(snapshot) = &report.latest_snapshot {
        render::print_table(
            &["field", "value"],
            &[
                vec![
                    "block".to_string(),
                    snapshot
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "current value ETH".to_string(),
                    render::fmt_opt(&snapshot.current_value_eth),
                ],
                vec![
                    "unrealized PnL ETH".to_string(),
                    render::fmt_opt(&snapshot.unrealized_profit_eth),
                ],
                vec!["ROI".to_string(), render::fmt_opt(&snapshot.roi)],
            ],
        );
    } else {
        println!("No snapshots for this position.");
    }
    println!();

    println!("## Checks");
    render::print_table(
        &["status", "code", "message"],
        &report
            .checks
            .iter()
            .map(|check| {
                vec![
                    check.status.clone(),
                    check.code.clone(),
                    check.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    );
    println!();

    println!("## Trajectory");
    render::print_table(
        &[
            "block",
            "value ETH",
            "PnL ETH",
            "ROI",
            "denom reserve",
            "token reserve",
            "spot",
            "buy",
            "sell",
        ],
        &report
            .trajectory
            .iter()
            .map(|point| {
                vec![
                    point
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    render::fmt_opt(&point.current_value_eth),
                    render::fmt_opt(&point.unrealized_profit_eth),
                    render::fmt_opt(&point.roi),
                    render::fmt_opt(&point.denom_reserve),
                    render::fmt_opt(&point.token_reserve),
                    render::fmt_opt(&point.spot_price),
                    point
                        .can_buy
                        .map(render::fmt_bool)
                        .unwrap_or("-")
                        .to_string(),
                    point
                        .can_sell
                        .map(render::fmt_bool)
                        .unwrap_or("-")
                        .to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn observation_row(label: &str, observation: Option<&PoolObservation>) -> Vec<String> {
    if let Some(obs) = observation {
        vec![
            label.to_string(),
            obs.block_number
                .map(|block| block.to_string())
                .unwrap_or_else(|| "-".to_string()),
            render::fmt_opt(&obs.protocol),
            render::fmt_opt(&obs.denom_symbol),
            render::fmt_opt(&obs.denom_reserve),
            render::fmt_opt(&obs.token_reserve),
            render::fmt_opt(&obs.price),
            obs.can_buy.map(render::fmt_bool).unwrap_or("-").to_string(),
            obs.can_sell
                .map(render::fmt_bool)
                .unwrap_or("-")
                .to_string(),
        ]
    } else {
        vec![
            label.to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ]
    }
}
