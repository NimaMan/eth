use super::balance_changes::AddressBalanceChange;
use super::fees::TransactionFees;
use super::receipt_models::*;
use super::trace_models::{InternalErc20Call, InternalErc20Transfer, InternalTransaction};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_primitives::{Address, B256, I256, U256};
use reth_chain_query::to_checksum_address;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tx_simulator::types::StructLog;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETHTransfer {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCreationEvent {
    pub contract_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedAccessListItem {
    pub address: Address,
    #[serde(default)]
    pub storage_keys: Vec<B256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    // Core transaction data
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

    /// Raw Ethereum transaction type (0 = legacy, 1 = access list, 2 = EIP-1559, etc.)
    #[serde(default)]
    pub raw_tx_type: u8,

    // Transaction classification
    pub tx_type: String,
    pub actions: Vec<String>,

    // Fee information
    pub fees: TransactionFees,
    pub bribe_amount: U256,

    // Addresses and contracts involved
    pub unique_addresses: HashSet<Address>,
    pub erc20_contracts: HashSet<Address>,
    pub erc721_contracts: HashSet<Address>,
    pub erc1155_contracts: HashSet<Address>,

    // Transfer events
    pub eth_transfers: Vec<ETHTransfer>,
    pub erc20_transfers: Vec<ERC20TransferEvent>,
    pub erc721_transfers: Vec<ERC721TransferEvent>,
    pub erc1155_transfers: Vec<ERC1155TransferEvent>,
    pub internal_transactions: Vec<InternalTransaction>,

    /// Standard ERC-20 mutating calls (`transfer`/`transferFrom`/`approve`)
    /// decoded from the call trace at any depth. Captures balance moves that
    /// emit no `Transfer` event (custody backdoors). Cross-reference against
    /// `erc20_transfers` to find event-less moves.
    #[serde(default)]
    pub internal_erc20_calls: Vec<InternalErc20Call>,

    /// Event-less ERC-20 transfers — the value-moving subset of
    /// `internal_erc20_calls` (Transfer/TransferFrom, succeeded, non-zero) that
    /// emitted NO matching `Transfer` event. `erc20_transfers ∪
    /// internal_erc20_transfers` is the complete transfer set; both feed
    /// `address_balance_changes`. Derived via
    /// `InternalErc20Transfer::event_less_complement`.
    #[serde(default)]
    pub internal_erc20_transfers: Vec<InternalErc20Transfer>,

    // DEX events
    pub uniswap_v2_syncs: Vec<UniswapV2SyncEvent>,
    pub uniswap_v2_swaps: Vec<UniswapV2SwapEvent>,
    pub uniswap_v3_pools: Vec<UniswapV3PoolCreatedEvent>,
    pub uniswap_v3_initializations: Vec<UniswapV3InitializeEvent>,
    pub uniswap_v3_burns: Vec<UniswapV3BurnEvent>,
    pub uniswap_v3_mints: Vec<UniswapV3MintEvent>,
    pub uniswap_v3_swaps: Vec<UniswapV3SwapEvent>,
    pub uniswap_v3_positions: Vec<UniswapV3PositionEvent>,
    pub uniswap_v3_increases: Vec<UniswapV3IncreaseLiquidityEvent>,
    pub uniswap_v3_decreases: Vec<UniswapV3DecreaseLiquidityEvent>,
    pub uniswap_v4_initializes: Vec<UniswapV4InitializeEvent>,
    pub uniswap_v4_modifies: Vec<UniswapV4ModifyLiquidityEvent>,
    pub uniswap_v4_swaps: Vec<UniswapV4SwapEvent>,
    pub uniswap_v4_donates: Vec<UniswapV4DonateEvent>,
    pub uniswap_v4_protocol_fee_updates: Vec<UniswapV4FeeUpdatedEvent>,
    pub uniswap_v4_dynamic_lp_fee_updates: Vec<UniswapV4DynamicLPFeeUpdatedEvent>,
    pub uniswap_v4_protocol_fee_controller_updates: Vec<UniswapV4FeeControllerUpdatedEvent>,
    pub uniswap_v4_balance_deltas: Vec<UniswapV4BalanceDeltaEvent>,
    pub permit2_events: Vec<Permit2Event>,
    pub access_list: Vec<ProcessedAccessListItem>,
    pub blob_versioned_hashes: Vec<B256>,
    pub signed_authorizations: Vec<SignedAuthorization>,

    // Other events and actions
    pub erc20_approval_events: Vec<ERC20ApprovalEvent>,
    pub erc721_approval_events: Vec<ERC721ApprovalEvent>,
    #[serde(default)]
    pub approval_for_all_events: Vec<ApprovalForAllEvent>,
    pub uniswap_v2_mints: Vec<UniswapV2MintEvent>,
    pub uniswap_v2_burns: Vec<UniswapV2BurnEvent>,
    pub deposit_events: Vec<DepositEvent>,
    pub withdraw_events: Vec<WithdrawEvent>,
    pub uniswap_v2_pair_created_events: Vec<UniswapV2PairCreatedEvent>,
    pub ownership_transferred_events: Vec<OwnershipTransferredEvent>,
    pub ownership_transfer_started_events: Vec<OwnershipTransferStartedEvent>,
    pub access_control_role_granted_events: Vec<AccessControlRoleGrantedEvent>,
    pub access_control_role_revoked_events: Vec<AccessControlRoleRevokedEvent>,
    pub proxy_admin_changed_events: Vec<ProxyAdminChangedEvent>,
    pub contract_creation_events: Vec<ContractCreationEvent>,
    pub trading_enabled_events: Vec<TradingEnabledEvent>,
    pub trading_disabled_events: Vec<TradingDisabledEvent>,

    // Generic events and state
    #[serde(with = "super::serde_helpers::json_value_vec_map_binary")]
    pub other_events: Vec<HashMap<String, serde_json::Value>>,
    pub address_balance_changes: HashMap<Address, AddressBalanceChange>,
    #[serde(with = "super::serde_helpers::json_value_address_map_binary")]
    pub latest_states: HashMap<Address, serde_json::Value>,
    pub input: Vec<u8>,
    pub struct_logs: Option<Vec<StructLog>>,
}

impl ProcessedTransaction {
    /// Add trace-derived ERC-20 call/transfer participants to the address
    /// indexes that downstream token routing uses.
    pub fn refresh_trace_derived_indexes(&mut self) {
        for call in &self.internal_erc20_calls {
            if !is_canonical_weth(call.token_address) {
                self.erc20_contracts.insert(call.token_address);
            }
            self.unique_addresses.insert(call.token_address);
            self.unique_addresses.insert(call.caller);
            self.unique_addresses.insert(call.from_address);
            self.unique_addresses.insert(call.to_address);
        }
        for transfer in &self.internal_erc20_transfers {
            if !is_canonical_weth(transfer.token_address) {
                self.erc20_contracts.insert(transfer.token_address);
            }
            self.unique_addresses.insert(transfer.token_address);
            self.unique_addresses.insert(transfer.caller);
            self.unique_addresses.insert(transfer.from_address);
            self.unique_addresses.insert(transfer.to_address);
        }
    }

    /// Create a failed/empty transaction placeholder for skipped transactions
    pub fn empty_failed(from: Address, to: Option<Address>, nonce: u64, reason: &str) -> Self {
        let hash = B256::from_slice(&[nonce as u8; 32]);
        let mut tx = Self::new(
            hash,
            0,     // block_number
            0,     // block_timestamp
            nonce, // tx_index
            from,
            to,
            U256::ZERO, // value
            false,      // status (failed)
            nonce,
            0,
            Vec::new(), // input
        );
        tx.tx_type = "skipped".to_string();
        tx.actions = vec![reason.to_string()];
        tx
    }

    pub fn new(
        hash: B256,
        block_number: u64,
        block_timestamp: u64,
        tx_index: u64,
        from_address: Address,
        to_address: Option<Address>,
        value: U256,
        status: bool,
        nonce: u64,
        raw_tx_type: u8,
        input: Vec<u8>,
    ) -> Self {
        Self {
            hash,
            block_number,
            block_timestamp,
            tx_index,
            from_address,
            to_address,
            contract_address: None,
            value,
            status,
            nonce,
            raw_tx_type,
            tx_type: String::new(),
            actions: Vec::new(),
            fees: TransactionFees::default(),
            bribe_amount: U256::ZERO,
            unique_addresses: HashSet::new(),
            erc20_contracts: HashSet::new(),
            erc721_contracts: HashSet::new(),
            erc1155_contracts: HashSet::new(),
            eth_transfers: Vec::new(),
            erc20_transfers: Vec::new(),
            erc721_transfers: Vec::new(),
            erc1155_transfers: Vec::new(),
            internal_transactions: Vec::new(),
            internal_erc20_calls: Vec::new(),
            internal_erc20_transfers: Vec::new(),
            uniswap_v2_syncs: Vec::new(),
            uniswap_v2_swaps: Vec::new(),
            uniswap_v3_pools: Vec::new(),
            uniswap_v3_initializations: Vec::new(),
            uniswap_v3_burns: Vec::new(),
            uniswap_v3_mints: Vec::new(),
            uniswap_v3_swaps: Vec::new(),
            uniswap_v3_positions: Vec::new(),
            uniswap_v3_increases: Vec::new(),
            uniswap_v3_decreases: Vec::new(),
            uniswap_v4_initializes: Vec::new(),
            uniswap_v4_modifies: Vec::new(),
            uniswap_v4_swaps: Vec::new(),
            uniswap_v4_donates: Vec::new(),
            uniswap_v4_protocol_fee_updates: Vec::new(),
            uniswap_v4_dynamic_lp_fee_updates: Vec::new(),
            uniswap_v4_protocol_fee_controller_updates: Vec::new(),
            uniswap_v4_balance_deltas: Vec::new(),
            permit2_events: Vec::new(),
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            signed_authorizations: Vec::new(),
            erc20_approval_events: Vec::new(),
            erc721_approval_events: Vec::new(),
            approval_for_all_events: Vec::new(),
            uniswap_v2_mints: Vec::new(),
            uniswap_v2_burns: Vec::new(),
            deposit_events: Vec::new(),
            withdraw_events: Vec::new(),
            uniswap_v2_pair_created_events: Vec::new(),
            ownership_transferred_events: Vec::new(),
            ownership_transfer_started_events: Vec::new(),
            access_control_role_granted_events: Vec::new(),
            access_control_role_revoked_events: Vec::new(),
            proxy_admin_changed_events: Vec::new(),
            contract_creation_events: Vec::new(),
            trading_enabled_events: Vec::new(),
            trading_disabled_events: Vec::new(),
            other_events: Vec::new(),
            address_balance_changes: HashMap::new(),
            latest_states: HashMap::new(),
            input,
            struct_logs: None,
        }
    }

    /// Get currency balance change for an address by currency symbol (ETH, USDC, USDT, etc.)
    /// Returns the signed amount from currency_net
    pub fn get_address_currency_balance_change(
        &self,
        address: &Address,
        symbol: &str,
    ) -> Option<I256> {
        self.address_balance_changes
            .get(address)
            .and_then(|changes| changes.currency_net.get(symbol))
            .copied()
    }

    /// Get token balance change for an address by token contract address
    /// Returns the signed amount from token_net
    pub fn get_address_token_balance_change(
        &self,
        address: &Address,
        token_address: &Address,
    ) -> Option<I256> {
        let token_checksum = to_checksum_address(token_address);
        self.address_balance_changes
            .get(address)
            .and_then(|changes| changes.token_net.get(&token_checksum))
            .copied()
    }
}

fn is_canonical_weth(address: Address) -> bool {
    address == alloy_primitives::address!("0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2")
}
