use std::collections::HashSet;

use alloy_primitives::{keccak256, Address, B256};
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::utils::append_with_history_limit;

const CONTROL_ROLE_NAMES: &[&str] = &[
    "DEFAULT_ADMIN_ROLE",
    "ADMIN_ROLE",
    "CONTROLLER_ROLE",
    "OWNER_ROLE",
    "MANAGER_ROLE",
    "PAUSER_ROLE",
    "GOVERNANCE_ROLE",
    "TRADING_ROLE",
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwnerEventRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub tx_index: u64,
    pub log_index: u64,
    pub previous_owner: String,
    pub new_owner: String,
    pub token_address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenouncementState {
    pub ownership_renounced: bool,
    pub renouncement_block: Option<u64>,
    pub renouncement_tx: Option<String>,
    pub renouncement_event: Option<OwnerEventRecord>,
    pub renouncement_event_index: Option<u64>,
    pub renouncement_event_log_index: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlAddressTracker {
    pub token_control_addresses: HashSet<String>,
    pub role_signatures: HashSet<B256>,
    pub history_limit: usize,
    pub all_owners: Vec<Option<String>>,
    pub owner_events: Vec<OwnerEventRecord>,
    pub current_owner: Option<String>,
    pub ownership_renounced: bool,
    pub renouncement_block: Option<u64>,
    pub renouncement_tx: Option<String>,
    pub renouncement_event: Option<OwnerEventRecord>,
    pub renouncement_event_index: Option<u64>,
    pub renouncement_event_log_index: Option<u64>,
}

impl ControlAddressTracker {
    pub fn new(history_limit: usize) -> Self {
        Self {
            token_control_addresses: HashSet::new(),
            role_signatures: default_role_signatures(),
            history_limit,
            all_owners: Vec::new(),
            owner_events: Vec::new(),
            current_owner: None,
            ownership_renounced: false,
            renouncement_block: None,
            renouncement_tx: None,
            renouncement_event: None,
            renouncement_event_index: None,
            renouncement_event_log_index: None,
        }
    }

    pub fn with_initial_addresses(
        history_limit: usize,
        addresses: impl IntoIterator<Item = Address>,
    ) -> Self {
        let mut tracker = Self::new(history_limit);
        tracker.register(addresses);
        tracker
    }

    pub fn addresses(&self) -> HashSet<String> {
        self.token_control_addresses.clone()
    }

    pub fn register(&mut self, addresses: impl IntoIterator<Item = Address>) -> Vec<String> {
        let mut added = Vec::new();
        for address in addresses {
            if address == Address::ZERO {
                continue;
            }
            let normalized = address_string(&address);
            if self.token_control_addresses.insert(normalized.clone()) {
                added.push(normalized);
            }
        }
        added
    }

    pub fn update_from_processed_transaction(&mut self, tx: &ProcessedTransaction) -> Vec<String> {
        let mut added = Vec::new();
        added.extend(self.handle_owner_events(tx));
        added.extend(self.handle_pending_owner_events(tx));
        added.extend(self.handle_access_control_role_events(tx));
        added.extend(self.handle_proxy_admin_events(tx));
        added
    }

    pub fn get_owner_history(&self) -> Vec<Option<String>> {
        self.all_owners.clone()
    }

    pub fn get_owner_events(&self) -> Vec<OwnerEventRecord> {
        self.owner_events.clone()
    }

    pub fn renouncement_state(&self) -> RenouncementState {
        RenouncementState {
            ownership_renounced: self.ownership_renounced,
            renouncement_block: self.renouncement_block,
            renouncement_tx: self.renouncement_tx.clone(),
            renouncement_event: self.renouncement_event.clone(),
            renouncement_event_index: self.renouncement_event_index,
            renouncement_event_log_index: self.renouncement_event_log_index,
        }
    }

    fn handle_owner_events(&mut self, tx: &ProcessedTransaction) -> Vec<String> {
        let mut added = Vec::new();
        let tx_hash = hash_string(&tx.hash);
        for event in &tx.ownership_transferred_events {
            added.extend(self.register([event.new_owner]));
            let new_owner = address_string(&event.new_owner);
            let previous_owner = address_string(&event.previous_owner);
            let record = OwnerEventRecord {
                tx_hash: tx_hash.clone(),
                block_number: tx.block_number,
                tx_index: tx.tx_index,
                log_index: event.log_index,
                previous_owner,
                new_owner: new_owner.clone(),
                token_address: address_string(&event.contract_address),
            };

            append_with_history_limit(
                &mut self.all_owners,
                Some(new_owner.clone()),
                self.history_limit,
            );
            self.current_owner = Some(new_owner);
            append_with_history_limit(&mut self.owner_events, record.clone(), self.history_limit);

            if event.new_owner == Address::ZERO {
                self.mark_renounced(tx, &record);
            }
        }
        added
    }

    fn handle_pending_owner_events(&mut self, tx: &ProcessedTransaction) -> Vec<String> {
        self.register(
            tx.ownership_transfer_started_events
                .iter()
                .map(|event| event.new_owner),
        )
    }

    fn handle_access_control_role_events(&mut self, tx: &ProcessedTransaction) -> Vec<String> {
        let accounts: Vec<_> = tx
            .access_control_role_granted_events
            .iter()
            .filter(|event| self.role_grants_control(event.role))
            .map(|event| event.account)
            .collect();
        self.register(accounts)
    }

    fn handle_proxy_admin_events(&mut self, tx: &ProcessedTransaction) -> Vec<String> {
        self.register(
            tx.proxy_admin_changed_events
                .iter()
                .map(|event| event.new_admin),
        )
    }

    fn mark_renounced(&mut self, tx: &ProcessedTransaction, event: &OwnerEventRecord) {
        self.ownership_renounced = true;
        self.renouncement_block = Some(tx.block_number);
        self.renouncement_tx = Some(hash_string(&tx.hash));
        self.renouncement_event = Some(event.clone());
        self.renouncement_event_index = Some(tx.tx_index);
        self.renouncement_event_log_index = Some(event.log_index);
    }

    fn role_grants_control(&self, role: B256) -> bool {
        self.role_signatures.contains(&role)
    }
}

fn default_role_signatures() -> HashSet<B256> {
    let mut signatures = HashSet::from([B256::ZERO]);
    for role in CONTROL_ROLE_NAMES {
        signatures.insert(keccak256(role.as_bytes()));
    }
    signatures
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use tx_processor::tx_processor::data_models::{
        AccessControlRoleGrantedEvent, OwnershipTransferStartedEvent, OwnershipTransferredEvent,
        ProxyAdminChangedEvent,
    };

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            2,
            address!("1111111111111111111111111111111111111111"),
            None,
            alloy_primitives::U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn tracks_owner_pending_role_and_proxy_admins() {
        let mut tx = tx();
        tx.ownership_transferred_events
            .push(OwnershipTransferredEvent {
                contract_address: address!("9999999999999999999999999999999999999999"),
                previous_owner: address!("1111111111111111111111111111111111111111"),
                new_owner: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });
        tx.ownership_transfer_started_events
            .push(OwnershipTransferStartedEvent {
                contract_address: address!("9999999999999999999999999999999999999999"),
                previous_owner: address!("2222222222222222222222222222222222222222"),
                new_owner: address!("3333333333333333333333333333333333333333"),
                log_index: 2,
            });
        tx.access_control_role_granted_events
            .push(AccessControlRoleGrantedEvent {
                contract_address: address!("9999999999999999999999999999999999999999"),
                role: keccak256("PAUSER_ROLE".as_bytes()),
                account: address!("4444444444444444444444444444444444444444"),
                sender: address!("2222222222222222222222222222222222222222"),
                log_index: 3,
            });
        tx.proxy_admin_changed_events.push(ProxyAdminChangedEvent {
            contract_address: address!("9999999999999999999999999999999999999999"),
            previous_admin: address!("2222222222222222222222222222222222222222"),
            new_admin: address!("5555555555555555555555555555555555555555"),
            log_index: 4,
        });

        let mut tracker = ControlAddressTracker::new(10);
        let added = tracker.update_from_processed_transaction(&tx);

        assert_eq!(added.len(), 4);
        assert_eq!(
            tracker.current_owner.as_deref(),
            Some("0x2222222222222222222222222222222222222222")
        );
        assert!(tracker
            .addresses()
            .contains("0x5555555555555555555555555555555555555555"));
        assert_eq!(tracker.owner_events.len(), 1);
    }

    #[test]
    fn marks_ownership_renounced_when_new_owner_is_zero() {
        let mut tx = tx();
        tx.ownership_transferred_events
            .push(OwnershipTransferredEvent {
                contract_address: address!("9999999999999999999999999999999999999999"),
                previous_owner: address!("1111111111111111111111111111111111111111"),
                new_owner: Address::ZERO,
                log_index: 8,
            });

        let mut tracker = ControlAddressTracker::new(10);
        tracker.update_from_processed_transaction(&tx);

        assert!(tracker.ownership_renounced);
        assert_eq!(tracker.renouncement_block, Some(100));
        assert_eq!(tracker.renouncement_event_log_index, Some(8));
    }
}
