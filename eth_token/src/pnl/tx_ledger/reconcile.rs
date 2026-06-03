use std::collections::{BTreeMap, BTreeSet};

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

use super::model::{AssetFamily, RawDelta, RawSourceKind, TxAsset, TxLedgerContext, TxMovement};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressReconciliationKind {
    FamilyPassThrough,
    NetChange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AddressReconciliationSummary {
    pub address: Address,
    pub movement_count: u64,
    pub asset_deltas: BTreeMap<TxAsset, RawDelta>,
    pub family_deltas: BTreeMap<AssetFamily, RawDelta>,
    pub raw_sources: BTreeSet<RawSourceKind>,
    pub kind: AddressReconciliationKind,
}

impl AddressReconciliationSummary {
    pub fn asset_delta(&self, asset: TxAsset) -> Option<&RawDelta> {
        self.asset_deltas.get(&asset)
    }

    pub fn family_delta(&self, family: AssetFamily) -> Option<&RawDelta> {
        self.family_deltas.get(&family)
    }

    pub fn is_family_net_zero(&self, family: AssetFamily) -> bool {
        self.family_delta(family)
            .map(RawDelta::is_net_zero)
            .unwrap_or(true)
    }

    pub fn is_pass_through(&self) -> bool {
        self.kind == AddressReconciliationKind::FamilyPassThrough
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReconciledTxLedger {
    pub context: TxLedgerContext,
    pub addresses: BTreeMap<Address, AddressReconciliationSummary>,
}

impl ReconciledTxLedger {
    pub fn address(&self, address: &Address) -> Option<&AddressReconciliationSummary> {
        self.addresses.get(address)
    }
}

#[derive(Default)]
struct AddressAccumulator {
    movement_count: u64,
    asset_deltas: BTreeMap<TxAsset, RawDelta>,
    family_deltas: BTreeMap<AssetFamily, RawDelta>,
    raw_sources: BTreeSet<RawSourceKind>,
}

pub fn reconcile_movements(
    context: TxLedgerContext,
    movements: &[TxMovement],
) -> ReconciledTxLedger {
    let mut accumulators: BTreeMap<Address, AddressAccumulator> = BTreeMap::new();

    for movement in movements {
        let family = context.asset_family(movement.asset);

        {
            let accumulator = accumulators.entry(movement.from).or_default();
            accumulator.movement_count = accumulator.movement_count.saturating_add(1);
            accumulator.raw_sources.insert(movement.source.kind);
            accumulator
                .asset_deltas
                .entry(movement.asset)
                .or_default()
                .record_outgoing(movement.amount);
            accumulator
                .family_deltas
                .entry(family)
                .or_default()
                .record_outgoing(movement.amount);
        }

        {
            let accumulator = accumulators.entry(movement.to).or_default();
            accumulator.movement_count = accumulator.movement_count.saturating_add(1);
            accumulator.raw_sources.insert(movement.source.kind);
            accumulator
                .asset_deltas
                .entry(movement.asset)
                .or_default()
                .record_incoming(movement.amount);
            accumulator
                .family_deltas
                .entry(family)
                .or_default()
                .record_incoming(movement.amount);
        }
    }

    let addresses = accumulators
        .into_iter()
        .map(|(address, accumulator)| {
            let kind = if accumulator
                .family_deltas
                .values()
                .all(RawDelta::is_net_zero)
            {
                AddressReconciliationKind::FamilyPassThrough
            } else {
                AddressReconciliationKind::NetChange
            };
            (
                address,
                AddressReconciliationSummary {
                    address,
                    movement_count: accumulator.movement_count,
                    asset_deltas: accumulator.asset_deltas,
                    family_deltas: accumulator.family_deltas,
                    raw_sources: accumulator.raw_sources,
                    kind,
                },
            )
        })
        .collect();

    ReconciledTxLedger { context, addresses }
}
