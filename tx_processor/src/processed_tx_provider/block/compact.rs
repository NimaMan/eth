use std::collections::{HashMap, HashSet};

use alloy_eips::eip7702::SignedAuthorization;
use alloy_primitives::Bytes;
use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tx_simulator::types::StructLog;

use reth_chain_query::provider::{TransactionData, TransactionReceipt};

use crate::tx_processor::data_models::tx_models::ETHTransfer;
use crate::tx_processor::data_models::{
    AccessControlRoleGrantedEvent, AccessControlRoleRevokedEvent, AddressBalanceChange,
    ApprovalForAllEvent, ContractCreationEvent, DepositEvent, ERC1155TransferEvent,
    ERC20ApprovalEvent, ERC20TransferEvent, ERC721ApprovalEvent, ERC721TransferEvent,
    InternalErc20Call, InternalErc20Transfer, InternalTransaction, OwnershipTransferStartedEvent,
    OwnershipTransferredEvent, Permit2Event,
    ProcessedAccessListItem, ProcessedTransaction, ProxyAdminChangedEvent, TradingDisabledEvent,
    TradingEnabledEvent, TransactionFees, UniswapV2BurnEvent, UniswapV2MintEvent,
    UniswapV2PairCreatedEvent, UniswapV2SwapEvent, UniswapV2SyncEvent, UniswapV3BurnEvent,
    UniswapV3DecreaseLiquidityEvent, UniswapV3IncreaseLiquidityEvent, UniswapV3InitializeEvent,
    UniswapV3MintEvent, UniswapV3PoolCreatedEvent, UniswapV3PositionEvent, UniswapV3SwapEvent,
    UniswapV4BalanceDeltaEvent, UniswapV4DonateEvent, UniswapV4DynamicLPFeeUpdatedEvent,
    UniswapV4FeeControllerUpdatedEvent, UniswapV4FeeUpdatedEvent, UniswapV4InitializeEvent,
    UniswapV4ModifyLiquidityEvent, UniswapV4SwapEvent, WithdrawEvent,
};
use crate::ProcessedBlockTransactions;

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
    pub tx_type: Option<String>,
    pub actions: Option<Vec<String>>,
    pub fees: TransactionFees,
    pub bribe_amount: Option<U256>,
    pub unique_addresses: Option<Vec<Address>>,
    pub erc20_contracts: Option<Vec<Address>>,
    pub erc721_contracts: Option<Vec<Address>>,
    pub erc1155_contracts: Option<Vec<Address>>,
    pub eth_transfers: Option<Vec<ETHTransfer>>,
    pub erc20_transfers: Option<Vec<ERC20TransferEvent>>,
    pub erc721_transfers: Option<Vec<ERC721TransferEvent>>,
    pub erc1155_transfers: Option<Vec<ERC1155TransferEvent>>,
    pub internal_transactions: Option<Vec<InternalTransaction>>,
    #[serde(default)]
    pub internal_erc20_calls: Option<Vec<InternalErc20Call>>,
    #[serde(default)]
    pub internal_erc20_transfers: Option<Vec<InternalErc20Transfer>>,
    pub uniswap_v2_syncs: Option<Vec<UniswapV2SyncEvent>>,
    pub uniswap_v2_swaps: Option<Vec<UniswapV2SwapEvent>>,
    pub uniswap_v3_pools: Option<Vec<UniswapV3PoolCreatedEvent>>,
    pub uniswap_v3_initializations: Option<Vec<UniswapV3InitializeEvent>>,
    pub uniswap_v3_burns: Option<Vec<UniswapV3BurnEvent>>,
    pub uniswap_v3_mints: Option<Vec<UniswapV3MintEvent>>,
    pub uniswap_v3_swaps: Option<Vec<UniswapV3SwapEvent>>,
    pub uniswap_v3_positions: Option<Vec<UniswapV3PositionEvent>>,
    pub uniswap_v3_increases: Option<Vec<UniswapV3IncreaseLiquidityEvent>>,
    pub uniswap_v3_decreases: Option<Vec<UniswapV3DecreaseLiquidityEvent>>,
    pub uniswap_v4_initializes: Option<Vec<UniswapV4InitializeEvent>>,
    pub uniswap_v4_modifies: Option<Vec<UniswapV4ModifyLiquidityEvent>>,
    pub uniswap_v4_swaps: Option<Vec<UniswapV4SwapEvent>>,
    pub uniswap_v4_donates: Option<Vec<UniswapV4DonateEvent>>,
    pub uniswap_v4_protocol_fee_updates: Option<Vec<UniswapV4FeeUpdatedEvent>>,
    pub uniswap_v4_dynamic_lp_fee_updates: Option<Vec<UniswapV4DynamicLPFeeUpdatedEvent>>,
    pub uniswap_v4_protocol_fee_controller_updates: Option<Vec<UniswapV4FeeControllerUpdatedEvent>>,
    pub uniswap_v4_balance_deltas: Option<Vec<UniswapV4BalanceDeltaEvent>>,
    pub permit2_events: Option<Vec<Permit2Event>>,
    pub access_list: Option<Vec<ProcessedAccessListItem>>,
    pub blob_versioned_hashes: Option<Vec<B256>>,
    pub signed_authorizations: Option<Vec<SignedAuthorization>>,
    pub erc20_approval_events: Option<Vec<ERC20ApprovalEvent>>,
    pub erc721_approval_events: Option<Vec<ERC721ApprovalEvent>>,
    pub approval_for_all_events: Option<Vec<ApprovalForAllEvent>>,
    pub uniswap_v2_mints: Option<Vec<UniswapV2MintEvent>>,
    pub uniswap_v2_burns: Option<Vec<UniswapV2BurnEvent>>,
    pub deposit_events: Option<Vec<DepositEvent>>,
    pub withdraw_events: Option<Vec<WithdrawEvent>>,
    pub uniswap_v2_pair_created_events: Option<Vec<UniswapV2PairCreatedEvent>>,
    pub ownership_transferred_events: Option<Vec<OwnershipTransferredEvent>>,
    pub ownership_transfer_started_events: Option<Vec<OwnershipTransferStartedEvent>>,
    pub access_control_role_granted_events: Option<Vec<AccessControlRoleGrantedEvent>>,
    pub access_control_role_revoked_events: Option<Vec<AccessControlRoleRevokedEvent>>,
    pub proxy_admin_changed_events: Option<Vec<ProxyAdminChangedEvent>>,
    pub contract_creation_events: Option<Vec<ContractCreationEvent>>,
    pub trading_enabled_events: Option<Vec<TradingEnabledEvent>>,
    pub trading_disabled_events: Option<Vec<TradingDisabledEvent>>,
    #[serde(
        default,
        with = "crate::tx_processor::data_models::serde_helpers::option_json_value_vec_map_binary"
    )]
    pub other_events: Option<Vec<HashMap<String, Value>>>,
    pub address_balance_changes: Option<HashMap<Address, AddressBalanceChange>>,
    #[serde(
        default,
        with = "crate::tx_processor::data_models::serde_helpers::option_json_value_address_map_binary"
    )]
    pub latest_states: Option<HashMap<Address, Value>>,
    #[serde(
        default,
        with = "crate::tx_processor::data_models::serde_helpers::option_bytes_hex"
    )]
    pub input: Option<Vec<u8>>,
    pub struct_logs: Option<Vec<StructLog>>,
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
            tx_type: option_string(&tx.tx_type),
            actions: option_vec(&tx.actions),
            fees: tx.fees.clone(),
            bribe_amount: option_nonzero_u256(tx.bribe_amount),
            unique_addresses: option_address_set(&tx.unique_addresses),
            erc20_contracts: option_address_set(&tx.erc20_contracts),
            erc721_contracts: option_address_set(&tx.erc721_contracts),
            erc1155_contracts: option_address_set(&tx.erc1155_contracts),
            eth_transfers: option_vec(&tx.eth_transfers),
            erc20_transfers: option_vec(&tx.erc20_transfers),
            erc721_transfers: option_vec(&tx.erc721_transfers),
            erc1155_transfers: option_vec(&tx.erc1155_transfers),
            internal_transactions: option_vec(&tx.internal_transactions),
            internal_erc20_calls: option_vec(&tx.internal_erc20_calls),
            internal_erc20_transfers: option_vec(&tx.internal_erc20_transfers),
            uniswap_v2_syncs: option_vec(&tx.uniswap_v2_syncs),
            uniswap_v2_swaps: option_vec(&tx.uniswap_v2_swaps),
            uniswap_v3_pools: option_vec(&tx.uniswap_v3_pools),
            uniswap_v3_initializations: option_vec(&tx.uniswap_v3_initializations),
            uniswap_v3_burns: option_vec(&tx.uniswap_v3_burns),
            uniswap_v3_mints: option_vec(&tx.uniswap_v3_mints),
            uniswap_v3_swaps: option_vec(&tx.uniswap_v3_swaps),
            uniswap_v3_positions: option_vec(&tx.uniswap_v3_positions),
            uniswap_v3_increases: option_vec(&tx.uniswap_v3_increases),
            uniswap_v3_decreases: option_vec(&tx.uniswap_v3_decreases),
            uniswap_v4_initializes: option_vec(&tx.uniswap_v4_initializes),
            uniswap_v4_modifies: option_vec(&tx.uniswap_v4_modifies),
            uniswap_v4_swaps: option_vec(&tx.uniswap_v4_swaps),
            uniswap_v4_donates: option_vec(&tx.uniswap_v4_donates),
            uniswap_v4_protocol_fee_updates: option_vec(&tx.uniswap_v4_protocol_fee_updates),
            uniswap_v4_dynamic_lp_fee_updates: option_vec(&tx.uniswap_v4_dynamic_lp_fee_updates),
            uniswap_v4_protocol_fee_controller_updates: option_vec(
                &tx.uniswap_v4_protocol_fee_controller_updates,
            ),
            uniswap_v4_balance_deltas: option_vec(&tx.uniswap_v4_balance_deltas),
            permit2_events: option_vec(&tx.permit2_events),
            access_list: option_vec(&tx.access_list),
            blob_versioned_hashes: option_vec(&tx.blob_versioned_hashes),
            signed_authorizations: option_vec(&tx.signed_authorizations),
            erc20_approval_events: option_vec(&tx.erc20_approval_events),
            erc721_approval_events: option_vec(&tx.erc721_approval_events),
            approval_for_all_events: option_vec(&tx.approval_for_all_events),
            uniswap_v2_mints: option_vec(&tx.uniswap_v2_mints),
            uniswap_v2_burns: option_vec(&tx.uniswap_v2_burns),
            deposit_events: option_vec(&tx.deposit_events),
            withdraw_events: option_vec(&tx.withdraw_events),
            uniswap_v2_pair_created_events: option_vec(&tx.uniswap_v2_pair_created_events),
            ownership_transferred_events: option_vec(&tx.ownership_transferred_events),
            ownership_transfer_started_events: option_vec(&tx.ownership_transfer_started_events),
            access_control_role_granted_events: option_vec(&tx.access_control_role_granted_events),
            access_control_role_revoked_events: option_vec(&tx.access_control_role_revoked_events),
            proxy_admin_changed_events: option_vec(&tx.proxy_admin_changed_events),
            contract_creation_events: option_vec(&tx.contract_creation_events),
            trading_enabled_events: option_vec(&tx.trading_enabled_events),
            trading_disabled_events: option_vec(&tx.trading_disabled_events),
            other_events: option_vec(&tx.other_events),
            address_balance_changes: option_map(&tx.address_balance_changes),
            latest_states: option_map(&tx.latest_states),
            input: option_vec(&tx.input),
            struct_logs: option_optional_vec(&tx.struct_logs),
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
        tx.tx_type = self.tx_type.unwrap_or_default();
        tx.actions = self.actions.unwrap_or_default();
        tx.bribe_amount = self.bribe_amount.unwrap_or_default();
        tx.unique_addresses = option_address_vec_into_set(self.unique_addresses);
        tx.erc20_contracts = option_address_vec_into_set(self.erc20_contracts);
        tx.erc721_contracts = option_address_vec_into_set(self.erc721_contracts);
        tx.erc1155_contracts = option_address_vec_into_set(self.erc1155_contracts);
        tx.eth_transfers = self.eth_transfers.unwrap_or_default();
        tx.erc20_transfers = self.erc20_transfers.unwrap_or_default();
        tx.erc721_transfers = self.erc721_transfers.unwrap_or_default();
        tx.erc1155_transfers = self.erc1155_transfers.unwrap_or_default();
        tx.internal_transactions = self.internal_transactions.unwrap_or_default();
        tx.internal_erc20_calls = self.internal_erc20_calls.unwrap_or_default();
        tx.internal_erc20_transfers = self.internal_erc20_transfers.unwrap_or_default();
        tx.uniswap_v2_syncs = self.uniswap_v2_syncs.unwrap_or_default();
        tx.uniswap_v2_swaps = self.uniswap_v2_swaps.unwrap_or_default();
        tx.uniswap_v3_pools = self.uniswap_v3_pools.unwrap_or_default();
        tx.uniswap_v3_initializations = self.uniswap_v3_initializations.unwrap_or_default();
        tx.uniswap_v3_burns = self.uniswap_v3_burns.unwrap_or_default();
        tx.uniswap_v3_mints = self.uniswap_v3_mints.unwrap_or_default();
        tx.uniswap_v3_swaps = self.uniswap_v3_swaps.unwrap_or_default();
        tx.uniswap_v3_positions = self.uniswap_v3_positions.unwrap_or_default();
        tx.uniswap_v3_increases = self.uniswap_v3_increases.unwrap_or_default();
        tx.uniswap_v3_decreases = self.uniswap_v3_decreases.unwrap_or_default();
        tx.uniswap_v4_initializes = self.uniswap_v4_initializes.unwrap_or_default();
        tx.uniswap_v4_modifies = self.uniswap_v4_modifies.unwrap_or_default();
        tx.uniswap_v4_swaps = self.uniswap_v4_swaps.unwrap_or_default();
        tx.uniswap_v4_donates = self.uniswap_v4_donates.unwrap_or_default();
        tx.uniswap_v4_protocol_fee_updates =
            self.uniswap_v4_protocol_fee_updates.unwrap_or_default();
        tx.uniswap_v4_dynamic_lp_fee_updates =
            self.uniswap_v4_dynamic_lp_fee_updates.unwrap_or_default();
        tx.uniswap_v4_protocol_fee_controller_updates = self
            .uniswap_v4_protocol_fee_controller_updates
            .unwrap_or_default();
        tx.uniswap_v4_balance_deltas = self.uniswap_v4_balance_deltas.unwrap_or_default();
        tx.permit2_events = self.permit2_events.unwrap_or_default();
        tx.access_list = self.access_list.unwrap_or_default();
        tx.blob_versioned_hashes = self.blob_versioned_hashes.unwrap_or_default();
        tx.signed_authorizations = self.signed_authorizations.unwrap_or_default();
        tx.erc20_approval_events = self.erc20_approval_events.unwrap_or_default();
        tx.erc721_approval_events = self.erc721_approval_events.unwrap_or_default();
        tx.approval_for_all_events = self.approval_for_all_events.unwrap_or_default();
        tx.uniswap_v2_mints = self.uniswap_v2_mints.unwrap_or_default();
        tx.uniswap_v2_burns = self.uniswap_v2_burns.unwrap_or_default();
        tx.deposit_events = self.deposit_events.unwrap_or_default();
        tx.withdraw_events = self.withdraw_events.unwrap_or_default();
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
        tx.other_events = self.other_events.unwrap_or_default();
        tx.address_balance_changes = self.address_balance_changes.unwrap_or_default();
        tx.latest_states = self.latest_states.unwrap_or_default();
        tx.input = self.input.unwrap_or_default();
        tx.struct_logs = self.struct_logs;

        tx
    }

    pub fn to_sparse_json_value(&self) -> serde_json::Result<Value> {
        let mut value = serde_json::to_value(self)?;
        prune_empty_json_fields(&mut value);
        Ok(value)
    }

    pub(crate) fn into_block_transaction(
        self,
        processing_error: Option<String>,
    ) -> ProcessedBlockTransactions {
        block_transaction_from_processed(self.into_processed(), processing_error)
    }
}

