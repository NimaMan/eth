//! Transaction-level network update extraction.

use std::collections::BTreeSet;

use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::network::{
    activity::{AddressCostRecord, AddressMovement},
    model::{
        NetworkEdge, NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkLabel,
        NetworkLabelKind, NetworkLabelSource, NetworkNodeId, NetworkObservation,
    },
};

/// Transaction context shared by all ingest adapters.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionIngestContext {
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub from_address: String,
    pub to_address: Option<String>,
    pub status: bool,
}

impl TransactionIngestContext {
    pub fn from_transaction(tx: &ProcessedTransaction) -> Self {
        Self {
            tx_hash: hash_string(&tx.hash),
            block_number: tx.block_number,
            block_timestamp: tx.block_timestamp,
            tx_index: tx.tx_index,
            from_address: address_string(&tx.from_address),
            to_address: tx.to_address.map(|address| address_string(&address)),
            status: tx.status,
        }
    }

    pub fn observation(&self, log_index: Option<u64>) -> NetworkObservation {
        NetworkObservation::with_transaction(
            self.block_number,
            Some(self.block_timestamp),
            Some(self.tx_hash.clone()),
            Some(self.tx_index),
            log_index,
        )
    }

    pub fn from_node(&self) -> NetworkNodeId {
        NetworkNodeId::address(&self.from_address)
    }
}

/// Address activity movement extracted from one transaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressMovementIngest {
    pub address: String,
    pub movement: AddressMovement,
}

impl AddressMovementIngest {
    pub fn new(address: impl AsRef<str>, movement: AddressMovement) -> Self {
        Self {
            address: normalize_address(address),
            movement,
        }
    }
}

/// Address fee/bribe cost extracted from one transaction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressCostIngest {
    pub address: String,
    pub cost: AddressCostRecord,
}

impl AddressCostIngest {
    pub fn new(address: impl AsRef<str>, cost: AddressCostRecord) -> Self {
        Self {
            address: normalize_address(address),
            cost,
        }
    }
}

/// Label inferred or observed during ingestion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeLabelIngest {
    pub node_id: NetworkNodeId,
    pub label: NetworkLabel,
    pub evidence: Option<NetworkEvidence>,
}

impl NodeLabelIngest {
    pub fn observed(
        node_id: NetworkNodeId,
        kind: NetworkLabelKind,
        source: NetworkLabelSource,
        observation: NetworkObservation,
        evidence_source: NetworkEvidenceSource,
        description: impl Into<String>,
    ) -> Self {
        let mut label = NetworkLabel::observed(kind, source);
        label.observed.record(observation.clone());
        Self {
            node_id,
            label,
            evidence: Some(
                NetworkEvidence::new(evidence_source)
                    .with_observation(observation)
                    .with_description(description),
            ),
        }
    }
}

/// Common output batch produced by ingest adapters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkIngestBatch {
    pub address_movements: Vec<AddressMovementIngest>,
    pub address_costs: Vec<AddressCostIngest>,
    pub node_labels: Vec<NodeLabelIngest>,
    pub edges: Vec<NetworkEdge>,
}

impl NetworkIngestBatch {
    pub fn is_empty(&self) -> bool {
        self.address_movements.is_empty()
            && self.address_costs.is_empty()
            && self.node_labels.is_empty()
            && self.edges.is_empty()
    }

    pub fn extend(&mut self, other: Self) {
        self.address_movements.extend(other.address_movements);
        self.address_costs.extend(other.address_costs);
        self.node_labels.extend(other.node_labels);
        self.edges.extend(other.edges);
    }

    pub fn push_edge(&mut self, edge: NetworkEdge) {
        self.edges.push(edge);
    }

    pub fn push_label(&mut self, label: NodeLabelIngest) {
        self.node_labels.push(label);
    }

    pub fn push_movement(&mut self, address: impl AsRef<str>, movement: AddressMovement) {
        self.address_movements
            .push(AddressMovementIngest::new(address, movement));
    }

