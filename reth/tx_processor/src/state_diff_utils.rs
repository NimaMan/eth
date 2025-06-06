use std::collections::{HashMap, BTreeSet};
use std::sync::Arc;
use std::ops::{Add, Sub};

use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};
use alloy_eips::BlockId as AlloyBlockId;
use revm::database::{AlloyDB, CacheDB};
// use revm::database_interface::DatabaseAsync; // Not using db_before_tx for now
// use revm_state::AccountInfo; // No longer needed here
use revm_primitives::{Address as RevmAddress, B256 as RevmB256, KECCAK_EMPTY, U256 as RevmU256, Log as RevmLog};
use revm_context::{
    TxEnv as RevmTxEnv_ctx, BlockEnv as RevmBlockEnv_ctx, TransactTo as RevmTransactTo_ctx,
};

// Constants
const ERC20_TRANSFER_EVENT_SIGNATURE_B256: RevmB256 = RevmB256::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);

// Assuming SimCacheDB is defined in lib.rs or simulation_core.rs and re-exported
// For now, let's use a concrete type alias here based on current usage.
// This might need to be adjusted if SimCacheDB in lib.rs is more generic.
pub type SimCacheDBForDiff = CacheDB<revm::database::WrapDatabaseAsync<AlloyDB<AlloyEthereum, Arc<AlloyDynProvider<AlloyEthereum>>>>>;

// Enum to represent the status of an account before/after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatusInDiff {
    NonExistent,
    ExistedEmpty,
    ExistedWithState,
    Created,
    Deleted,
    // Added for simpler final state reporting
    FinalState, 
}

#[derive(Debug, Clone)]
pub struct StorageSlotDiff {
    pub old_value: RevmU256,
    pub new_value: RevmU256,
}

#[derive(Debug, Clone)]
pub struct AccountStateSummary { // Renamed from AccountStateDiff
    pub address: RevmAddress,
    pub balance_before: Option<RevmU256>,
    pub balance_after: RevmU256,
    pub nonce_after: u64,
    pub code_hash_after: Option<RevmB256>,
    pub final_storage: HashMap<RevmU256, RevmU256>,
    pub account_status: AccountStatusInDiff, // To indicate this is just the final state
}

// --- New Data Structures for Python-like State Changes ---

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignedAmount {
    pub absolute_value: RevmU256,
    pub is_negative: bool,
}

impl SignedAmount {
    pub fn new(value: RevmU256, is_negative: bool) -> Self {
        Self { absolute_value: value, is_negative }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    // Helper to convert to a displayable string with sign
    pub fn to_signed_string(&self) -> String {
        format!("{}{}", if self.is_negative { "-" } else { "" }, self.absolute_value)
    }

    // Helper methods for internal transfers
    pub fn add_positive(self, value: RevmU256) -> Self {
        self + SignedAmount::new(value, false)
    }

    pub fn subtract_positive(self, value: RevmU256) -> Self {
        self - SignedAmount::new(value, false)
    }
}

impl Add for SignedAmount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let val1 = self.absolute_value;
        let val2 = other.absolute_value;

        if self.is_negative == other.is_negative {
            Self::new(val1 + val2, self.is_negative)
        } else if val1 >= val2 {
            Self::new(val1 - val2, self.is_negative)
        } else {
            Self::new(val2 - val1, other.is_negative)
        }
    }
}

impl Sub for SignedAmount {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self + SignedAmount::new(other.absolute_value, !other.is_negative)
    }
}


#[derive(Debug, Clone, Default)]
pub struct TokenMovement {
    pub log_identifier: String, // e.g., "log_16"
    pub raw_amount: RevmU256,
}

#[derive(Debug, Clone, Default)]
pub struct DenomMovement {
    pub source_identifier: String, // e.g., "tx_value", "tx_fee", "block_reward"
    pub raw_amount: RevmU256,
}

#[derive(Debug, Clone, Default)]
pub struct TokenMovementsInOut {
    pub in_list: Vec<TokenMovement>,
    pub out_list: Vec<TokenMovement>,
}

