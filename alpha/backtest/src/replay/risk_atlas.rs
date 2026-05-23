use alloy_primitives::Address;
use eth_alpha_core::ids::{PoolAddress, TokenPoolId};
use eth_alpha_core::market::PoolSnapshot;
use eth_alpha_core::risk::{RiskEvent, RiskKind, RiskSeverity, RISK_SOURCE_RISK_ATLAS_MINED_CHAIN};
use eth_alpha_engine::wire::{decimal_from_f64, parse_address, parse_protocol};
use eth_alpha_store::observations::{query_risk_atlas_observations, RiskAtlasObservation};
use eyre::{Result, WrapErr};

use super::blocks::add_block_completed_events;

/// Load historical events from the token-lab Risk Atlas observation export.
///
/// This is the mined-chain replay path for token analytics features. It emits
/// pool update events from `risk_atlas_observations`, plus LP approval and
/// direct LP removal risk events derived from the observation event flags.
pub async fn load_events_from_risk_atlas(
    pool: &sqlx::PgPool,
    risk_atlas_run_id: &str,
    from_block: Option<u64>,
    to_block: Option<u64>,
    allowed_protocols: &[String],
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let rows = query_risk_atlas_observations(
        pool,
        risk_atlas_run_id,
        from_block,
        to_block,
        allowed_protocols,
    )
    .await
    .wrap_err("failed to query risk_atlas_observations")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;
    let mut lp_approval_risks = 0usize;
    let mut direct_lp_removal_risks = 0usize;

    for row in rows {
        let block = u64::try_from(row.block_number).unwrap_or_default();
        let token_address = match parse_address(&row.token_address) {
            Ok(address) => address,
            Err(error) => {
                tracing::warn!(token_address = %row.token_address, error = %error, "skipping Risk Atlas row with invalid token address");
                skipped += 1;
                continue;
            }
        };
        let pool_address = TokenPoolId::new(token_address, &row.pool_address);

        let pool_snapshot = match risk_atlas_pool_snapshot(
            &row,
            token_address,
            pool_address.clone(),
            block,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!(error = %error, "skipping malformed Risk Atlas pool observation");
                skipped += 1;
                continue;
            }
        };

        if row.lp_approval_count_in_block > 0 {
            events.push(eth_alpha_engine::EngineEvent::Risk(RiskEvent {
                kind: RiskKind::LpApproval,
                severity: RiskSeverity::Warning,
                source: Some(RISK_SOURCE_RISK_ATLAS_MINED_CHAIN.to_string()),
                token_address,
                pool_address: Some(pool_address.clone()),
                pending_tx_hash: None,
                observed_block: Some(block),
                message: risk_atlas_lp_approval_message(&row),
            }));
            lp_approval_risks += 1;
        }

        events.push(eth_alpha_engine::EngineEvent::Market(
            eth_alpha_core::market::MarketEvent::PoolUpdated {
                block_number: block,
                pool: pool_snapshot,
            },
        ));

        if row.direct_lp_removal_in_block {
            events.push(eth_alpha_engine::EngineEvent::Risk(RiskEvent {
                kind: RiskKind::LiquidityRemoval,
                severity: RiskSeverity::Critical,
                source: Some(RISK_SOURCE_RISK_ATLAS_MINED_CHAIN.to_string()),
                token_address,
                pool_address: Some(pool_address.clone()),
                pending_tx_hash: None,
                observed_block: Some(block),
                message: "risk atlas mined-chain direct LP liquidity removal".to_string(),
            }));
            direct_lp_removal_risks += 1;
        }
    }

    tracing::info!(
        risk_atlas_run_id,
        allowed_protocols = ?allowed_protocols,
        events = events.len(),
        skipped,
        lp_approval_risks,
        direct_lp_removal_risks,
        "loaded Risk Atlas historical events"
    );

    Ok(add_block_completed_events(events, from_block, to_block))
}

fn risk_atlas_pool_snapshot(
    row: &RiskAtlasObservation,
    token_address: Address,
    pool_address: PoolAddress,
    block: u64,
) -> Result<PoolSnapshot> {
    let denom_address = parse_address(&row.denom_address)?;
    let denom_reserve = row.denom_reserve.unwrap_or(0.0);
    let token_reserve = row.token_reserve.unwrap_or(0.0);
    let can_buy = row.effective_can_buy.unwrap_or(row.can_buy);
    let can_sell = row.effective_can_sell.unwrap_or(row.can_sell);
    let token_decimals = row
        .token_decimals
        .and_then(|decimals| u8::try_from(decimals).ok());

    Ok(PoolSnapshot {
        address: pool_address,
        token_address,
        protocol: parse_protocol(&row.protocol),
        denom_address: Some(denom_address),
        denom_symbol: row.quote_symbol.clone(),
        denom_reserve: decimal_from_f64(denom_reserve),
        token_reserve: decimal_from_f64(token_reserve),
        price_denom_per_token: row.price_denom_per_token.map(decimal_from_f64),
        initial_price_denom_per_token: row.initial_price_denom_per_token.map(decimal_from_f64),
        price_ratio_to_initial: row.price_ratio_to_initial.map(decimal_from_f64),
        creation_block: row
            .creation_block
            .and_then(|block| u64::try_from(block).ok()),
        token_decimals,
        fee_tier: None,
        uniswap_v4: None,
        latest_block: block,
        can_buy,
        can_sell,
        is_scam: false,
    })
}

fn risk_atlas_lp_approval_message(row: &RiskAtlasObservation) -> String {
    let owner_is_creator = row.lp_approval_owner_is_creator.unwrap_or(false);
    let approval_pct = row
        .lp_total_supply
        .zip(row.lp_max_approval_amount_as_of)
        .and_then(|(supply, approval)| {
            if supply > 0.0 {
                Some((approval / supply * 100.0).min(100.0))
            } else {
                None
            }
        });
    let approval_age = row
        .trading_enabled_to_last_lp_approval_chain_block_delta
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let approval_freshness = row
        .last_lp_approval_to_as_of_chain_block_delta
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "risk atlas mined-chain LP approval: count={}, generic_approved_pct={}, owner_is_creator={owner_is_creator}, trading_enabled_to_last_lp_approval_chain_block_delta={approval_age}, last_lp_approval_to_as_of_chain_block_delta={approval_freshness}",
        row.lp_approval_count_in_block,
        approval_pct
            .map(|pct| format!("{pct:.2}%"))
            .unwrap_or_else(|| "unknown".to_string())
    )
}
