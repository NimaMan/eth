use std::collections::HashSet;

use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};

use crate::tx_processor::data_models::{
    AccessControlRoleGrantedEvent, AccessControlRoleRevokedEvent, ContractCreationEvent,
    ERC20ApprovalEvent, ERC20TransferEvent, InternalTransaction, OwnershipTransferStartedEvent,
    OwnershipTransferredEvent, ProcessedAccessListItem, ProcessedTransaction,
    ProxyAdminChangedEvent, TradingDisabledEvent, TradingEnabledEvent, TransactionFees,
    UniswapV2BurnEvent, UniswapV2MintEvent, UniswapV2PairCreatedEvent, UniswapV2SwapEvent,
    UniswapV2SyncEvent,
};

/// Compact processed-transaction representation for block provider storage.
///
/// The runtime type keeps empty vectors and zero values for simple processing.
/// This wire/storage type converts empty collections and zero-only optional
/// values to `None`, so persistent providers only carry fields with data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactProcessedTransaction {
    pub hash: B256,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub contract_address: Option<Address>,
    pub value: U256,
    pub status: bool,
    pub nonce: u64,
    pub raw_tx_type: u8,
    pub fees: TransactionFees,
    pub input: Option<Vec<u8>>,
    pub access_list: Option<Vec<ProcessedAccessListItem>>,
    pub blob_versioned_hashes: Option<Vec<B256>>,
    pub bribe_amount: Option<U256>,
    pub unique_addresses: Option<Vec<Address>>,
    pub erc20_contracts: Option<Vec<Address>>,
    pub internal_transactions: Option<Vec<InternalTransaction>>,
    pub erc20_transfers: Option<Vec<ERC20TransferEvent>>,
    pub erc20_approval_events: Option<Vec<ERC20ApprovalEvent>>,
    pub uniswap_v2_syncs: Option<Vec<UniswapV2SyncEvent>>,
    pub uniswap_v2_swaps: Option<Vec<UniswapV2SwapEvent>>,
    pub uniswap_v2_mints: Option<Vec<UniswapV2MintEvent>>,
    pub uniswap_v2_burns: Option<Vec<UniswapV2BurnEvent>>,
    pub uniswap_v2_pair_created_events: Option<Vec<UniswapV2PairCreatedEvent>>,
    pub ownership_transferred_events: Option<Vec<OwnershipTransferredEvent>>,
    pub ownership_transfer_started_events: Option<Vec<OwnershipTransferStartedEvent>>,
    pub access_control_role_granted_events: Option<Vec<AccessControlRoleGrantedEvent>>,
    pub access_control_role_revoked_events: Option<Vec<AccessControlRoleRevokedEvent>>,
    pub proxy_admin_changed_events: Option<Vec<ProxyAdminChangedEvent>>,
    pub contract_creation_events: Option<Vec<ContractCreationEvent>>,
    pub trading_enabled_events: Option<Vec<TradingEnabledEvent>>,
    pub trading_disabled_events: Option<Vec<TradingDisabledEvent>>,
}

impl CompactProcessedTransaction {
    pub fn from_processed(tx: &ProcessedTransaction) -> Self {
        Self {
            hash: tx.hash,
            block_number: tx.block_number,
            block_timestamp: tx.block_timestamp,
            tx_index: tx.tx_index,
            from_address: tx.from_address,
            to_address: tx.to_address,
            contract_address: tx.contract_address,
            value: tx.value,
            status: tx.status,
            nonce: tx.nonce,
            raw_tx_type: tx.raw_tx_type,
            fees: tx.fees.clone(),
            input: option_vec(&tx.input),
            access_list: option_vec(&tx.access_list),
            blob_versioned_hashes: option_vec(&tx.blob_versioned_hashes),
            bribe_amount: option_nonzero_u256(tx.bribe_amount),
            unique_addresses: option_address_set(&tx.unique_addresses),
            erc20_contracts: option_address_set(&tx.erc20_contracts),
            internal_transactions: option_vec(&tx.internal_transactions),
            erc20_transfers: option_vec(&tx.erc20_transfers),
            erc20_approval_events: option_vec(&tx.erc20_approval_events),
            uniswap_v2_syncs: option_vec(&tx.uniswap_v2_syncs),
            uniswap_v2_swaps: option_vec(&tx.uniswap_v2_swaps),
            uniswap_v2_mints: option_vec(&tx.uniswap_v2_mints),
            uniswap_v2_burns: option_vec(&tx.uniswap_v2_burns),
            uniswap_v2_pair_created_events: option_vec(&tx.uniswap_v2_pair_created_events),
            ownership_transferred_events: option_vec(&tx.ownership_transferred_events),
            ownership_transfer_started_events: option_vec(&tx.ownership_transfer_started_events),
            access_control_role_granted_events: option_vec(&tx.access_control_role_granted_events),
            access_control_role_revoked_events: option_vec(&tx.access_control_role_revoked_events),
            proxy_admin_changed_events: option_vec(&tx.proxy_admin_changed_events),
            contract_creation_events: option_vec(&tx.contract_creation_events),
            trading_enabled_events: option_vec(&tx.trading_enabled_events),
            trading_disabled_events: option_vec(&tx.trading_disabled_events),
        }
    }

