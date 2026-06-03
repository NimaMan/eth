use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::supply_control::SupplyControlObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupplyControlSignal {
    Mintable,
    OwnerCanChangeBalance,
    OwnerCanBurnHolder,
    Rebasing,
    Reflection,
    FeeOnTransfer,
}

impl SupplyControlSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mintable => "mintable",
            Self::OwnerCanChangeBalance => "owner_can_change_balance",
            Self::OwnerCanBurnHolder => "owner_can_burn_holder",
            Self::Rebasing => "rebasing",
            Self::Reflection => "reflection",
            Self::FeeOnTransfer => "fee_on_transfer",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mintable => "supply:mintable",
            Self::OwnerCanChangeBalance => "supply:owner_can_change_balance",
            Self::OwnerCanBurnHolder => "supply:owner_can_burn_holder",
            Self::Rebasing => "supply:rebasing",
            Self::Reflection => "supply:reflection",
            Self::FeeOnTransfer => "supply:fee_on_transfer",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SupplyControlState {
    pub signal: SupplyControlSignal,
    pub present: bool,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub amount: Option<f64>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SupplyControlTrack {
    pub mintable: bool,
    pub owner_can_change_balance: bool,
    pub owner_can_burn_holder: bool,
    pub rebasing: bool,
    pub reflection: bool,
    pub fee_on_transfer: bool,
    pub controls: Vec<SupplyControlState>,
    pub evidence: Vec<EvidenceRef>,
}

impl SupplyControlTrack {
    pub fn from_observations(observations: &[SupplyControlObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &SupplyControlObservation) {
        if observation.present {
            match observation.signal {
                SupplyControlSignal::Mintable => self.mintable = true,
                SupplyControlSignal::OwnerCanChangeBalance => self.owner_can_change_balance = true,
                SupplyControlSignal::OwnerCanBurnHolder => self.owner_can_burn_holder = true,
                SupplyControlSignal::Rebasing => self.rebasing = true,
                SupplyControlSignal::Reflection => self.reflection = true,
                SupplyControlSignal::FeeOnTransfer => self.fee_on_transfer = true,
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.controls.push(SupplyControlState {
            signal: observation.signal,
            present: observation.present,
            actor: observation.actor.clone(),
            subject: observation.subject.clone(),
            amount: observation.amount,
            selector: observation.selector.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.mintable {
            labels.push(SupplyControlSignal::Mintable.label());
        }
        if self.owner_can_change_balance {
            labels.push(SupplyControlSignal::OwnerCanChangeBalance.label());
        }
        if self.owner_can_burn_holder {
            labels.push(SupplyControlSignal::OwnerCanBurnHolder.label());
        }
        if self.rebasing {
            labels.push(SupplyControlSignal::Rebasing.label());
        }
        if self.reflection {
            labels.push(SupplyControlSignal::Reflection.label());
        }
        if self.fee_on_transfer {
            labels.push(SupplyControlSignal::FeeOnTransfer.label());
        }
        labels
    }
}
