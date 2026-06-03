use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RouteabilityTrack {
    pub trading_enabled: bool,
    pub raw_can_buy: bool,
    pub raw_can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub can_buy_and_sell: bool,
    pub economic_sellable: Option<bool>,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub last_failure_reason: Option<String>,
    pub last_failure_class: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl RouteabilityTrack {
    pub fn from_base_and_flags(base: &BasePool, flags: &PoolStateFlags) -> Self {
        Self {
            trading_enabled: flags.route.trading_enabled,
            raw_can_buy: flags.route.raw_can_buy,
            raw_can_sell: flags.route.raw_can_sell,
            effective_can_buy: flags.route.effective_can_buy,
            effective_can_sell: flags.route.effective_can_sell,
            can_buy_and_sell: flags.route.can_buy_and_sell,
            economic_sellable: flags.route.economic_sellable,
            buy_tax: base.buy_tax,
            sell_tax: base.sell_tax,
            last_failure_reason: base.last_trading_failure_reason.clone(),
            last_failure_class: base.last_trading_failure_class.clone(),
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::Simulation,
                EvidenceConfidence::High,
            )
            .at_block(base.tax_check_block)
            .with_tx(base.tax_check_tx.clone())],
        }
    }
}
