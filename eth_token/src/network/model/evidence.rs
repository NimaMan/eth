//! Evidence records supporting graph edges and inferred clusters.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Block-level location for an observed event.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NetworkObservation {
    pub block_number: u64,
    pub block_timestamp: Option<u64>,
    pub tx_hash: Option<String>,
    pub tx_index: Option<u64>,
    pub log_index: Option<u64>,
}

impl NetworkObservation {
    pub fn new(block_number: u64) -> Self {
        Self {
            block_number,
            block_timestamp: None,
            tx_hash: None,
            tx_index: None,
            log_index: None,
        }
    }

    pub fn with_transaction(
        block_number: u64,
        block_timestamp: Option<u64>,
        tx_hash: Option<impl Into<String>>,
        tx_index: Option<u64>,
        log_index: Option<u64>,
    ) -> Self {
        Self {
            block_number,
            block_timestamp,
            tx_hash: tx_hash.map(Into::into),
            tx_index,
            log_index,
        }
    }

    fn order_key(&self) -> (u64, u64, u64) {
        (
            self.block_number,
            self.tx_index.unwrap_or(u64::MAX),
            self.log_index.unwrap_or(u64::MAX),
        )
    }
}

/// First/last seen range plus evidence count.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationRange {
    pub first_seen: Option<NetworkObservation>,
    pub last_seen: Option<NetworkObservation>,
    pub count: u64,
}

impl ObservationRange {
    pub fn record(&mut self, observation: NetworkObservation) {
        self.count += 1;

        if self
            .first_seen
            .as_ref()
            .map(|first| observation.order_key() < first.order_key())
            .unwrap_or(true)
        {
            self.first_seen = Some(observation.clone());
        }

        if self
            .last_seen
            .as_ref()
            .map(|last| observation.order_key() >= last.order_key())
            .unwrap_or(true)
        {
            self.last_seen = Some(observation);
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.count += other.count;
        if let Some(first_seen) = other.first_seen.clone() {
            if self
                .first_seen
                .as_ref()
                .map(|first| first_seen.order_key() < first.order_key())
                .unwrap_or(true)
            {
                self.first_seen = Some(first_seen);
            }
        }
        if let Some(last_seen) = other.last_seen.clone() {
            if self
                .last_seen
                .as_ref()
                .map(|last| last_seen.order_key() >= last.order_key())
                .unwrap_or(true)
            {
                self.last_seen = Some(last_seen);
            }
        }
    }
}

/// Source family for an evidence record.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEvidenceSource {
    TokenTransfer,
    DenomTransfer,
    PoolTrade,
    LpTransfer,
    LpApproval,
    FeeSourceTouch,
    Funding,
    Authority,
    PoolState,
    SharedIntermediary,
    TemporalCoactivity,
    Heuristic,
    Manual,
}

/// Amount observed in a transfer, trade, or LP event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkAmount {
    pub asset: Option<String>,
    pub symbol: Option<String>,
    pub raw: Option<String>,
    pub scaled: Option<f64>,
}

impl NetworkAmount {
    pub fn scaled(asset: impl Into<String>, symbol: Option<impl Into<String>>, value: f64) -> Self {
        Self {
            asset: Some(asset.into()),
            symbol: symbol.map(Into::into),
            raw: None,
            scaled: Some(value),
        }
    }

    pub fn raw(
        asset: impl Into<String>,
        symbol: Option<impl Into<String>>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            asset: Some(asset.into()),
            symbol: symbol.map(Into::into),
            raw: Some(value.into()),
            scaled: None,
        }
    }
}

/// One piece of evidence backing a node, edge, or cluster.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkEvidence {
    pub source: NetworkEvidenceSource,
    pub observation: Option<NetworkObservation>,
    pub description: Option<String>,
    pub amount: Option<NetworkAmount>,
    pub attributes: BTreeMap<String, String>,
}

impl NetworkEvidence {
    pub fn new(source: NetworkEvidenceSource) -> Self {
        Self {
            source,
            observation: None,
            description: None,
            amount: None,
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_observation(mut self, observation: NetworkObservation) -> Self {
        self.observation = Some(observation);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Bounded evidence summary for collapsed graph state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub observed: ObservationRange,
    pub examples: Vec<NetworkEvidence>,
    pub max_examples: usize,
}

impl EvidenceSummary {
    pub fn new(max_examples: usize) -> Self {
        Self {
            observed: ObservationRange::default(),
            examples: Vec::new(),
            max_examples,
        }
    }

    pub fn record(&mut self, evidence: NetworkEvidence) {
        if let Some(observation) = evidence.observation.clone() {
            self.observed.record(observation);
        } else {
            self.observed.count += 1;
        }

        if self.examples.len() < self.max_examples {
            self.examples.push(evidence);
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.observed.merge(&other.observed);
        for evidence in other.examples {
            if self.examples.len() >= self.max_examples {
                break;
            }
            self.examples.push(evidence);
        }
    }
}

impl Default for EvidenceSummary {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_range_tracks_first_and_last_seen() {
        let mut range = ObservationRange::default();
        range.record(NetworkObservation::with_transaction(
            100,
            None,
            Some("0xbbb"),
            Some(5),
            Some(1),
        ));
        range.record(NetworkObservation::with_transaction(
            99,
            None,
            Some("0xaaa"),
            Some(1),
            Some(1),
        ));
        range.record(NetworkObservation::with_transaction(
            100,
            None,
            Some("0xccc"),
            Some(7),
            Some(1),
        ));

        assert_eq!(range.count, 3);
        assert_eq!(
            range
                .first_seen
                .as_ref()
                .and_then(|obs| obs.tx_hash.as_deref()),
            Some("0xaaa")
        );
        assert_eq!(
            range
                .last_seen
                .as_ref()
                .and_then(|obs| obs.tx_hash.as_deref()),
            Some("0xccc")
        );
    }

    #[test]
    fn evidence_summary_bounds_examples() {
        let mut summary = EvidenceSummary::new(1);
        summary.record(NetworkEvidence::new(NetworkEvidenceSource::TokenTransfer));
        summary.record(NetworkEvidence::new(NetworkEvidenceSource::DenomTransfer));

        assert_eq!(summary.observed.count, 2);
        assert_eq!(summary.examples.len(), 1);
    }
}
