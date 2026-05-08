//! Authority/control-derived network update extraction.

use alloy_primitives::Address;
use tx_processor::ProcessedTransaction;

use crate::network::{
    ingest::transaction::{
        address_string, edge_with_evidence, NetworkIngestBatch, NodeLabelIngest,
        TransactionIngestContext,
    },
    model::{
        NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkLabelKind,
        NetworkLabelSource, NetworkNodeId,
    },
};

/// Extract token authority, control, and policy relationships.
pub fn extract_authority_updates(
    tx: &ProcessedTransaction,
    tracked_token_address: impl AsRef<str>,
) -> NetworkIngestBatch {
    let mut batch = NetworkIngestBatch::default();
    if !tx.status {
        return batch;
    }

    let context = TransactionIngestContext::from_transaction(tx);
    let tracked_token_address = tracked_token_address.as_ref().trim().to_ascii_lowercase();
    let token_node = NetworkNodeId::token(&tracked_token_address);

    for event in &tx.contract_creation_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let creator = context.from_address.clone();
        push_control_label_and_edge(
            &mut batch,
            &context,
            &creator,
            token_node.clone(),
            NetworkLabelKind::Creator,
            None,
            "contract_creation",
            "creator created tracked token contract",
        );
    }

    for event in &tx.ownership_transferred_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let new_owner = address_string(&event.new_owner);
        push_control_label_and_edge(
            &mut batch,
            &context,
            &new_owner,
            token_node.clone(),
            owner_label(&event.new_owner),
            Some(event.log_index),
            "ownership_transferred",
            "ownership transferred for tracked token",
        );
    }

    for event in &tx.ownership_transfer_started_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let pending_owner = address_string(&event.new_owner);
        push_control_label_and_edge(
            &mut batch,
            &context,
            &pending_owner,
            token_node.clone(),
            owner_label(&event.new_owner).then_pending(),
            Some(event.log_index),
            "ownership_transfer_started",
            "pending owner set for tracked token",
        );
    }

    for event in &tx.access_control_role_granted_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let account = address_string(&event.account);
        let mut edge = control_edge(
            &context,
            &account,
            token_node.clone(),
            Some(event.log_index),
            "access_control_role_granted",
            "access-control role granted on tracked token",
        );
        edge.attributes
            .insert("role".to_string(), format!("{:#x}", event.role));
        batch.push_edge(edge);
        batch.push_label(NodeLabelIngest::observed(
            NetworkNodeId::address(account),
            NetworkLabelKind::Admin,
            NetworkLabelSource::TokenState,
            context.observation(Some(event.log_index)),
            NetworkEvidenceSource::Authority,
            "admin role granted on tracked token",
        ));
    }

    for event in &tx.access_control_role_revoked_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let account = address_string(&event.account);
        let mut edge = control_edge(
            &context,
            &account,
            token_node.clone(),
            Some(event.log_index),
            "access_control_role_revoked",
            "access-control role revoked on tracked token",
        );
        edge.attributes
            .insert("role".to_string(), format!("{:#x}", event.role));
        batch.push_edge(edge);
    }

    for event in &tx.proxy_admin_changed_events {
        if !same_address(&event.contract_address, &tracked_token_address) {
            continue;
        }
        let new_admin = address_string(&event.new_admin);
        push_control_label_and_edge(
            &mut batch,
            &context,
            &new_admin,
            token_node.clone(),
            NetworkLabelKind::ProxyAdmin,
            Some(event.log_index),
            "proxy_admin_changed",
            "proxy admin changed for tracked token",
        );
    }

    for event in &tx.trading_enabled_events {
        if !same_address(&event.token_address, &tracked_token_address) {
            continue;
        }
        batch.push_edge(control_edge(
            &context,
            &context.from_address,
            token_node.clone(),
            Some(event.log_index),
            "trading_enabled",
            "trading enabled for tracked token",
        ));
    }

    for event in &tx.trading_disabled_events {
        if !same_address(&event.token_address, &tracked_token_address) {
            continue;
        }
        batch.push_edge(control_edge(
            &context,
            &context.from_address,
            token_node.clone(),
            Some(event.log_index),
            "trading_disabled",
            "trading disabled for tracked token",
        ));
    }

    batch
}

fn push_control_label_and_edge(
    batch: &mut NetworkIngestBatch,
    context: &TransactionIngestContext,
    actor: &str,
    target: NetworkNodeId,
    label_kind: NetworkLabelKind,
    log_index: Option<u64>,
    event_name: &str,
    description: &str,
) {
    batch.push_label(NodeLabelIngest::observed(
        NetworkNodeId::address(actor),
        label_kind,
        NetworkLabelSource::TokenState,
        context.observation(log_index),
        NetworkEvidenceSource::Authority,
        description,
    ));
    batch.push_edge(control_edge(
        context,
        actor,
        target,
        log_index,
        event_name,
        description,
    ));
}

fn control_edge(
    context: &TransactionIngestContext,
    actor: &str,
    target: NetworkNodeId,
    log_index: Option<u64>,
    event_name: &str,
    description: &str,
) -> crate::network::model::NetworkEdge {
    let mut evidence = NetworkEvidence::new(NetworkEvidenceSource::Authority)
        .with_observation(context.observation(log_index))
        .with_description(description);
    evidence
        .attributes
        .insert("event".to_string(), event_name.to_string());
    edge_with_evidence(
        NetworkNodeId::address(actor),
        target,
        NetworkEdgeKind::ControlRelation,
        evidence,
    )
}

fn owner_label(owner: &Address) -> NetworkLabelKind {
    if *owner == Address::ZERO {
        NetworkLabelKind::ZeroAddress
    } else {
        NetworkLabelKind::Owner
    }
}

trait PendingOwnerLabel {
    fn then_pending(self) -> Self;
}

impl PendingOwnerLabel for NetworkLabelKind {
    fn then_pending(self) -> Self {
        match self {
            NetworkLabelKind::ZeroAddress => NetworkLabelKind::ZeroAddress,
            _ => NetworkLabelKind::PendingOwner,
        }
    }
}

fn same_address(address: &Address, value: &str) -> bool {
    address_string(address) == value
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, U256};
    use tx_processor::tx_processor::data_models::{OwnershipTransferredEvent, TradingEnabledEvent};

    fn tx(from: Address) -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            from,
            None,
            U256::ZERO,
            true,
            7,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn authority_ingest_extracts_owner_labels_and_edges() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let owner = address!("2222222222222222222222222222222222222222");
        let mut tx = tx(address!("1111111111111111111111111111111111111111"));
        tx.ownership_transferred_events
            .push(OwnershipTransferredEvent {
                contract_address: token,
                previous_owner: Address::ZERO,
                new_owner: owner,
                log_index: 4,
            });

        let batch = extract_authority_updates(&tx, address_string(&token));

        assert_eq!(batch.node_labels.len(), 1);
        assert_eq!(batch.node_labels[0].label.kind, NetworkLabelKind::Owner);
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, NetworkEdgeKind::ControlRelation);
    }

    #[test]
    fn authority_ingest_extracts_trading_policy_edges() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut tx = tx(address!("1111111111111111111111111111111111111111"));
        tx.trading_enabled_events.push(TradingEnabledEvent {
            token_address: token,
            block_number: 100,
            log_index: 5,
        });

        let batch = extract_authority_updates(&tx, address_string(&token));

        assert_eq!(batch.edges.len(), 1);
        assert_eq!(
            batch.edges[0]
                .evidence
                .examples
                .first()
                .and_then(|evidence| evidence.attributes.get("event"))
                .map(String::as_str),
            Some("trading_enabled")
        );
    }
}