    pub fn into_processed(self) -> ProcessedTransaction {
        let mut tx = ProcessedTransaction::new(
            self.hash,
            self.block_number,
            self.block_timestamp,
            self.tx_index,
            self.from_address,
            self.to_address,
            self.value,
            self.status,
            self.nonce,
            self.raw_tx_type,
            Vec::new(),
        );

        tx.contract_address = self.contract_address;
        tx.fees = self.fees;
        tx.input = self.input.unwrap_or_default();
        tx.access_list = self.access_list.unwrap_or_default();
        tx.blob_versioned_hashes = self.blob_versioned_hashes.unwrap_or_default();
        tx.bribe_amount = self.bribe_amount.unwrap_or_default();
        tx.unique_addresses = option_address_vec_into_set(self.unique_addresses);
        tx.erc20_contracts = option_address_vec_into_set(self.erc20_contracts);
        tx.internal_transactions = self.internal_transactions.unwrap_or_default();
        tx.erc20_transfers = self.erc20_transfers.unwrap_or_default();
        tx.erc20_approval_events = self.erc20_approval_events.unwrap_or_default();
        tx.uniswap_v2_syncs = self.uniswap_v2_syncs.unwrap_or_default();
        tx.uniswap_v2_swaps = self.uniswap_v2_swaps.unwrap_or_default();
        tx.uniswap_v2_mints = self.uniswap_v2_mints.unwrap_or_default();
        tx.uniswap_v2_burns = self.uniswap_v2_burns.unwrap_or_default();
        tx.uniswap_v2_pair_created_events = self.uniswap_v2_pair_created_events.unwrap_or_default();
        tx.ownership_transferred_events = self.ownership_transferred_events.unwrap_or_default();
        tx.ownership_transfer_started_events =
            self.ownership_transfer_started_events.unwrap_or_default();
        tx.access_control_role_granted_events =
            self.access_control_role_granted_events.unwrap_or_default();
        tx.access_control_role_revoked_events =
            self.access_control_role_revoked_events.unwrap_or_default();
        tx.proxy_admin_changed_events = self.proxy_admin_changed_events.unwrap_or_default();
        tx.contract_creation_events = self.contract_creation_events.unwrap_or_default();
        tx.trading_enabled_events = self.trading_enabled_events.unwrap_or_default();
        tx.trading_disabled_events = self.trading_disabled_events.unwrap_or_default();

        tx
    }
}

fn option_vec<T: Clone>(items: &[T]) -> Option<Vec<T>> {
    if items.is_empty() {
        None
    } else {
        Some(items.to_vec())
    }
}

fn option_address_set(items: &HashSet<Address>) -> Option<Vec<Address>> {
    if items.is_empty() {
        return None;
    }
    let mut items: Vec<_> = items.iter().copied().collect();
    items.sort_unstable();
    Some(items)
}

fn option_address_vec_into_set(items: Option<Vec<Address>>) -> HashSet<Address> {
    items.unwrap_or_default().into_iter().collect()
}

fn option_nonzero_u256(value: U256) -> Option<U256> {
    if value.is_zero() {
        None
    } else {
        Some(value)
    }
}
