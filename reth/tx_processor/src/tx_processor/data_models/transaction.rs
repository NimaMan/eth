use super::balance_changes::AddressBalanceChange;
use super::events::*;
use super::fees::TransactionFees;
use alloy_primitives::{Address, B256, I256, U256};
use reth_chain_query::to_checksum_address;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETHTransfer {
    pub from_address: Address,
    pub to_address: Address,
    pub amount: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCreationEvent {
    pub contract_address: Address,
    pub contract_type: String,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub name: Option<String>,
    pub total_supply: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    // Core transaction data
    pub hash: B256,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub txn_index: u64,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub contract_address: Option<Address>,
    pub value: U256,
    pub status: String,
    pub nonce: u64,

    // Transaction classification
    pub txn_type: String,
    pub actions: Vec<String>,

    // Fee information
    pub fees: TransactionFees,
    pub bribe_amount: f64,

    // Addresses and contracts involved
    pub unique_addresses: HashSet<Address>,
    pub erc20_contracts: HashSet<Address>,
    pub erc721_contracts: HashSet<Address>,
    pub erc1155_contracts: HashSet<Address>,

    // Transfer events
    pub eth_transfers: Vec<ETHTransfer>,
    pub erc20_transfers: Vec<ERC20Transfer>,
    pub erc721_transfers: Vec<ERC721Transfer>,
    pub erc1155_transfers: Vec<ERC1155Transfer>,
    pub internal_transactions: Vec<InternalTransaction>,

    // DEX events
    pub uniswap_v2_syncs: Vec<UniswapV2Sync>,
    pub uniswap_v2_swaps: Vec<UniswapV2Swap>,
    pub uniswap_v3_pools: Vec<UniswapV3PoolCreated>,
    pub uniswap_v3_initializations: Vec<UniswapV3Initialize>,
    pub uniswap_v3_burns: Vec<UniswapV3Burn>,
    pub uniswap_v3_mints: Vec<UniswapV3Mint>,
    pub uniswap_v3_swaps: Vec<UniswapV3Swap>,
    pub uniswap_v3_positions: Vec<UniswapV3Position>,
    pub uniswap_v3_increases: Vec<UniswapV3IncreaseLiquidity>,
    pub uniswap_v3_decreases: Vec<UniswapV3DecreaseLiquidity>,
    pub uniswap_v4_initializes: Vec<UniswapV4Initialize>,
    pub uniswap_v4_modifies: Vec<UniswapV4ModifyLiquidity>,
    pub uniswap_v4_swaps: Vec<UniswapV4Swap>,
    pub permit2_events: Vec<Permit2>,

    // Other events and actions
    pub approvals: Vec<ERC20Approval>,
    pub erc721_approvals: Vec<ERC721Approval>,
    pub mints: Vec<MintAction>,
    pub burns: Vec<BurnAction>,
    pub deposits: Vec<DepositAction>,
    pub withdraws: Vec<WithdrawAction>,
    pub pair_events: Vec<PairAction>,
    pub owner_events: Vec<OwnerEvent>,
    pub contract_creation_events: Vec<ContractCreationEvent>,
    pub trading_enabled_events: Vec<TradingEnabledEvent>,
    pub trading_disabled_events: Vec<TradingDisabledEvent>,

    // Generic events and state
    pub other_events: Vec<HashMap<String, serde_json::Value>>,
    pub address_balance_changes: HashMap<Address, AddressBalanceChange>,
    pub latest_states: HashMap<Address, serde_json::Value>,
    pub input: Vec<u8>,
}

impl ProcessedTransaction {
    /// Create a failed/empty transaction placeholder for skipped transactions
    pub fn empty_failed(from: Address, to: Option<Address>, nonce: u64, reason: &str) -> Self {
        let hash = B256::from_slice(&[nonce as u8; 32]);
        let mut tx = Self::new(
            hash,
            0,     // block_number
            0,     // block_timestamp
            nonce, // txn_index
            from,
            to,
            U256::ZERO,      // value
            "0".to_string(), // status (failed)
            nonce,
            vec![], // input
        );
        tx.txn_type = "skipped".to_string();
        tx.actions = vec![reason.to_string()];
        tx
    }

    pub fn new(
        hash: B256,
        block_number: u64,
        block_timestamp: u64,
        txn_index: u64,
        from_address: Address,
        to_address: Option<Address>,
        value: U256,
        status: String,
        nonce: u64,
        input: Vec<u8>,
    ) -> Self {
        Self {
            hash,
            block_number,
            block_timestamp,
            txn_index,
            from_address,
            to_address,
            contract_address: None,
            value,
            status,
            nonce,
            txn_type: String::new(),
            actions: Vec::new(),
            fees: TransactionFees::default(),
            bribe_amount: 0.0,
            unique_addresses: HashSet::new(),
            erc20_contracts: HashSet::new(),
            erc721_contracts: HashSet::new(),
            erc1155_contracts: HashSet::new(),
            eth_transfers: Vec::new(),
            erc20_transfers: Vec::new(),
            erc721_transfers: Vec::new(),
            erc1155_transfers: Vec::new(),
            internal_transactions: Vec::new(),
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
            permit2_events: Vec::new(),
            approvals: Vec::new(),
            erc721_approvals: Vec::new(),
            mints: Vec::new(),
            burns: Vec::new(),
            deposits: Vec::new(),
            withdraws: Vec::new(),
            pair_events: Vec::new(),
            owner_events: Vec::new(),
            contract_creation_events: Vec::new(),
            trading_enabled_events: Vec::new(),
            trading_disabled_events: Vec::new(),
            other_events: Vec::new(),
            address_balance_changes: HashMap::new(),
            latest_states: HashMap::new(),
            input,
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
