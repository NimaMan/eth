use serde::{Deserialize, Serialize};

use crate::custody::{CustodyCapability, CustodyFinding, CustodyState};
use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCustodyCapability {
    Seize,
    Freeze,
    BurnDrain,
    Clawback,
}

impl TokenCustodyCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Seize => "custody_seize",
            Self::Freeze => "custody_freeze",
            Self::BurnDrain => "custody_burn_drain",
            Self::Clawback => "custody_clawback",
        }
    }
}

impl From<CustodyCapability> for TokenCustodyCapability {
    fn from(capability: CustodyCapability) -> Self {
        match capability {
            CustodyCapability::Seize => Self::Seize,
            CustodyCapability::Freeze => Self::Freeze,
            CustodyCapability::BurnDrain => Self::BurnDrain,
            CustodyCapability::Clawback => Self::Clawback,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyFindingState {
    Latent,
    Realized,
}

impl CustodyFindingState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Latent => "latent",
            Self::Realized => "realized",
        }
    }
}

impl From<CustodyState> for CustodyFindingState {
    fn from(state: CustodyState) -> Self {
        match state {
            CustodyState::Latent => Self::Latent,
            CustodyState::Realized => Self::Realized,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustodyCapabilityState {
    pub capability: TokenCustodyCapability,
    pub state: CustodyFindingState,
    pub label: String,
    pub block_number: Option<u64>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CustodyTrack {
    pub latent: bool,
    pub realized: bool,
    pub finding_count: usize,
    pub capabilities: Vec<CustodyCapabilityState>,
    pub token_control_addresses: Vec<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl CustodyTrack {
    pub fn from_base_flags_and_findings(
        base: &BasePool,
        flags: &PoolStateFlags,
        findings: &[CustodyFinding],
    ) -> Self {
        let mut capabilities = Vec::new();
        for finding in findings {
            let confidence = match finding.state {
                CustodyState::Latent => EvidenceConfidence::Medium,
                CustodyState::Realized => EvidenceConfidence::Confirmed,
            };
            let evidence = EvidenceRef::new(EvidenceSourceKind::CustodyFinding, confidence)
                .at_block(finding.block_number);
            capabilities.push(CustodyCapabilityState {
                capability: finding.capability.into(),
                state: finding.state.into(),
                label: finding.capability.label().to_string(),
                block_number: finding.block_number,
                evidence: vec![evidence],
            });
        }

        let mut token_control_addresses: Vec<_> =
            base.token_control_addresses.iter().cloned().collect();
        token_control_addresses.sort();

        Self {
            latent: flags.custody.latent,
            realized: flags.custody.realized,
            finding_count: flags.custody.finding_count,
            capabilities,
            token_control_addresses,
            evidence: vec![EvidenceRef::flags_projection().at_block(flags.latest_block_number)],
        }
    }
}
