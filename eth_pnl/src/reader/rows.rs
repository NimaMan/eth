use serde_json::{json, Value};
use sqlx::Row;

use crate::Result;

// Row mappers preserve the HTTP payload contract while translating raw SQL rows
// into address, pool-position, breakdown, and movement JSON fragments.
pub(super) fn eth_trader_summary_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "address_count": row.try_get::<i64, _>("address_count")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "scam_address_count": row.try_get::<i64, _>("scam_address_count")?,
        "all_scam_address_count": row.try_get::<i64, _>("all_scam_address_count")?,
        "high_scam_ratio_address_count": row.try_get::<i64, _>("high_scam_ratio_address_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
        "avg_scam_ratio": row.try_get::<Option<f64>, _>("avg_scam_ratio")?,
        "avg_scam_token_ratio": row.try_get::<Option<f64>, _>("avg_scam_token_ratio")?,
        "avg_inflation_score": row.try_get::<Option<f64>, _>("avg_inflation_score")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "movement_rows_available": row.try_get::<bool, _>("movement_rows_available")?,
    }))
}

pub(super) fn eth_trader_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "rank": row.try_get::<i64, _>("rank").ok(),
        "address": row.try_get::<String, _>("address")?,
        "inflation_score": row.try_get::<Option<f64>, _>("inflation_score")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "first_block": row.try_get::<Option<i64>, _>("first_block")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "net_denom_cashflow": row.try_get::<Option<f64>, _>("net_denom_cashflow")?,
        "total_abs_denom_flow": row.try_get::<Option<f64>, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<Option<f64>, _>("scam_abs_denom_flow")?,
        "scam_ratio": row.try_get::<Option<f64>, _>("scam_ratio")?,
        "scam_token_ratio": row.try_get::<Option<f64>, _>("scam_token_ratio")?,
        "scam_mechanisms": row.try_get::<Option<Vec<String>>, _>("scam_mechanisms")?.unwrap_or_default(),
        "scam_labels": row.try_get::<Option<Vec<String>>, _>("scam_labels")?.unwrap_or_default(),
        "lifecycles": row.try_get::<Option<Vec<String>>, _>("lifecycles")?.unwrap_or_default(),
        "pool_labels": row.try_get::<Vec<String>, _>("pool_labels")?,
        "role_flags": row.try_get::<Vec<String>, _>("role_flags")?,
        "token_creator_position_count": row.try_get::<i64, _>("token_creator_position_count")?,
        "pool_creator_position_count": row.try_get::<i64, _>("pool_creator_position_count")?,
        "movement_rows_available": row.try_get::<i64, _>("movement_rows")? > 0,
    }))
}

/// Address-level aggregate PnL band for the profile/activity page. Reads the
/// SUM columns added to `address_agg` (flow through `ranked` via `aa.*`).
pub(super) fn eth_address_aggregate_json(row: &sqlx::postgres::PgRow) -> Result<Value> {
    let all_eth = row
        .try_get::<Option<bool>, _>("all_denom_eth")?
        .unwrap_or(false);
    let realized = row.try_get::<Option<f64>, _>("realized_pnl_denom_sum")?;
    let unrealized = row.try_get::<Option<f64>, _>("unrealized_pnl_denom_sum")?;
    let total = row.try_get::<Option<f64>, _>("total_pnl_denom_sum")?;
    Ok(json!({
        "realized_pnl_denom": realized,
        "unrealized_pnl_denom": unrealized,
        "total_pnl_denom": total,
        "realized_pnl_eth": if all_eth { realized } else { None },
        "unrealized_pnl_eth": if all_eth { unrealized } else { None },
        "total_pnl_eth": if all_eth { total } else { None },
        "denom_is_eth": all_eth,
        "net_denom_cashflow": row.try_get::<Option<f64>, _>("net_denom_cashflow")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "gas_paid_eth": row.try_get::<Option<f64>, _>("gas_paid")?,
        "pool_count": row.try_get::<i64, _>("pool_position_count")?,
        "scam_pool_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "win_count": row.try_get::<i64, _>("win_count")?,
        "loss_count": row.try_get::<i64, _>("loss_count")?,
        "breakeven_count": row.try_get::<i64, _>("breakeven_count")?,
    }))
}

/// Derive a per-address `type` + stackable validity flags from DB-only signals
/// (roles, scam-share, gas, reconciliation). `confidence = "db_only"`; the
/// EOA-vs-contract / known-router refinement is a chain-enriched fast-follow.
/// Scam-pool PnL share at/above which an address's PnL is treated as rug
/// cashflow rather than trader edge (D4 Tier-One admissibility gate).
const SCAM_DOMINANCE_THRESHOLD: f64 = 0.8;

