use std::collections::BTreeMap;

use eth_alpha_core::market::MarketEvent;
use eth_alpha_engine::EngineEvent;

pub fn add_block_completed_events(
    events: Vec<EngineEvent>,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Vec<EngineEvent> {
    let mut by_block: BTreeMap<u64, Vec<EngineEvent>> = BTreeMap::new();
    let mut without_block = Vec::new();

    for event in events {
        if let Some(block) = event_block(&event) {
            by_block.entry(block).or_default().push(event);
        } else {
            without_block.push(event);
        }
    }

    let Some(first_event_block) = by_block.keys().next().copied() else {
        return without_block;
    };
    let Some(last_event_block) = by_block.keys().next_back().copied() else {
        return without_block;
    };
    let start_block = from_block.unwrap_or(first_event_block);
    let end_block = to_block.unwrap_or(last_event_block);
    if start_block > end_block {
        return without_block;
    }

    let mut expanded = Vec::with_capacity(
        without_block
            .len()
            .saturating_add(by_block.values().map(Vec::len).sum())
            .saturating_add((end_block - start_block + 1) as usize),
    );
    for block in start_block..=end_block {
        let mut updated_pools = 0usize;
        if let Some(block_events) = by_block.remove(&block) {
            updated_pools = block_events
                .iter()
                .filter(|event| {
                    matches!(event, EngineEvent::Market(MarketEvent::PoolUpdated { .. }))
                })
                .count();
            expanded.extend(block_events);
        }
        expanded.push(EngineEvent::Market(MarketEvent::BlockCompleted {
            block_number: block,
            updated_tokens: 0,
            updated_pools,
        }));
    }
    expanded.extend(without_block);
    expanded
}

pub fn sort_events_by_block(mut events: Vec<EngineEvent>) -> Vec<EngineEvent> {
    events.sort_by_key(|event| (event_block(event).unwrap_or(u64::MAX), event_order(event)));
    events
}

fn event_block(event: &EngineEvent) -> Option<u64> {
    match event {
        EngineEvent::Market(
            MarketEvent::PoolUpdated { block_number, .. }
            | MarketEvent::TokenUpdated { block_number, .. }
            | MarketEvent::BlockCompleted { block_number, .. },
        ) => Some(*block_number),
        EngineEvent::Risk(risk) => risk.observed_block,
        EngineEvent::Execution(report) => report.block_number,
    }
}

fn event_order(event: &EngineEvent) -> u8 {
    match event {
        // A mined, value-destroying drain (confirmed liquidity removal / scam — not a
        // pending mempool projection) must be applied BEFORE the same block's market
        // valuation and execution. Otherwise the open-position valuation pass re-marks a
        // confiscated position positively (a holder-balance drain leaves pool reserves
        // intact, so a synthetic sell still "succeeds"), and an in-flight sell confirms
        // with phantom proceeds. Ordering the drain first lets the existing `drained`
        // guards zero the valuation and zero-close the position.
        EngineEvent::Risk(risk) if is_mined_value_destroying_risk(risk) => 0,
        EngineEvent::Market(MarketEvent::PoolUpdated { .. } | MarketEvent::TokenUpdated { .. }) => {
            1
        }
        EngineEvent::Risk(_) => 2,
        EngineEvent::Execution(_) => 3,
        EngineEvent::Market(MarketEvent::BlockCompleted { .. }) => 9,
    }
}

/// A confirmed (mined) liquidity-removal / scam drain, as opposed to a pending mempool
/// projection (`pending_tx_hash` set). Only mined drains reorder ahead of the block's
/// market events; mempool signals stay in normal order so they see current market state.
fn is_mined_value_destroying_risk(risk: &eth_alpha_core::risk::RiskEvent) -> bool {
    use eth_alpha_core::risk::RiskKind;
    matches!(
        risk.kind,
        RiskKind::LiquidityRemoval | RiskKind::ScamConfirmed
    ) && risk.pending_tx_hash.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth_alpha_core::risk::{RiskEvent, RiskKind, RiskSeverity};

    fn risk_event(kind: RiskKind, pending: bool) -> RiskEvent {
        RiskEvent {
            kind,
            severity: RiskSeverity::Critical,
            source: None,
            token_address: Default::default(),
            pool_address: None,
            pending_tx_hash: pending.then(Default::default),
            observed_block: Some(100),
            message: String::new(),
            evidence: None,
        }
    }

    #[test]
    fn mined_drain_leads_the_block_ahead_of_market_events() {
        // Mined liquidity-removal / scam drains lead the block (order 0), strictly ahead
        // of PoolUpdated/TokenUpdated (order 1), so the drain is applied before the
        // block's open-position valuation and sell execution.
        for kind in [RiskKind::LiquidityRemoval, RiskKind::ScamConfirmed] {
            let event = risk_event(kind, false);
            assert!(is_mined_value_destroying_risk(&event));
            assert_eq!(event_order(&EngineEvent::Risk(event)), 0);
        }
    }

    #[test]
    fn pending_mempool_drain_keeps_normal_order() {
        // A pending mempool projection is a predictive exit trigger, not a confirmed
        // drain: it stays in normal order (after market events) so it sees current state.
        let pending = risk_event(RiskKind::LiquidityRemoval, true);
        assert!(!is_mined_value_destroying_risk(&pending));
        assert_eq!(event_order(&EngineEvent::Risk(pending)), 2);
    }
}