#[derive(Debug, Clone, Default)]
pub struct DenomMovementsInOut {
    pub in_list: Vec<DenomMovement>,
    pub out_list: Vec<DenomMovement>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountMovements {
    pub token: HashMap<RevmAddress, TokenMovementsInOut>, // Token Address -> Movements
    pub denom: DenomMovementsInOut, // ETH Movements
}

#[derive(Debug, Clone, Default)]
pub struct CalculatedAccountChanges {
    pub address: RevmAddress,
    pub eth_net_change: SignedAmount,
    pub token_net_changes: HashMap<RevmAddress, SignedAmount>, // Token Address -> Net Change
    pub movements: AccountMovements,
    // Optional: could add final_nonce, final_code_hash if needed later
}

// Simplified function to extract only final states of touched accounts
pub async fn extract_final_touched_account_states(
    final_evm_db: &SimCacheDBForDiff,
    mut initial_balances: HashMap<RevmAddress, RevmU256>, // Pass initial balances for known addresses, make it mutable
    alloy_provider: Arc<AlloyDynProvider<AlloyEthereum>>, // Added alloy_provider
    fork_block_id: AlloyBlockId, // Added fork_block_id
) -> Result<Vec<AccountStateSummary>, anyhow::Error> { // Changed return type and error type
    let mut summaries: Vec<AccountStateSummary> = Vec::new();

    for (address, final_account_value) in final_evm_db.cache.accounts.iter() {
        let revm_address = *address;

        let balance_after = final_account_value.info.balance;
        let nonce_after = final_account_value.info.nonce;
        let code_hash_after_raw = final_account_value.info.code_hash;
        let code_hash_after = if code_hash_after_raw == KECCAK_EMPTY { None } else { Some(code_hash_after_raw) };

        let mut final_storage_map = HashMap::new();
        for (slot_key_u256, final_value_u256_ref) in final_account_value.storage.iter() {
            final_storage_map.insert(*slot_key_u256, *final_value_u256_ref);
        }
        
        // Check if balance_before is already fetched (for caller/to_address)
        // If not, fetch it for this touched account.
        let balance_before = match initial_balances.get(&revm_address).cloned() {
            Some(bal) => Some(bal),
            None => {
                // Fetch initial balance for other touched accounts
                match alloy_provider.get_balance(revm_address.into()).block_id(fork_block_id).await {
                    Ok(bal_on_chain) => {
                        let revm_bal = RevmU256::from_limbs(bal_on_chain.into_limbs());
                        initial_balances.insert(revm_address, revm_bal); // Store for potential future reference if needed
                        Some(revm_bal)
                    }
                    Err(e) => {
                        // Log or handle error appropriately
                        eprintln!("Warning: Failed to get initial balance for touched account {}: {}", revm_address, e);
                        None
                    }
                }
            }
        };

        summaries.push(AccountStateSummary {
            address: revm_address,
            balance_before,
            balance_after,
            nonce_after,
            code_hash_after,
            final_storage: final_storage_map,
            account_status: AccountStatusInDiff::FinalState, // Indicate this is a final state snapshot
        });
    }

    Ok(summaries)
}

// --- New Core Logic Function ---
pub async fn generate_calculated_account_changes(
    final_evm_db: &SimCacheDBForDiff,
    initial_eth_balances_input: &HashMap<RevmAddress, RevmU256>,
    transaction_logs: &[RevmLog],
    tx_env: &RevmTxEnv_ctx,
    block_env: &RevmBlockEnv_ctx, 
    gas_used: u64,
    alloy_provider: Arc<AlloyDynProvider<AlloyEthereum>>, // Un-prefixed
    fork_block_id: AlloyBlockId,                         // Un-prefixed
) -> Result<HashMap<RevmAddress, CalculatedAccountChanges>, anyhow::Error> {
    let mut all_changes: HashMap<RevmAddress, CalculatedAccountChanges> = HashMap::new();
    let mut relevant_accounts: BTreeSet<RevmAddress> = BTreeSet::new();
    let mut fetched_initial_balances: HashMap<RevmAddress, RevmU256> = initial_eth_balances_input.clone();

    // 1. Collect all relevant accounts
    relevant_accounts.insert(tx_env.caller);
    if let RevmTransactTo_ctx::Call(to_addr) = tx_env.kind {
        relevant_accounts.insert(to_addr);
    }
    relevant_accounts.insert(block_env.beneficiary); // Block beneficiary

    for (_log_idx, log) in transaction_logs.iter().enumerate() {
        if log.topics().len() >= 3 && log.topics()[0] == ERC20_TRANSFER_EVENT_SIGNATURE_B256 {
            // ERC20 Transfer: address(indexed from), address(indexed to), uint256 value
            // Topic1: from_address, Topic2: to_address
            if log.topics()[1].as_slice().len() >= 20 && log.topics()[2].as_slice().len() >= 20 {
                let from_address = RevmAddress::from_slice(&log.topics()[1].as_slice()[12..]);
                let to_address = RevmAddress::from_slice(&log.topics()[2].as_slice()[12..]);
                relevant_accounts.insert(from_address);
                relevant_accounts.insert(to_address);
                relevant_accounts.insert(log.address); // The token contract itself
            }
        }
    }

    // Initialize changes for all relevant accounts
    for &acc_addr in relevant_accounts.iter() {
        all_changes.insert(acc_addr, CalculatedAccountChanges { 
            address: acc_addr, 
            ..Default::default()
        });
    }

    // 2. Process ETH movements and net changes
    let tx_gas_price_u256 = RevmU256::from(tx_env.gas_price);
    let total_tx_fee = RevmU256::from(gas_used) * tx_gas_price_u256;

    // Sender pays fee and value
    if let Some(sender_changes) = all_changes.get_mut(&tx_env.caller) {
        if tx_env.value > RevmU256::ZERO {
            sender_changes.movements.denom.out_list.push(DenomMovement {
                source_identifier: "tx_value_sent".to_string(),
                raw_amount: tx_env.value,
            });
        }
        sender_changes.movements.denom.out_list.push(DenomMovement {
            source_identifier: "tx_fee_payment".to_string(),
            raw_amount: total_tx_fee,
        });
    }

    // Receiver gets value
    if let RevmTransactTo_ctx::Call(to_addr) = tx_env.kind {
        if tx_env.value > RevmU256::ZERO {
            if let Some(receiver_changes) = all_changes.get_mut(&to_addr) {
                receiver_changes.movements.denom.in_list.push(DenomMovement {
                    source_identifier: "tx_value_received".to_string(),
                    raw_amount: tx_env.value,
                });
            }
        }
    }
    
    // Beneficiary gets fee (simplified: all fee goes to beneficiary)
    // For EIP-1559, this would be more complex (tip to beneficiary, base fee burnt)
    // block_env.basefee is U256 (actually u64), tx_env.gas_priority_fee is Option<U256> (actually Option<u128>)
    let block_base_fee_u256 = RevmU256::from(block_env.basefee); // Convert u64 to U256
    let tx_priority_fee_u256 = tx_env.gas_priority_fee.map_or(RevmU256::ZERO, RevmU256::from); // Convert Option<u128> to U256
    
    let mut tip_to_beneficiary = RevmU256::ZERO;

    if tx_gas_price_u256 >= block_base_fee_u256 { // Compare U256 types
        let effective_tip_per_gas = tx_gas_price_u256 - block_base_fee_u256;
        let actual_tip_per_gas = effective_tip_per_gas.min(tx_priority_fee_u256);
        tip_to_beneficiary = RevmU256::from(gas_used) * actual_tip_per_gas; 
    }
    
    if tip_to_beneficiary > RevmU256::ZERO {
        if let Some(beneficiary_changes) = all_changes.get_mut(&block_env.beneficiary) {
            beneficiary_changes.movements.denom.in_list.push(DenomMovement {
                source_identifier: "tx_fee_reward_tip".to_string(),
                raw_amount: tip_to_beneficiary,
            });
        }
    }


    // Calculate final ETH balances and net changes
    for acc_addr in relevant_accounts.iter() {
        let initial_balance = match fetched_initial_balances.get(acc_addr).copied() {
            Some(bal) => bal,
            None => {
                // Fetch if not in the provided map
                match alloy_provider.get_balance((*acc_addr).into()).block_id(fork_block_id).await {
                    Ok(bal_on_chain) => {
                        let revm_bal = RevmU256::from_limbs(bal_on_chain.into_limbs());
                        fetched_initial_balances.insert(*acc_addr, revm_bal); // Store for future ref
                        revm_bal
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to fetch initial balance for {} via provider: {}. Assuming 0 for diff calc.", 
                            acc_addr, e
                        );
                        RevmU256::ZERO
                    }
                }
            }
        };

        let final_balance = final_evm_db.cache.accounts.get(acc_addr)
            .map(|acc_data| acc_data.info.balance)
            .unwrap_or_else(|| {
                 // If not in cache, it might mean it was not touched or its final state is same as pre-state.
                 // For simplicity, we'll try to fetch from provider if not in initial_balances map for completeness, 
                 // but ideally all *touched* accounts should be in final_evm_db.cache.
                 // This could be an area for refinement if initial_balances isn't exhaustive.
                initial_balance // Fallback to initial if not in cache
            });

        if let Some(changes) = all_changes.get_mut(acc_addr) {
            if final_balance >= initial_balance {
                changes.eth_net_change = SignedAmount::new(final_balance - initial_balance, false);
            } else {
                changes.eth_net_change = SignedAmount::new(initial_balance - final_balance, true);
            }
        }
    }