/// Known router / Telegram-bot / protocol / builder addresses whose per-address
/// PnL is aggregated multi-user or infra flow, never a single trader's edge. The
/// top/bottom-100 review found the calc `known_infrastructure()` set does NOT flag
/// these (routers get tagged `user_candidate`), so this read-layer set excludes the
/// prominent offenders now. Lowercased to match stored addresses; the exhaustive
/// fix belongs in the calc via `reth_chain_query::identify_known_address`.
const INFRA_ADDRESSES: &[&str] = &[
    "0x7a250d5630b4cf539739df2c5dacb4c659f2488d", // Uniswap V2 Router02
    "0xe592427a0aece92de3edee1f18e0157c05861564", // Uniswap V3 SwapRouter
    "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45", // Uniswap V3 SwapRouter02
    "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad", // Uniswap UniversalRouter
    "0x000000000004444c5dc75cb358380d2e3de08a90", // Uniswap V4 PoolManager
    "0xc36442b4a4522e871399cd717abdd847ab11fe88", // NonfungiblePositionManager
    "0x35fc556d6f8675b26fdf1542e6e894100155b34e", // Banana Gun: Deployer 2
    "0x3328f7f4a1d1c57c35df56bbf0c9dcafca309c49", // Banana Gun: Router 2
    "0xc465cc50b7d5a29b9308968f870a4b242a8e1873", // Banana Gun
    "0xbcd3a47e4d0000cf170e25d1bd3d53f7c08be0a6", // Banana Gun: Deployer
    "0x80a64c6d7f12c47b7c66c5b4e20e72bc1fcd5d9e", // Maestro: Router 2
    "0x5c9321e92ba4eb43f2901c4952358e132163a85a", // Unibot
    "0xe76014c179f19da26bb30a0f085ff0a466b92829", // Sigma / Alphaman
    "0x1111111254eeb25477b68fb85ed929f73a960582", // 1inch v5 Aggregation Router
    "0x881d40237659c251811cec9c364ef91dc08d300c", // MetaMask Swap Router
    "0x9008d19f58aabd9ed0d60971565aa8510560ab41", // CoW Protocol GPv2Settlement
    "0x4838b106fce9647bdf1e7877bf73ce8b0bad5f97", // Titan Builder (MEV)
];