    pub fn push_cost(&mut self, address: impl AsRef<str>, cost: AddressCostRecord) {
        self.address_costs
            .push(AddressCostIngest::new(address, cost));
    }
}

/// Transaction-only updates: fee/bribe costs and fee-source coactivity links.
pub fn extract_transaction_updates(tx: &ProcessedTransaction) -> NetworkIngestBatch {
    let mut batch = NetworkIngestBatch::default();
    if !tx.status {
        return batch;
    }

    let context = TransactionIngestContext::from_transaction(tx);
    let fee = scale_u256(tx.fees.tx_fee, 18);
    let bribe = scale_u256(tx.bribe_amount, 18);
    if fee > 0.0 || bribe > 0.0 {
        batch.push_cost(
            &context.from_address,
            AddressCostRecord::new(context.observation(None), fee, bribe),
        );
    }

    batch.push_label(NodeLabelIngest::observed(
        context.from_node(),
        NetworkLabelKind::Wallet,
        NetworkLabelSource::TransferFlow,
        context.observation(None),
        NetworkEvidenceSource::FeeSourceTouch,
        "transaction fee source",
    ));

    for touched in touched_addresses(tx) {
        if touched == context.from_address {
            continue;
        }
        let mut evidence = NetworkEvidence::new(NetworkEvidenceSource::FeeSourceTouch)
            .with_observation(context.observation(None))
            .with_description("fee source touched address in transaction");
        evidence
            .attributes
            .insert("tx_hash".to_string(), context.tx_hash.clone());
        let mut edge = NetworkEdge::new(
            NetworkNodeId::address(&context.from_address),
            NetworkNodeId::address(touched),
            NetworkEdgeKind::FeeSourceTouches,
        );
        edge.record_evidence(evidence);
        batch.push_edge(edge);
    }

    batch
}

pub fn edge_with_evidence(
    source: NetworkNodeId,
    target: NetworkNodeId,
    kind: NetworkEdgeKind,
    evidence: NetworkEvidence,
) -> NetworkEdge {
    let mut edge = NetworkEdge::new(source, target, kind);
    edge.record_evidence(evidence);
    edge
}

pub fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

pub fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

pub fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

pub fn scale_u256(value: U256, decimals: u8) -> f64 {
    if value.is_zero() {
        return 0.0;
    }
    let raw = value.to_string().parse::<f64>().unwrap_or(0.0);
    raw / 10_f64.powi(i32::from(decimals))
}

fn touched_addresses(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();
    for address in &tx.unique_addresses {
        addresses.insert(address_string(address));
    }
    for address in tx.address_balance_changes.keys() {
        addresses.insert(address_string(address));
    }
    if let Some(address) = tx.to_address {
        addresses.insert(address_string(&address));
    }
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, U256};

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            address!("1111111111111111111111111111111111111111"),
            Some(address!("2222222222222222222222222222222222222222")),
            U256::ZERO,
            true,
            7,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn transaction_ingest_records_costs_and_fee_source_edges() {
        let mut tx = tx();
        tx.fees.tx_fee = U256::from(1_000_000_000_000_000_u64);
        tx.bribe_amount = U256::from(2_000_000_000_000_000_u64);
        tx.unique_addresses
            .insert(address!("3333333333333333333333333333333333333333"));

        let batch = extract_transaction_updates(&tx);

        assert_eq!(batch.address_costs.len(), 1);
        assert_eq!(batch.address_costs[0].cost.tx_fee, 0.001);
        assert_eq!(batch.address_costs[0].cost.bribe_amount, 0.002);
        assert_eq!(batch.edges.len(), 2);
        assert!(batch
            .edges
            .iter()
            .all(|edge| edge.kind == NetworkEdgeKind::FeeSourceTouches));
    }

    #[test]
    fn failed_transaction_produces_no_updates() {
        let mut tx = tx();
        tx.status = false;

        assert!(extract_transaction_updates(&tx).is_empty());
    }
}
