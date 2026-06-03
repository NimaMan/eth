use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::contract_posture::ContractPostureObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractPostureSignal {
    ClosedSource,
    Proxy,
    ExternalPolicyCall,
    HiddenOwner,
    OwnershipReclaimable,
    SelfdestructCapable,
}

impl ContractPostureSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClosedSource => "closed_source",
            Self::Proxy => "proxy",
            Self::ExternalPolicyCall => "external_policy_call",
            Self::HiddenOwner => "hidden_owner",
            Self::OwnershipReclaimable => "ownership_reclaimable",
            Self::SelfdestructCapable => "selfdestruct_capable",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ClosedSource => "contract:closed_source",
            Self::Proxy => "contract:proxy",
            Self::ExternalPolicyCall => "contract:external_policy_call",
            Self::HiddenOwner => "contract:hidden_owner",
            Self::OwnershipReclaimable => "contract:ownership_reclaimable",
            Self::SelfdestructCapable => "contract:selfdestruct_capable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContractPostureState {
    pub signal: ContractPostureSignal,
    pub present: bool,
    pub contract_address: Option<String>,
    pub implementation_address: Option<String>,
    pub owner_address: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContractPostureTrack {
    pub closed_source: bool,
    pub proxy: bool,
    pub external_policy_call: bool,
    pub hidden_owner: bool,
    pub ownership_reclaimable: bool,
    pub selfdestruct_capable: bool,
    pub posture: Vec<ContractPostureState>,
    pub evidence: Vec<EvidenceRef>,
}

impl ContractPostureTrack {
    pub fn from_observations(observations: &[ContractPostureObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &ContractPostureObservation) {
        if observation.present {
            match observation.signal {
                ContractPostureSignal::ClosedSource => self.closed_source = true,
                ContractPostureSignal::Proxy => self.proxy = true,
                ContractPostureSignal::ExternalPolicyCall => self.external_policy_call = true,
                ContractPostureSignal::HiddenOwner => self.hidden_owner = true,
                ContractPostureSignal::OwnershipReclaimable => self.ownership_reclaimable = true,
                ContractPostureSignal::SelfdestructCapable => self.selfdestruct_capable = true,
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.posture.push(ContractPostureState {
            signal: observation.signal,
            present: observation.present,
            contract_address: observation.contract_address.clone(),
            implementation_address: observation.implementation_address.clone(),
            owner_address: observation.owner_address.clone(),
            selector: observation.selector.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.closed_source {
            labels.push(ContractPostureSignal::ClosedSource.label());
        }
        if self.proxy {
            labels.push(ContractPostureSignal::Proxy.label());
        }
        if self.external_policy_call {
            labels.push(ContractPostureSignal::ExternalPolicyCall.label());
        }
        if self.hidden_owner {
            labels.push(ContractPostureSignal::HiddenOwner.label());
        }
        if self.ownership_reclaimable {
            labels.push(ContractPostureSignal::OwnershipReclaimable.label());
        }
        if self.selfdestruct_capable {
            labels.push(ContractPostureSignal::SelfdestructCapable.label());
        }
        labels
    }
}