pub(super) fn eth_address_type_and_flags(
    row: &sqlx::postgres::PgRow,
) -> Result<(Value, Value, Value)> {
    let role_flags = row
        .try_get::<Vec<String>, _>("role_flags")
        .unwrap_or_default();
    let scam_ratio = row.try_get::<Option<f64>, _>("scam_ratio")?.unwrap_or(0.0);
    let pool_count = row.try_get::<i64, _>("pool_position_count")?;
    let total_pnl = row
        .try_get::<Option<f64>, _>("total_pnl_denom_sum")?
        .unwrap_or(0.0);
    let gas_paid = row.try_get::<Option<f64>, _>("gas_paid")?.unwrap_or(0.0);
    let movement_count = row.try_get::<i64, _>("movement_count")?;
    let total_abs = row
        .try_get::<Option<f64>, _>("total_abs_denom_flow")?
        .unwrap_or(0.0);
    let scam_abs = row
        .try_get::<Option<f64>, _>("scam_abs_denom_flow")?
        .unwrap_or(0.0);
    let movement_backed = row.try_get::<i64, _>("movement_backed_position_count")?;
    let token_creator = row.try_get::<i64, _>("token_creator_position_count")?;
    let pool_creator = row.try_get::<i64, _>("pool_creator_position_count")?;
    let all_eth = row
        .try_get::<Option<bool>, _>("all_denom_eth")?
        .unwrap_or(true);
    let address = row
        .try_get::<String, _>("address")
        .map(|value| value.to_lowercase())
        .unwrap_or_default();
    let is_known_infra = INFRA_ADDRESSES.contains(&address.as_str());

    let has = |role: &str| role_flags.iter().any(|value| value == role);
    let is_creator =
        token_creator > 0 || pool_creator > 0 || has("token_creator") || has("pool_creator");

    let address_type = if is_known_infra {
        "infra_known"
    } else if is_creator && scam_ratio >= 0.5 {
        "creator_scammer"
    } else if is_creator {
        "creator"
    } else if has("external_token_source") && has("seller") && !has("buyer") {
        "external_inflow_seller"
    } else if has("custody_victim_candidate") && total_pnl > 0.0 {
        "custody_anomaly"
    } else if scam_ratio >= 0.8 {
        "fresh_launch_sniper"
    } else if has("buyer") && scam_ratio < 0.5 {
        "clean_trader"
    } else {
        "mixed_trader"
    };

    let mut flags: Vec<String> = Vec::new();
    if is_known_infra {
        flags.push("known_infra_address".to_string());
    }
    if total_abs > 0.0 && scam_abs / total_abs >= SCAM_DOMINANCE_THRESHOLD {
        flags.push("pnl_dominated_by_scam_pools".to_string());
    }
    if gas_paid == 0.0 && movement_count > 0 {
        flags.push("gas_not_attributed".to_string());
    }
    if movement_backed < pool_count {
        flags.push("movement_reconciliation_incomplete".to_string());
    }
    if has("custody_victim_candidate") && total_pnl > 0.0 {
        flags.push("custody_victim_with_positive_pnl".to_string());
    }
    // D7: PnL summed across non-WETH-denominated pools (USDC/USDT/DAI, often
    // 6-decimal) is not ETH and silently mis-scales — e.g. 56 USDC booked as
    // "-56 ETH". The top/bottom-100 review found this passing the gate as the
    // single largest fake loss in the run (-39,894 "ETH" = ~39,867 USDC). Any
    // address whose pools are not all WETH-denominated is untrustworthy as an
    // ETH-denominated total and must not be admissible.
    if !all_eth {
        flags.push("non_eth_denom_units".to_string());
    }

    // D4 + D6A: a PnL row is Tier-One-admissible only when NO blocking validity
    // flag fires (scam-dominated cashflow / gas not attributed to the address /
    // movement reconciliation incomplete / custody-victim-with-positive-pnl).
    // These are exactly the signals that make realized/total PnL untrustworthy as
    // trader edge, so the gate is "no blocking flag".
    let admissible = flags.is_empty();
    let admissibility = json!({
        "tier_one_admissible": admissible,
        "pnl_trustworthy": admissible,
        "inadmissible_reasons": flags.clone(),
    });

    Ok((
        json!({ "type": address_type, "confidence": "db_only" }),
        json!(flags),
        admissibility,
    ))
}

pub(super) fn eth_trader_pool_position_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    let pool_labels = row.try_get::<Vec<String>, _>("pool_labels")?;
    let role_flags = row.try_get::<Vec<String>, _>("role_flags")?;
    let actor_role_flags = row
        .try_get::<Vec<String>, _>("actor_role_flags")
        .unwrap_or_default();
    let actor_roles = actor_role_flags
        .iter()
        .map(|role| {
            json!({
                "role": role,
                "source": "pnl_accounting",
            })
        })
        .collect::<Vec<_>>();
    let scam_label = row.try_get::<Option<String>, _>("scam_label")?;
    let scam_mechanism = row.try_get::<Option<String>, _>("scam_mechanism")?;
    let lifecycle = row.try_get::<Option<String>, _>("lifecycle")?;
    let position_status = row.try_get::<String, _>("position_status")?;
    let valuation_status = row.try_get::<String, _>("valuation_status")?;
    let reconciliation_status = row.try_get::<String, _>("reconciliation_status")?;
    let denom_is_eth = row.try_get::<bool, _>("denom_is_eth").unwrap_or(false);
    let realized_pnl_denom = row.try_get::<Option<f64>, _>("realized_pnl_denom")?;
    let unrealized_pnl_denom = row.try_get::<Option<f64>, _>("unrealized_pnl_denom")?;
    let total_pnl_denom = row.try_get::<Option<f64>, _>("total_pnl_denom")?;
    let labels = merged_labels(
        &pool_labels,
        &role_flags,
        [
            scam_label.as_deref(),
            scam_mechanism.as_deref(),
            lifecycle.as_deref(),
            Some(position_status.as_str()),
            Some(valuation_status.as_str()),
            Some(reconciliation_status.as_str()),
        ],
    );

    Ok(json!({
        "pool_id": row.try_get::<String, _>("pool_id")?,
        "token_address": row.try_get::<String, _>("token_address")?,
        "denom_address": row.try_get::<String, _>("denom_address")?,
        "protocol": row.try_get::<Option<String>, _>("protocol")?,
        "is_scam": row.try_get::<bool, _>("is_scam")?,
        "scam_label": scam_label,
        "scam_mechanism": scam_mechanism,
        "lifecycle": lifecycle,
        "token_creator_address": row.try_get::<Option<String>, _>("token_creator_address")?,
        "pool_creator_address": row.try_get::<Option<String>, _>("pool_creator_address")?,
        "pool_labels": pool_labels,
        "role_flags": role_flags,
        "actor_roles": actor_roles,
        "labels": labels,
        "position_status": position_status,
        "valuation_status": valuation_status,
        "reconciliation_status": reconciliation_status,
        "movement_backed_status": reconciliation_status,
        "movement_rows_retained": row.try_get::<i64, _>("movement_rows_retained")?,
        "movement_backed": row.try_get::<bool, _>("movement_rows_backed")?,
        "movement_rows_backed": row.try_get::<bool, _>("movement_rows_backed")?,
        "is_user_candidate": row.try_get::<bool, _>("is_user_candidate")?,
        "accounting_context": row.try_get::<Value, _>("accounting_context")?,
        "first_block": row.try_get::<Option<i64>, _>("first_block")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "denom_cashflow": row.try_get::<f64, _>("denom_cashflow")?,
        "abs_denom_cashflow": row.try_get::<f64, _>("abs_denom_cashflow")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "token_balance": row.try_get::<Option<f64>, _>("token_balance")?,
        "marked_token_value_denom": row.try_get::<Option<f64>, _>("marked_token_value_denom")?,
        "pnl_proxy_denom": row.try_get::<Option<f64>, _>("pnl_proxy_denom")?,
        "realized_pnl_denom": realized_pnl_denom,
        "unrealized_value_denom": row.try_get::<Option<f64>, _>("unrealized_value_denom")?,
        "unrealized_pnl_denom": unrealized_pnl_denom,
        "total_pnl_denom": total_pnl_denom,
        "realized_pnl_eth": if denom_is_eth { realized_pnl_denom } else { None },
        "unrealized_pnl_eth": if denom_is_eth { unrealized_pnl_denom } else { None },
        "total_pnl_eth": if denom_is_eth { total_pnl_denom } else { None },
    }))
}

