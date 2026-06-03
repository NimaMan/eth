use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceConfidence {
    Unknown,
    Low,
    Inferred,
    Medium,
    High,
    Confirmed,
}

impl Default for EvidenceConfidence {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    BasePoolProjection,
    PoolStateFlags,
    PoolEvent,
    ReserveTracker,
    Simulation,
    CustodyFinding,
    Trace,
    StateRead,
    Classification,
    Fixture,
}

impl Default for EvidenceSourceKind {
    fn default() -> Self {
        Self::BasePoolProjection
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub source: EvidenceSourceKind,
    pub confidence: EvidenceConfidence,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub log_index: Option<u64>,
    pub note: Option<String>,
}

impl EvidenceRef {
    pub fn new(source: EvidenceSourceKind, confidence: EvidenceConfidence) -> Self {
        Self {
            source,
            confidence,
            block_number: None,
            tx_hash: None,
            log_index: None,
            note: None,
        }
    }

    pub fn base_projection() -> Self {
        Self::new(
            EvidenceSourceKind::BasePoolProjection,
            EvidenceConfidence::Inferred,
        )
    }

    pub fn flags_projection() -> Self {
        Self::new(
            EvidenceSourceKind::PoolStateFlags,
            EvidenceConfidence::Inferred,
        )
    }

    pub fn at_block(mut self, block_number: Option<u64>) -> Self {
        self.block_number = block_number;
        self
    }

    pub fn with_tx(mut self, tx_hash: Option<String>) -> Self {
        self.tx_hash = tx_hash;
        self
    }

    pub fn with_log_index(mut self, log_index: Option<u64>) -> Self {
        self.log_index = log_index;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}