pub(crate) fn block_transaction_from_processed(
    processed: ProcessedTransaction,
    processing_error: Option<String>,
) -> ProcessedBlockTransactions {
    let metadata = metadata_from_processed_transaction(&processed);
    let receipt = receipt_from_processed_transaction(&processed);
    ProcessedBlockTransactions {
        metadata,
        receipt,
        processed,
        trace: None,
        processing_error,
    }
}

fn option_vec<T: Clone>(items: &[T]) -> Option<Vec<T>> {
    if items.is_empty() {
        None
    } else {
        Some(items.to_vec())
    }
}

fn option_optional_vec<T: Clone>(items: &Option<Vec<T>>) -> Option<Vec<T>> {
    items.as_ref().and_then(|items| option_vec(items))
}

fn option_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
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

fn option_map<K, V>(items: &HashMap<K, V>) -> Option<HashMap<K, V>>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    if items.is_empty() {
        None
    } else {
        Some(items.clone())
    }
}

fn option_nonzero_u256(value: U256) -> Option<U256> {
    if value.is_zero() {
        None
    } else {
        Some(value)
    }
}

fn prune_empty_json_fields(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for nested in object.values_mut() {
                prune_empty_json_fields(nested);
            }
            object.retain(|_, nested| !is_empty_json_field(nested));
        }
        Value::Array(items) => {
            for nested in items {
                prune_empty_json_fields(nested);
            }
        }
        _ => {}
    }
}

