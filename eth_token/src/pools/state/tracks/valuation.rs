use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationState {
    NoMark,
    Priced,
    TerminalZero,
}

impl Default for ValuationState {
    fn default() -> Self {
        Self::NoMark
    }
}

impl ValuationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoMark => "no_mark",
            Self::Priced => "priced",
            Self::TerminalZero => "terminal_zero",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValuationTrack {
    pub state: ValuationState,
    pub price_denom_per_token: Option<f64>,
    pub raw_reserve_price_denom_per_token: Option<f64>,
    pub price_source: Option<String>,
    pub price_history_len: usize,
    pub priced_from_reserves: bool,
    pub terminal_zero: bool,
    pub evidence: Vec<EvidenceRef>,
}

impl ValuationTrack {
    pub fn from_base_and_flags(base: &BasePool, flags: &PoolStateFlags) -> Self {
        let raw_price = finite_positive(base.state.price_denom_per_token);
        let terminal_zero =
            flags.risk.terminal_position_risk || flags.liquidity.reserve_liquidity_removed;
        let state = if terminal_zero {
            ValuationState::TerminalZero
        } else if flags.liquidity.has_reserves && raw_price.is_some() {
            ValuationState::Priced
        } else {
            ValuationState::NoMark
        };

        Self {
            state,
            price_denom_per_token: match state {
                ValuationState::Priced => raw_price,
                ValuationState::TerminalZero => Some(0.0),
                ValuationState::NoMark => None,
            },
            raw_reserve_price_denom_per_token: raw_price,
            price_source: (state == ValuationState::Priced).then(|| "reserves".to_string()),
            price_history_len: base.price_history.len(),
            priced_from_reserves: state == ValuationState::Priced,
            terminal_zero,
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::ReserveTracker,
                EvidenceConfidence::High,
            )
            .at_block(Some(base.state.last_update_block))],
        }
    }
}

fn finite_positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}
