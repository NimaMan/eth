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
        EngineEvent::Market(MarketEvent::PoolUpdated { .. } | MarketEvent::TokenUpdated { .. }) => {
            0
        }
        EngineEvent::Risk(_) => 1,
        EngineEvent::Execution(_) => 2,
        EngineEvent::Market(MarketEvent::BlockCompleted { .. }) => 9,
    }
}
