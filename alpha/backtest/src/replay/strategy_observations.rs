use eth_alpha_core::risk::RISK_SOURCE_MEMPOOL_SIGNAL;
use eth_alpha_engine::wire::{MempoolSignalWire, PoolWire};
use eth_alpha_store::observations::{query_strategy_observations, StrategyObservation};
use eyre::{Result, WrapErr};
use serde_json::Value;

use super::blocks::add_block_completed_events;

/// Load historical events by replaying `strategy_observations` from an existing
/// live chain-sim run.
pub async fn load_events_from_observations(
    pool: &sqlx::PgPool,
    replay_run_id: &str,
    skip_primed: bool,
    from_block: Option<u64>,
    to_block: Option<u64>,
    include_signal_risk_events: bool,
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let rows = query_strategy_observations(pool, replay_run_id, from_block, to_block)
        .await
        .wrap_err("failed to query strategy_observations")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;
    let mut skipped_mempool = 0usize;
    let mut skipped_position_monitor = 0usize;
    let mut included_signal_risk_events = 0usize;

    for row in &rows {
        let payload = &row.payload;
        if skip_primed && payload.get("suppress_events").and_then(|v| v.as_bool()) == Some(true) {
            skipped += 1;
            continue;
        }

        match row.event_source.as_str() {
            "pool_update" => {
                if payload.get("pool").is_none() {
                    skipped_position_monitor += 1;
                    continue;
                }
                let mut pool_wire: PoolWire = match serde_json::from_value(
                    payload.get("pool").cloned().unwrap_or(Value::Null),
                ) {
                    Ok(w) => w,
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping malformed pool observation");
                        skipped += 1;
                        continue;
                    }
                };
                // Historical observations recorded before May 2026 may be missing
                // denom_symbol / denom_address. Default to WETH for backtest
                // validation so classification does not reject every pool.
                if pool_wire.denom_symbol.is_none() && pool_wire.currency.is_none() {
                    pool_wire.denom_symbol = Some("WETH".to_string());
                }
                let pool = match pool_wire.to_pool_snapshot() {
                    Ok(p) => p,
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping pool with missing fields");
                        skipped += 1;
                        continue;
                    }
                };
                events.push(eth_alpha_engine::EngineEvent::Market(
                    eth_alpha_core::market::MarketEvent::PoolUpdated {
                        block_number: pool.latest_block,
                        pool,
                    },
                ));
            }
            "mempool_signal" => {
                if include_signal_risk_events {
                    let Some(observed_block) = observation_replay_block(row) else {
                        skipped += 1;
                        continue;
                    };
                    let signal: MempoolSignalWire = match serde_json::from_value(
                        payload.get("signal").cloned().unwrap_or(Value::Null),
                    ) {
                        Ok(signal) => signal,
                        Err(error) => {
                            tracing::warn!(error = %error, "skipping malformed signal observation");
                            skipped += 1;
                            continue;
                        }
                    };
                    if let Some(event) = historical_signal_risk_event(signal, observed_block)? {
                        events.push(eth_alpha_engine::EngineEvent::Risk(event));
                        included_signal_risk_events += 1;
                    } else {
                        skipped_mempool += 1;
                    }
                    continue;
                }
                skipped_mempool += 1;
                continue;
            }
            "position_monitor" => {
                skipped_position_monitor += 1;
                continue;
            }
            other => {
                tracing::warn!(event_source = %other, "skipping unknown observation source");
                skipped += 1;
            }
        }
    }

    if skipped > 0 || skipped_mempool > 0 {
        tracing::info!(
            skipped_malformed = skipped,
            skipped_mempool,
            skipped_position_monitor,
            included_signal_risk_events,
            total = rows.len(),
            "skipped historical observations"
        );
    }

    Ok(add_block_completed_events(events, from_block, to_block))
}

fn observation_replay_block(row: &StrategyObservation) -> Option<u64> {
    row.replay_block.and_then(|block| u64::try_from(block).ok())
}

fn historical_signal_risk_event(
    signal: MempoolSignalWire,
    observed_block: u64,
) -> Result<Option<eth_alpha_core::risk::RiskEvent>> {
    if !matches!(
        signal.signal_type.as_str(),
        "trading_enabled" | "lp_approval" | "lp_position_approval" | "liquidity_removal"
    ) {
        return Ok(None);
    }

    let Some(mut event) = signal.to_risk_event()? else {
        return Ok(None);
    };
    event.observed_block = Some(observed_block);
    event.source = Some(RISK_SOURCE_MEMPOOL_SIGNAL.to_string());
    event.message = format!("replayed mempool signal: {}", event.message);
    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use eth_alpha_core::{
        mempool_entry::MEMPOOL_ENTRY_EVIDENCE_KEY,
        risk::{RiskKind, RISK_SOURCE_MEMPOOL_SIGNAL},
    };
    use eth_alpha_engine::wire::MempoolSignalWire;
    use serde_json::{json, Value};

    use super::historical_signal_risk_event;

    #[test]
    fn historical_replay_includes_trading_enabled_entry_evidence() {
        let entry_evidence = json!({
            "evidence_version": "mempool_entry_evidence_v1",
            "base_block": 12,
            "projected_pool": {
                "protocol": "UNISWAP-V2",
                "denom_reserve": "1",
                "token_reserve": "100",
                "latest_block": 12,
                "can_buy": true,
                "can_sell": true,
                "is_scam": false
            },
            "viability": {
                "can_buy": true,
                "can_approve": true,
                "can_sell": true
            },
            "vault_buy_simulation": {
                "route": "pool_buy_sell_probe",
                "would_revert": false,
                "gas_used": 176000,
                "eth_spent_wei": "10000000000000000",
                "tokens_received_raw": "1000000"
            }
        });
        let signal = MempoolSignalWire {
            signal_id: "1".to_string(),
            signal_type: "trading_enabled".to_string(),
            signal_source: None,
            signal_created_at: None,
            mempool_first_seen_at: None,
            mempool_first_seen_ms: None,
            detection_timestamp: None,
            detection_tx_hash: None,
            token_address: Some("0x1111111111111111111111111111111111111111".to_string()),
            pool_address: Some("0x2222222222222222222222222222222222222222".to_string()),
            pool_type: None,
            creator_address: None,
            subject_address: None,
            headline: None,
            value_1: None,
            value_2: None,
            flag: None,
            payload: Value::Null,
            mempool_entry_evidence: Some(entry_evidence.clone()),
        };

        let event = historical_signal_risk_event(signal, 12)
            .expect("risk conversion")
            .expect("trading enabled risk event");

        assert_eq!(event.kind, RiskKind::TradingEnabled);
        assert_eq!(event.source.as_deref(), Some(RISK_SOURCE_MEMPOOL_SIGNAL));
        assert_eq!(event.observed_block, Some(12));
        assert_eq!(
            event.evidence.unwrap()[MEMPOOL_ENTRY_EVIDENCE_KEY],
            entry_evidence
        );
    }
}