pub(super) fn eth_trader_mechanism_breakdown_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "mechanism": row.try_get::<String, _>("mechanism")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
    }))
}

pub(super) fn eth_trader_label_breakdown_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "kind": row.try_get::<String, _>("kind")?,
        "label": row.try_get::<String, _>("label")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
    }))
}

pub(super) fn eth_trader_movement_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "pool_id": row.try_get::<String, _>("pool_id")?,
        "token_address": row.try_get::<String, _>("token_address")?,
        "denom_address": row.try_get::<String, _>("denom_address")?,
        "protocol": row.try_get::<Option<String>, _>("protocol")?,
        "is_scam": row.try_get::<bool, _>("is_scam")?,
        "scam_label": row.try_get::<Option<String>, _>("scam_label")?,
        "scam_mechanism": row.try_get::<Option<String>, _>("scam_mechanism")?,
        "entry_index": row.try_get::<i64, _>("entry_index")?,
        "tx_hash": row.try_get::<String, _>("tx_hash")?,
        "block_number": row.try_get::<i64, _>("block_number")?,
        "block_timestamp": row.try_get::<i64, _>("block_timestamp")?,
        "tx_index": row.try_get::<i64, _>("tx_index")?,
        "log_index": row.try_get::<Option<i64>, _>("log_index")?,
        "kind": row.try_get::<String, _>("kind")?,
        "pool_direct": row.try_get::<bool, _>("pool_direct")?,
        "token_in": row.try_get::<Option<f64>, _>("token_in")?,
        "token_out": row.try_get::<Option<f64>, _>("token_out")?,
        "token_delta": row.try_get::<Option<f64>, _>("token_delta")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "denom_delta": row.try_get::<Option<f64>, _>("denom_delta")?,
        "native_fee_eth": row.try_get::<Option<f64>, _>("native_fee_eth")?,
        "native_priority_fee_eth": row.try_get::<Option<f64>, _>("native_priority_fee_eth")?,
    }))
}

fn merged_labels<'a>(
    pool_labels: &[String],
    role_flags: &[String],
    optional_labels: impl IntoIterator<Item = Option<&'a str>>,
) -> Vec<String> {
    let mut labels = pool_labels
        .iter()
        .chain(role_flags.iter())
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    for label in optional_labels.into_iter().flatten() {
        let label = label.trim();
        if !label.is_empty() {
            labels.push(label.to_string());
        }
    }
    labels.sort();
    labels.dedup();
    labels
}
