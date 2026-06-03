use serde::{Deserialize, Serialize};

use crate::pools::data_models::PoolLifecycle;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::EvidenceRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    Discovered,
    LiquidityDeposited,
    Trading,
    CannotSell,
    Dust,
    Drained,
    Active,
    LiquidityRemoved,
    Evicted,
}

impl Default for LifecyclePhase {
    fn default() -> Self {
        Self::Discovered
    }
}

impl LifecyclePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::LiquidityDeposited => "liquidity_deposited",
            Self::Trading => "trading",
            Self::CannotSell => "cannot_sell",
            Self::Dust => "dust",
            Self::Drained => "drained",
            Self::Active => "active",
            Self::LiquidityRemoved => "liquidity_removed",
            Self::Evicted => "evicted",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Drained | Self::LiquidityRemoved | Self::Evicted)
    }
}

impl From<PoolLifecycle> for LifecyclePhase {
    fn from(lifecycle: PoolLifecycle) -> Self {
        match lifecycle {
            PoolLifecycle::Discovered => Self::Discovered,
            PoolLifecycle::LiquidityDeposited => Self::LiquidityDeposited,
            PoolLifecycle::Trading => Self::Trading,
            PoolLifecycle::CannotSell => Self::CannotSell,
            PoolLifecycle::Dust => Self::Dust,
            PoolLifecycle::Drained => Self::Drained,
            PoolLifecycle::Active => Self::Active,
            PoolLifecycle::LiquidityRemoved => Self::LiquidityRemoved,
            PoolLifecycle::Evicted => Self::Evicted,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LifecycleTrack {
    pub phase: LifecyclePhase,
    pub legacy_lifecycle: String,
    pub terminal: bool,
    pub latest_block_number: Option<u64>,
    pub evidence: Vec<EvidenceRef>,
}

impl LifecycleTrack {
    pub fn from_lifecycle(lifecycle: PoolLifecycle, flags: &PoolStateFlags) -> Self {
        let phase = LifecyclePhase::from(lifecycle);
        Self {
            phase,
            legacy_lifecycle: flags.lifecycle.clone(),
            terminal: phase.is_terminal() || flags.risk.terminal_position_risk,
            latest_block_number: flags.latest_block_number,
            evidence: vec![EvidenceRef::flags_projection().at_block(flags.latest_block_number)],
        }
    }
}
