use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::{AssetFamily, RawDelta, TxAsset, TxLedgerContext, TxMovement};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConservationSummary {
    pub by_asset: BTreeMap<TxAsset, RawDelta>,
    pub by_family: BTreeMap<AssetFamily, RawDelta>,
}

impl ConservationSummary {
    pub fn asset_is_conserved(&self, asset: TxAsset) -> bool {
        self.by_asset
            .get(&asset)
            .map(RawDelta::is_net_zero)
            .unwrap_or(true)
    }

    pub fn family_is_conserved(&self, family: AssetFamily) -> bool {
        self.by_family
            .get(&family)
            .map(RawDelta::is_net_zero)
            .unwrap_or(true)
    }
}

pub fn conservation_summary(
    context: TxLedgerContext,
    movements: &[TxMovement],
) -> ConservationSummary {
    let mut by_asset: BTreeMap<TxAsset, RawDelta> = BTreeMap::new();
    let mut by_family: BTreeMap<AssetFamily, RawDelta> = BTreeMap::new();

    for movement in movements {
        by_asset
            .entry(movement.asset)
            .or_default()
            .record_incoming(movement.amount);
        by_asset
            .entry(movement.asset)
            .or_default()
            .record_outgoing(movement.amount);

        let family = context.asset_family(movement.asset);
        by_family
            .entry(family)
            .or_default()
            .record_incoming(movement.amount);
        by_family
            .entry(family)
            .or_default()
            .record_outgoing(movement.amount);
    }

    ConservationSummary {
        by_asset,
        by_family,
    }
}