    // 3. Process ERC20 movements and net changes
    for (_log_idx, log) in transaction_logs.iter().enumerate() {
        if log.topics().len() >= 3 && log.topics()[0] == ERC20_TRANSFER_EVENT_SIGNATURE_B256 {
            if log.topics()[1].as_slice().len() >= 20 && log.topics()[2].as_slice().len() >= 20 {
                let token_contract_addr = log.address;
                let from_address = RevmAddress::from_slice(&log.topics()[1].as_slice()[12..]);
                let to_address = RevmAddress::from_slice(&log.topics()[2].as_slice()[12..]);
                
                // Correctly convert log data to [u8; 32] for U256::from_be_bytes
                let data_slice = log.data.data.as_ref();
                let mut amount_bytes = [0u8; 32];
                let data_len = data_slice.len();
                if data_len > 0 {
                    let start_index = if data_len >= 32 { 0 } else { 32 - data_len };
                    let len_to_copy = data_len.min(32);
                    let src_start_index = if data_len > 32 { data_len - 32 } else { 0 }; // If data is >32 bytes, take the last 32 bytes.
                    // Ensure that the slice operation is valid.
                    if src_start_index + len_to_copy <= data_slice.len() {
                         amount_bytes[start_index..start_index + len_to_copy].copy_from_slice(&data_slice[src_start_index..src_start_index + len_to_copy]);
                    } else {
                        // This case should ideally not happen if logic is correct, but as a fallback:
                        eprintln!(
                            "Warning: Slice bounds error in ERC20 amount parsing. data_len: {}, src_start: {}, len_to_copy: {}. Zeroing amount.",
                            data_len, src_start_index, len_to_copy
                        );
                        // amount_bytes remains zeros
                    }
                }
                let amount = RevmU256::from_be_bytes(amount_bytes);

                let log_id_str = format!("log_{}", _log_idx); // Use _log_idx here as well

                // Update for 'from' account
                if let Some(sender_changes) = all_changes.get_mut(&from_address) {
                    let token_movements = sender_changes.movements.token.entry(token_contract_addr).or_default();
                    token_movements.out_list.push(TokenMovement { log_identifier: log_id_str.clone(), raw_amount: amount });
                    
                    let net_change = sender_changes.token_net_changes.entry(token_contract_addr).or_insert_with(SignedAmount::zero);
                    *net_change = net_change.clone() - SignedAmount::new(amount, false); 
                }

                // Update for 'to' account
                if let Some(receiver_changes) = all_changes.get_mut(&to_address) {
                    let token_movements = receiver_changes.movements.token.entry(token_contract_addr).or_default();
                    token_movements.in_list.push(TokenMovement { log_identifier: log_id_str.clone(), raw_amount: amount });

                    let net_change = receiver_changes.token_net_changes.entry(token_contract_addr).or_insert_with(SignedAmount::zero);
                    *net_change = net_change.clone() + SignedAmount::new(amount, false);
                }
            }
        }
    }

    Ok(all_changes)
} 