fn is_empty_json_field(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        _ => false,
    }
}

fn metadata_from_processed_transaction(tx: &ProcessedTransaction) -> TransactionData {
    TransactionData {
        hash: tx.hash,
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_index: tx.tx_index,
        tx_number: 0,
        from: tx.from_address,
        to: tx.to_address,
        value: tx.value,
        input: Bytes::from(tx.input.clone()),
        gas_price: U256::ZERO,
        gas_limit: tx.fees.gas_limit,
        nonce: tx.nonce,
        transaction_type: tx.raw_tx_type,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        access_list: Vec::new(),
        blob_versioned_hashes: Vec::new(),
        max_fee_per_blob_gas: None,
        signed_authorizations: Vec::new(),
    }
}

fn receipt_from_processed_transaction(tx: &ProcessedTransaction) -> TransactionReceipt {
    TransactionReceipt {
        tx_hash: tx.hash,
        status: tx.status,
        gas_used: 0,
        logs: Vec::new(),
        cumulative_gas_used: 0,
        effective_gas_price: U256::ZERO,
        contract_address: tx.contract_address,
        blob_gas_used: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::I256;
    use serde_json::json;

    use crate::tx_processor::data_models::{
        AddressBalanceChange, ApprovalForAllEvent, ERC1155TransferEvent, ERC20ApprovalEvent,
        ERC20TransferEvent, ERC721ApprovalEvent, ERC721TransferEvent, ProcessedAccessListItem,
        TokenMovement, TokenMovements, UniswapV3PoolCreatedEvent, UniswapV4InitializeEvent,
    };

    #[test]
    fn sparse_json_omits_empty_compact_fields() {
        let tx = ProcessedTransaction::new(
            B256::ZERO,
            1,
            1_700_000_000,
            0,
            Address::ZERO,
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );

        let value = CompactProcessedTransaction::from_processed(&tx)
            .to_sparse_json_value()
            .expect("serialize compact tx");
        let object = value.as_object().expect("compact tx object");

        assert!(!object.contains_key("input"));
        assert!(!object.contains_key("actions"));
        assert!(!object.contains_key("unique_addresses"));
        assert!(!object.contains_key("bribe_amount"));
        assert_no_empty_json_fields(&value);
    }

    #[test]
    fn sparse_json_decodes_access_list_items_with_pruned_storage_keys() {
        let mut tx = ProcessedTransaction::new(
            B256::repeat_byte(0x01),
            1,
            1_700_000_000,
            0,
            Address::repeat_byte(0x02),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );
        let access_list_address = Address::repeat_byte(0x03);
        tx.access_list.push(ProcessedAccessListItem {
            address: access_list_address,
            storage_keys: Vec::new(),
        });

        let value = CompactProcessedTransaction::from_processed(&tx)
            .to_sparse_json_value()
            .expect("serialize compact tx");
        let access_list = value
            .get("access_list")
            .and_then(Value::as_array)
            .expect("access list");
        let item = access_list[0].as_object().expect("access list item");
        assert!(item.contains_key("address"));
        assert!(!item.contains_key("storage_keys"));

        let decoded: CompactProcessedTransaction =
            serde_json::from_value(value).expect("decode compact tx");
        let processed = decoded.into_processed();
        assert_eq!(processed.access_list.len(), 1);
        assert_eq!(processed.access_list[0].address, access_list_address);
        assert!(processed.access_list[0].storage_keys.is_empty());
    }

    #[test]
    fn compact_transaction_round_trips_rich_runtime_fields() {
        let owner = Address::repeat_byte(0x11);
        let token = Address::repeat_byte(0x22);
        let pool = Address::repeat_byte(0x33);
        let recipient = Address::repeat_byte(0x44);
        let mut tx = ProcessedTransaction::new(
            B256::repeat_byte(0xaa),
            42,
            1_700_000_000,
            7,
            owner,
            Some(pool),
            U256::from(123),
            true,
            9,
            2,
            vec![0xde, 0xad, 0xbe, 0xef],
        );

        tx.tx_type = "swap".to_string();
        tx.actions = vec!["swap".to_string(), "token_tracking".to_string()];
        tx.bribe_amount = U256::from(5);
        tx.unique_addresses.extend([owner, token, pool, recipient]);
        tx.erc20_contracts.insert(token);
        tx.erc721_contracts.insert(Address::repeat_byte(0x55));
        tx.erc1155_contracts.insert(Address::repeat_byte(0x66));
        tx.eth_transfers.push(ETHTransfer {
            from_address: owner,
            to_address: recipient,
            amount: U256::from(1_000),
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: token,
            from_address: owner,
            to_address: pool,
            amount: U256::from(2_000),
            log_index: 1,
        });
        tx.erc721_transfers.push(ERC721TransferEvent {
            token_address: Address::repeat_byte(0x55),
            from_address: owner,
            to_address: recipient,
            token_id: U256::from(77),
            log_index: 2,
        });
        tx.erc1155_transfers.push(ERC1155TransferEvent {
            token_address: Address::repeat_byte(0x66),
            operator: owner,
            from_address: owner,
            to_address: recipient,
            token_ids: vec![U256::from(1)],
            amounts: vec![U256::from(10)],
            log_index: 3,
        });
        tx.erc20_approval_events.push(ERC20ApprovalEvent {
            token_address: token,
            owner,
            spender: pool,
            amount: U256::MAX,
            log_index: 4,
        });
        tx.erc721_approval_events.push(ERC721ApprovalEvent {
            token_address: Address::repeat_byte(0x55),
            owner,
            approved_address: pool,
            token_id: U256::from(77),
            log_index: 5,
        });
        tx.approval_for_all_events.push(ApprovalForAllEvent {
            token_address: Address::repeat_byte(0x66),
            owner,
            operator: pool,
            approved: true,
            log_index: 6,
        });
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: pool,
                token0: token,
                token1: Address::ZERO,
                factory_address: Address::repeat_byte(0x77),
                log_index: 7,
            });
        tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
            factory_address: Address::repeat_byte(0x78),
            token0: token,
            token1: Address::ZERO,
            fee: 3_000,
            tick_spacing: 60,
            pool,
            log_index: 8,
        });
        tx.uniswap_v4_initializes.push(UniswapV4InitializeEvent {
            pool_manager_address: Address::repeat_byte(0x88),
            event_id: B256::repeat_byte(0xbb),
            currency0: token,
            currency1: Address::ZERO,
            fee: 3_000,
            tick_spacing: 60,
            hooks: Address::ZERO,
            sqrt_price_x96: U256::from(1),
            tick: 0,
            log_index: 9,
        });
        tx.permit2_events.push(Permit2Event {
            pool_manager_address: Address::repeat_byte(0x88),
            owner,
            token,
            spender: pool,
            amount: U256::from(100),
            expiration: 1_800_000_000,
            nonce: 1,
            log_index: 10,
        });
        tx.other_events.push(HashMap::from([(
            "event".to_string(),
            json!({"kind": "debug", "value": 1}),
        )]));

        let mut balance = AddressBalanceChange::default();
        balance.token_net.insert(
            reth_chain_query::to_checksum_address(&token),
            I256::try_from(U256::from(5)).unwrap(),
        );
        balance
            .currency_net
            .insert("ETH".to_string(), I256::try_from(U256::from(10)).unwrap());
        balance.movements = TokenMovements {
            tokens: HashMap::from([(
                reth_chain_query::to_checksum_address(&token),
                TokenMovement {
                    incoming: HashMap::from([("in-1".to_string(), U256::from(5))]),
                    outgoing: HashMap::new(),
                },
            )]),
            currencies: HashMap::new(),
        };
        tx.address_balance_changes.insert(owner, balance);
        tx.latest_states.insert(pool, json!({"reserve0": "1"}));
        tx.internal_erc20_calls.push(InternalErc20Call {
            token_address: token,
            caller: owner,
            kind: crate::tx_processor::data_models::Erc20CallKind::TransferFrom,
            from_address: pool,
            to_address: recipient,
            amount: U256::from(4_242),
            depth: 2,
            call_type: Some("Call".to_string()),
            succeeded: true,
        });

        let compact = CompactProcessedTransaction::from_processed(&tx);
        let json_value = compact.to_sparse_json_value().expect("compact json");
        assert_no_empty_json_fields(&json_value);
        let json_roundtrip: CompactProcessedTransaction =
            serde_json::from_value(json_value).expect("decode compact json");
        let binary = bincode::serialize(&json_roundtrip).expect("encode compact bincode");
        let binary_roundtrip: CompactProcessedTransaction =
            bincode::deserialize(&binary).expect("decode compact bincode");
        let decoded = binary_roundtrip.into_processed();

        assert_eq!(decoded.tx_type, "swap");
        assert_eq!(decoded.actions, vec!["swap", "token_tracking"]);
        assert_eq!(decoded.input, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(decoded.bribe_amount, U256::from(5));
        assert_eq!(decoded.internal_erc20_calls.len(), 1);
        assert_eq!(decoded.internal_erc20_calls[0].amount, U256::from(4_242));
        assert_eq!(decoded.internal_erc20_calls[0].depth, 2);
        assert!(decoded.unique_addresses.contains(&recipient));
        assert_eq!(decoded.erc20_transfers.len(), 1);
        assert_eq!(decoded.erc721_transfers.len(), 1);
        assert_eq!(decoded.erc1155_transfers.len(), 1);
        assert_eq!(decoded.erc20_approval_events.len(), 1);
        assert_eq!(decoded.erc721_approval_events.len(), 1);
        assert_eq!(decoded.approval_for_all_events.len(), 1);
        assert_eq!(decoded.uniswap_v2_pair_created_events.len(), 1);
        assert_eq!(decoded.uniswap_v3_pools.len(), 1);
        assert_eq!(
            decoded.uniswap_v3_pools[0].factory_address,
            Address::repeat_byte(0x78)
        );
        assert_eq!(decoded.uniswap_v4_initializes.len(), 1);
        assert_eq!(decoded.permit2_events.len(), 1);
        assert_eq!(decoded.other_events[0]["event"]["kind"], "debug");
        assert_eq!(
            decoded
                .address_balance_changes
                .get(&owner)
                .and_then(|change| change.currency_net.get("ETH"))
                .copied(),
            Some(I256::try_from(U256::from(10)).unwrap())
        );
        assert_eq!(decoded.latest_states[&pool]["reserve0"], "1");
    }

    fn assert_no_empty_json_fields(value: &Value) {
        match value {
            Value::Null => panic!("compact JSON contains null"),
            Value::String(value) if value.is_empty() => {
                panic!("compact JSON contains empty string")
            }
            Value::Array(values) => {
                assert!(!values.is_empty(), "compact JSON contains empty array");
                for value in values {
                    assert_no_empty_json_fields(value);
                }
            }
            Value::Object(values) => {
                assert!(!values.is_empty(), "compact JSON contains empty object");
                for value in values.values() {
                    assert_no_empty_json_fields(value);
                }
            }
            _ => {}
        }
    }
}
