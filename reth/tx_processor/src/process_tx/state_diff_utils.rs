use std::collections::{HashMap, BTreeSet};
use std::sync::Arc;
use std::ops::{Add, Sub};

use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};
use alloy_eips::BlockId as AlloyBlockId;
use revm::database::{AlloyDB, CacheDB};
// use revm::database_interface::DatabaseAsync; // Not using db_before_tx for now
// use revm_state::AccountInfo; // No longer needed here
use revm_primitives::{Address as RevmAddress, B256 as RevmB256, KECCAK_EMPTY, U256 as RevmU256, Log as RevmLog, keccak256};
use revm_context::{
    TxEnv as RevmTxEnv_ctx, BlockEnv as RevmBlockEnv_ctx, TransactTo as RevmTransactTo_ctx,
};

// Constants
const ERC20_TRANSFER_EVENT_SIGNATURE_B256: RevmB256 = RevmB256::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);

// WETH contract address on Ethereum mainnet
const WETH_ADDRESS: RevmAddress = RevmAddress::new([
    0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e,
    0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2
]);

// Known token addresses and their symbols
pub fn get_token_symbol(address: &RevmAddress) -> Option<&'static str> {
    match address.as_slice() {
        // Stablecoins
        [0xA0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9D, 0x4a, 0x2e, 0x9E, 0xb0, 0xcE, 0x36, 0x06, 0xeB, 0x48] => Some("USDC"),
        [0xdA, 0xC1, 0x7F, 0x95, 0x8D, 0x2e, 0xe5, 0x23, 0xa2, 0x20, 0x62, 0x06, 0x99, 0x45, 0x97, 0xC1, 0x3D, 0x83, 0x1e, 0xc7] => Some("USDT"),
        [0x6B, 0x17, 0x54, 0x74, 0xE8, 0x90, 0x94, 0xC4, 0x4D, 0xa9, 0x8b, 0x95, 0x4E, 0xed, 0xeA, 0xC4, 0x95, 0x27, 0x1d, 0x0F] => Some("DAI"),
        [0x4F, 0xab, 0xb1, 0x45, 0xd6, 0x46, 0x52, 0xa9, 0x48, 0xd7, 0x25, 0x33, 0x02, 0x3f, 0x6E, 0x7A, 0x62, 0x3C, 0x7C, 0x53] => Some("BUSD"),
        [0x8E, 0x87, 0x0D, 0x67, 0xF6, 0x60, 0xD9, 0x5d, 0x5b, 0xe5, 0x30, 0x38, 0x0D, 0x0e, 0xC0, 0xbd, 0x38, 0x82, 0x89, 0xE1] => Some("PAX"),
        [0x95, 0x6F, 0x47, 0xF5, 0x0A, 0x91, 0x01, 0x63, 0xD8, 0xBF, 0x95, 0x7C, 0xf5, 0x84, 0x6D, 0x57, 0x3E, 0x7f, 0x87, 0xCA] => Some("FEI"),
        [0x85, 0x3d, 0x95, 0x5a, 0xCE, 0xf8, 0x22, 0xDb, 0x05, 0x8e, 0xb8, 0x50, 0x59, 0x11, 0xED, 0x77, 0xF1, 0x75, 0xb9, 0x9e] => Some("FRAX"),
        [0x5f, 0x98, 0x80, 0x5A, 0x4E, 0x8b, 0xe2, 0x55, 0xa3, 0x28, 0x80, 0xFD, 0xeC, 0x7F, 0x67, 0x28, 0xC6, 0x56, 0x8b, 0xA0] => Some("LUSD"),
        // Wrapped Tokens
        [0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2] => Some("WETH"),
        [0x22, 0x60, 0xFA, 0xC5, 0xE5, 0x54, 0x2a, 0x77, 0x3A, 0xa4, 0x4f, 0xBC, 0xfE, 0xDf, 0x7C, 0x19, 0x3b, 0xc2, 0xC5, 0x99] => Some("WBTC"),
        // Other Major Tokens
        [0x7D, 0x1A, 0xfA, 0x7B, 0x71, 0x8f, 0xb8, 0x93, 0xdB, 0x30, 0xA3, 0xaB, 0xc0, 0xCf, 0xc6, 0x08, 0xAa, 0xCf, 0xeB, 0xB0] => Some("MATIC"),
        [0x51, 0x49, 0x10, 0x77, 0x1A, 0xF9, 0xCa, 0x65, 0x6a, 0xf8, 0x40, 0xdf, 0xf8, 0x3E, 0x82, 0x64, 0xEc, 0xF9, 0x86, 0xCA] => Some("LINK"),
        [0x1f, 0x98, 0x40, 0xa8, 0x5d, 0x5a, 0xF5, 0xbf, 0x1D, 0x17, 0x62, 0xF9, 0x25, 0xBD, 0xAD, 0xdC, 0x42, 0x01, 0xF9, 0x84] => Some("UNI"),
        [0x7F, 0xc6, 0x65, 0x00, 0xc8, 0x4A, 0x76, 0xAd, 0x7e, 0x9c, 0x93, 0x43, 0x7b, 0xFc, 0x5A, 0xc3, 0x3E, 0x2D, 0xDa, 0xE9] => Some("AAVE"),
        _ => None,
    }
}

// Get token decimals
pub fn get_token_decimals(symbol: &str) -> u8 {
    match symbol {
        "USDC" | "USDT" | "EUROC" | "EURT" | "PYUSD" | "USDS" | "XAUt" | "XIDR" | "XSGD" | "XUSD" | "GYEN" => 6,
        "WBTC" => 8,
        "EURS" | "GUSD" | "IDRT" => 2,
        _ => 18, // Default to 18 decimals
    }
}

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
pub struct EthMovement {
    pub source_identifier: String, // e.g., "tx_value", "tx_fee", "block_reward", "internal_0"
    pub raw_amount: RevmU256,
}

#[derive(Debug, Clone, Default)]
pub struct TokenMovementsInOut {
    pub in_list: Vec<TokenMovement>,
    pub out_list: Vec<TokenMovement>,
}

#[derive(Debug, Clone, Default)]
pub struct EthMovementsInOut {
    pub in_list: Vec<EthMovement>,
    pub out_list: Vec<EthMovement>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountMovements {
    pub token: HashMap<RevmAddress, TokenMovementsInOut>, // Token Address -> Movements
    pub eth: EthMovementsInOut, // ETH Movements (including WETH transfers)
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: RevmAddress,
    pub symbol: String,
    pub decimals: u8,
    pub net_change: SignedAmount,
}

#[derive(Debug, Clone, Default)]
pub struct CalculatedAccountChanges {
    pub address: RevmAddress,
    pub eth_net_change: SignedAmount,
    pub token_net_changes: HashMap<RevmAddress, SignedAmount>, // Token Address -> Net Change
    pub token_infos: Vec<TokenInfo>, // Enhanced token info with symbols
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
            sender_changes.movements.eth.out_list.push(EthMovement {
                source_identifier: "tx_value_sent".to_string(),
                raw_amount: tx_env.value,
            });
        }
        sender_changes.movements.eth.out_list.push(EthMovement {
            source_identifier: "tx_fee_payment".to_string(),
            raw_amount: total_tx_fee,
        });
    }

    // Receiver gets value
    if let RevmTransactTo_ctx::Call(to_addr) = tx_env.kind {
        if tx_env.value > RevmU256::ZERO {
            if let Some(receiver_changes) = all_changes.get_mut(&to_addr) {
                receiver_changes.movements.eth.in_list.push(EthMovement {
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
            beneficiary_changes.movements.eth.in_list.push(EthMovement {
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

                // Check if this is a WETH transfer - handle as ETH movement
                if token_contract_addr == WETH_ADDRESS {
                    // WETH transfers are tracked as ETH movements, not token movements
                    // This matches Python behavior where WETH operations affect eth_net
                    let log_id_str = format!("log_{}", _log_idx);
                    
                    // For WETH, track as ETH movement
                    // Update sender - ETH goes out
                    if let Some(sender_changes) = all_changes.get_mut(&from_address) {
                        sender_changes.movements.eth.out_list.push(EthMovement {
                            source_identifier: log_id_str.clone(),
                            raw_amount: amount,
                        });
                        sender_changes.eth_net_change = sender_changes.eth_net_change.clone().subtract_positive(amount);
                    }
                    
                    // Update receiver - ETH comes in
                    if let Some(receiver_changes) = all_changes.get_mut(&to_address) {
                        receiver_changes.movements.eth.in_list.push(EthMovement {
                            source_identifier: log_id_str.clone(),
                            raw_amount: amount,
                        });
                        receiver_changes.eth_net_change = receiver_changes.eth_net_change.clone().add_positive(amount);
                    }
                } else {
                    // Process all non-WETH tokens as regular token transfers
                    let log_id_str = format!("log_{}", _log_idx);

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
    }

    // Populate token_infos with symbols and decimals
    for (_, changes) in all_changes.iter_mut() {
        for (token_addr, net_change) in &changes.token_net_changes {
            let symbol = get_token_symbol(token_addr)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{:?}", token_addr));
            
            let decimals = if let Some(sym) = get_token_symbol(token_addr) {
                get_token_decimals(sym)
            } else {
                18 // Default to 18 decimals
            };
            
            changes.token_infos.push(TokenInfo {
                address: *token_addr,
                symbol,
                decimals,
                net_change: net_change.clone(),
            });
        }
    }

    // Filter out WETH contract from state changes to match Python behavior
    all_changes.retain(|&addr, _| addr != WETH_ADDRESS);
    
    Ok(all_changes)
}

// --- Python-Compatible State Change Extraction ---

/// Error types for process_tx operations
#[derive(Debug, thiserror::Error)]
pub enum ProcessTxError {
    #[error("Transaction simulation failed: {0}")]
    SimulationError(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Invalid transaction data: {0}")]
    InvalidTransactionData(String),
}

/// Python-compatible state change format that matches the validation service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PythonCompatibleStateChanges {
    /// Address -> state changes for that address
    pub state_changes: std::collections::HashMap<String, AddressStateChange>,
    /// Processing metadata
    pub metadata: ProcessingMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddressStateChange {
    /// Net ETH change as signed decimal string (e.g., "-0.5", "1.25")
    pub eth_net: String,
    /// Token changes: symbol -> net change as signed decimal string
    pub token_net: std::collections::HashMap<String, String>,
    /// Detailed movements breakdown (same structure as Python)
    pub movements: MovementsBreakdown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MovementsBreakdown {
    /// ETH (denomination) movements
    pub denom: DenomMovements,
    /// Token movements by token address
    pub tokens: std::collections::HashMap<String, TokenMovements>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DenomMovements {
    /// Incoming ETH transfers: transfer_id -> amount
    #[serde(rename = "in")]
    pub in_transfers: std::collections::HashMap<String, f64>,
    /// Outgoing ETH transfers: transfer_id -> amount  
    #[serde(rename = "out")]
    pub out_transfers: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenMovements {
    /// Incoming token transfers: transfer_id -> amount
    #[serde(rename = "in")]
    pub in_transfers: std::collections::HashMap<String, f64>,
    /// Outgoing token transfers: transfer_id -> amount
    #[serde(rename = "out")]
    pub out_transfers: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessingMetadata {
    /// Transaction hash
    pub tx_hash: String,
    /// Block number
    pub block_number: u64,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
    /// Whether internal transfers were included
    pub includes_internal_transfers: bool,
    /// Total number of addresses affected
    pub addresses_affected: usize,
    /// Total number of tokens involved
    pub tokens_involved: usize,
}

/// Convert a RevmU256 amount to a human-readable decimal string with proper decimals
pub fn format_token_amount(amount: &RevmU256, decimals: u8, is_negative: bool) -> String {
    if *amount == RevmU256::ZERO {
        return "0".to_string();
    }

    let divisor = RevmU256::from(10).pow(RevmU256::from(decimals));
    let whole_part = *amount / divisor;
    let remainder = *amount % divisor;

    let sign = if is_negative { "-" } else { "" };

    if remainder == RevmU256::ZERO {
        format!("{}{}", sign, whole_part)
    } else {
        // Convert remainder to decimal string, padding with zeros
        let remainder_str = format!("{:0width$}", remainder, width = decimals as usize);
        let trimmed = remainder_str.trim_end_matches('0');
        
        if trimmed.is_empty() {
            format!("{}{}", sign, whole_part)
        } else {
            format!("{}{}.{}", sign, whole_part, trimmed)
        }
    }
}

/// Get token decimals (hardcoded for known tokens, should fetch from chain)
fn get_token_decimals_by_address(token_address: &str) -> u8 {
    match token_address.to_lowercase().as_str() {
        "0x6dafe226126cd471954b1e0a825e52f1d7c014b9" => 18, // The token from the test transaction
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => 6,  // USDC
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => 6,  // USDT
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => 18, // WETH
        _ => 18, // Default to 18 decimals
    }
}

/// Add two token amounts (handling negative values)
fn add_token_amounts(amount1: &str, amount2: &str) -> String {
    // Simple string-based addition for now
    // In production, would use proper decimal arithmetic
    if amount1 == "0" {
        return amount2.to_string();
    }
    if amount2 == "0" {
        return amount1.to_string();
    }
    
    // Parse as f64 for simplicity
    let val1: f64 = amount1.parse().unwrap_or(0.0);
    let val2: f64 = amount2.parse().unwrap_or(0.0);
    let sum = val1 + val2;
    
    if sum == 0.0 {
        "0".to_string()
    } else {
        format!("{}", sum)
    }
}

/// Convert address to EIP-55 checksummed format
pub fn checksum_address(address: &RevmAddress) -> String {
    let address_hex = format!("{:x}", address);
    let hash = keccak256(address_hex.as_bytes());
    
    let mut checksummed = String::with_capacity(42); // "0x" + 40 hex chars
    checksummed.push_str("0x");
    
    for (i, ch) in address_hex.chars().enumerate() {
        if ch.is_ascii_alphabetic() {
            // Check if the corresponding bit in the hash is set
            let byte_index = i / 2;
            let bit_index = if i % 2 == 0 { 4 } else { 0 };
            let bit_set = (hash[byte_index] >> bit_index) & 0x8 != 0;
            
            if bit_set {
                checksummed.push(ch.to_ascii_uppercase());
            } else {
                checksummed.push(ch.to_ascii_lowercase());
            }
        } else {
            checksummed.push(ch);
        }
    }
    
    checksummed
}

/// Convert ETH amount (18 decimals) to decimal string
pub fn format_eth_amount(signed_amount: &SignedAmount) -> String {
    format_token_amount(&signed_amount.absolute_value, 18, signed_amount.is_negative)
}

/// Python-compatible state change calculator that follows the exact Python logic
pub struct StateChangeCalculator {
    eth_state_change_threshold: f64,
    token_state_change_threshold: f64,
    weth_address: RevmAddress,
    // Detailed tracking for movements breakdown
    detailed_denom_movements: HashMap<String, HashMap<String, HashMap<String, f64>>>, // address -> {in/out -> {transfer_id -> amount}}
    detailed_token_movements: HashMap<String, HashMap<String, HashMap<String, HashMap<String, f64>>>>, // address -> token_addr -> {in/out -> {transfer_id -> amount}}
}

impl StateChangeCalculator {
    pub fn new(
        eth_threshold: f64,
        token_threshold: f64,
    ) -> Self {
        Self {
            eth_state_change_threshold: eth_threshold,
            token_state_change_threshold: token_threshold,
            weth_address: WETH_ADDRESS,
            detailed_denom_movements: HashMap::new(),
            detailed_token_movements: HashMap::new(),
        }
    }

    fn track_movement(
        &mut self,
        movement_type: &str,
        from_addr: &str,
        to_addr: &str,
        amount: f64,
        transfer_id: String,
        token_address: Option<&str>,
    ) {
        // WETH conversions are denomination-neutral (skip)
        if to_addr == "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" || 
           from_addr == "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" {
            return;
        }

        if movement_type == "denom" {
            // Track detailed outgoing from from_addr
            let from_entry = self.detailed_denom_movements.entry(from_addr.to_string()).or_insert_with(|| {
                let mut entry = HashMap::new();
                entry.insert("in".to_string(), HashMap::new());
                entry.insert("out".to_string(), HashMap::new());
                entry
            });
            from_entry.get_mut("out").unwrap().insert(transfer_id.clone(), amount);
            
            // Track detailed incoming to to_addr (skip fee recipient check for now)
            let to_entry = self.detailed_denom_movements.entry(to_addr.to_string()).or_insert_with(|| {
                let mut entry = HashMap::new();
                entry.insert("in".to_string(), HashMap::new());
                entry.insert("out".to_string(), HashMap::new());
                entry
            });
            to_entry.get_mut("in").unwrap().insert(transfer_id, amount);
            
        } else if movement_type == "token" && token_address.is_some() {
            let token_addr = token_address.unwrap();
            
            // Track detailed outgoing from from_addr
            let from_entry = self.detailed_token_movements.entry(from_addr.to_string()).or_insert_with(HashMap::new);
            let from_token_entry = from_entry.entry(token_addr.to_string()).or_insert_with(|| {
                let mut entry = HashMap::new();
                entry.insert("in".to_string(), HashMap::new());
                entry.insert("out".to_string(), HashMap::new());
                entry
            });
            from_token_entry.get_mut("out").unwrap().insert(transfer_id.clone(), amount);
            
            // Track detailed incoming to to_addr
            let to_entry = self.detailed_token_movements.entry(to_addr.to_string()).or_insert_with(HashMap::new);
            let to_token_entry = to_entry.entry(token_addr.to_string()).or_insert_with(|| {
                let mut entry = HashMap::new();
                entry.insert("in".to_string(), HashMap::new());
                entry.insert("out".to_string(), HashMap::new());
                entry
            });
            to_token_entry.get_mut("in").unwrap().insert(transfer_id, amount);
        }
    }

    pub fn get_net_changes(&self, from_address: &str) -> HashMap<String, AddressStateChange> {
        let mut net = HashMap::new();
        
        // Get all addresses that had movements
        let mut all_addrs = std::collections::HashSet::new();
        
        // Collect addresses from detailed denom movements
        for addr in self.detailed_denom_movements.keys() {
            all_addrs.insert(addr.clone());
        }
        
        // Collect addresses from detailed token movements
        for addr in self.detailed_token_movements.keys() {
            all_addrs.insert(addr.clone());
        }
        
        for addr in all_addrs {
            // Calculate ETH net change and build denom movements
            let mut denom_in_transfers = HashMap::new();
            let mut denom_out_transfers = HashMap::new();
            let mut eth_net = 0.0;
            
            if let Some(denom_movements) = self.detailed_denom_movements.get(&addr) {
                if let Some(in_transfers) = denom_movements.get("in") {
                    for (transfer_id, amount) in in_transfers {
                        denom_in_transfers.insert(transfer_id.clone(), *amount);
                        eth_net += amount;
                    }
                }
                if let Some(out_transfers) = denom_movements.get("out") {
                    for (transfer_id, amount) in out_transfers {
                        denom_out_transfers.insert(transfer_id.clone(), *amount);
                        eth_net -= amount;
                    }
                }
            }
            
            // Calculate token net changes and build token movements
            let mut token_net = HashMap::new();
            let mut token_movements_breakdown = HashMap::new();
            let mut total_token_movement = 0.0;
            
            if let Some(addr_tokens) = self.detailed_token_movements.get(&addr) {
                for (token_addr, directions) in addr_tokens {
                    let mut token_in_transfers = HashMap::new();
                    let mut token_out_transfers = HashMap::new();
                    let mut token_net_change = 0.0;
                    
                    if let Some(in_transfers) = directions.get("in") {
                        for (transfer_id, amount) in in_transfers {
                            token_in_transfers.insert(transfer_id.clone(), *amount);
                            token_net_change += amount;
                        }
                    }
                    if let Some(out_transfers) = directions.get("out") {
                        for (transfer_id, amount) in out_transfers {
                            token_out_transfers.insert(transfer_id.clone(), *amount);
                            token_net_change -= amount;
                        }
                    }
                    
                    if token_net_change.abs() > self.token_state_change_threshold {
                        token_net.insert(token_addr.clone(), format!("{}", token_net_change));
                        total_token_movement += token_net_change.abs();
                    }
                    
                    // Always include in movements breakdown for detailed view
                    token_movements_breakdown.insert(token_addr.clone(), TokenMovements {
                        in_transfers: token_in_transfers,
                        out_transfers: token_out_transfers,
                    });
                }
            }
            
            // Include address if it meets thresholds or is the sender
            if total_token_movement > 0.0 || 
               eth_net.abs() > self.eth_state_change_threshold || 
               addr == from_address {
                net.insert(addr, AddressStateChange {
                    eth_net: if eth_net == 0.0 { "0".to_string() } else { format!("{}", eth_net) },
                    token_net,
                    movements: MovementsBreakdown {
                        denom: DenomMovements {
                            in_transfers: denom_in_transfers,
                            out_transfers: denom_out_transfers,
                        },
                        tokens: token_movements_breakdown,
                    },
                });
            }
        }
        
        net
    }
}

/// Main function to extract state changes in Python-compatible format
pub async fn extract_state_changes_python_format(
    tx_hash: String,
    rpc_url: &str,
) -> Result<PythonCompatibleStateChanges, ProcessTxError> {
    use std::time::Instant;
    let start_time = Instant::now();

    // Import required modules for simulation
    use crate::simulate_signed_tx::simulate_signed_tx;
    use ethers_core::types::H256;
    use std::str::FromStr;

    // Parse transaction hash
    let hash = H256::from_str(&tx_hash)
        .map_err(|e| ProcessTxError::InvalidTransactionData(format!("Invalid hash: {}", e)))?;

    // Simulate the transaction to get state changes
    println!("🔍 CALLING SIMULATION for tx: {}", tx_hash);
    let simulation_result = simulate_signed_tx(hash, rpc_url).await
        .map_err(|e| ProcessTxError::SimulationError(format!("Simulation failed: {}", e)))?;
    println!("🔍 SIMULATION RETURNED {} internal transfers, {} logs", 
             simulation_result.internal_transfers.len(), 
             simulation_result.logs.len());

    // Get transaction details for sender
    let (from_address, block_number, txn_index) = {
        use ethers_providers::{Provider as EthersProvider, Http, Middleware};
        
        let provider = EthersProvider::<Http>::try_from(rpc_url)
            .map_err(|e| ProcessTxError::RpcError(format!("Failed to create provider: {}", e)))?;
        
        let tx_hash_h256 = H256::from_str(&tx_hash)
            .map_err(|e| ProcessTxError::InvalidTransactionData(format!("Invalid hash: {}", e)))?;
            
        let tx = provider.get_transaction(tx_hash_h256).await
            .map_err(|e| ProcessTxError::RpcError(format!("Failed to fetch transaction: {}", e)))?
            .ok_or_else(|| ProcessTxError::TransactionNotFound(tx_hash.clone()))?;
        
        let block_num = tx.block_number.unwrap_or_default().as_u64();
        let tx_idx = tx.transaction_index.unwrap_or_default().as_u64() as u32;
        let from_addr = checksum_address(&crate::conversions::ethers_to_revm_address(tx.from));
        
        println!("🔍 TRANSACTION INFO:");
        println!("  Block: {}", block_num);
        println!("  Transaction Index: {}", tx_idx);
        println!("  From: {}", from_addr);
        println!("  Value: {} wei", tx.value);
        println!("  Gas Price: {} wei", tx.gas_price.unwrap_or_default());
        
        (from_addr, block_num, tx_idx)
    };

    // Initialize calculator with Python's thresholds
    let mut calculator = StateChangeCalculator::new(0.0005, 0.1);

    // Process internal transfers (ETH movements)
    println!("🔍 RUST INTERNAL TRANSFERS ({} total):", simulation_result.internal_transfers.len());
    for (i, transfer) in simulation_result.internal_transfers.iter().enumerate() {
        let from_addr = checksum_address(&transfer.from);
        let to_addr = checksum_address(&transfer.to);
        
        // Convert U256 to f64 safely
        let amount_wei_str = transfer.value.to_string();
        let amount_wei_f64: f64 = amount_wei_str.parse().unwrap_or(0.0);
        let amount_eth = amount_wei_f64 / 1e18; // Convert wei to ETH
        let transfer_id = format!("{},{},internal_{}", block_number, txn_index, i);
        
        println!("  Internal {}: {} wei ({} ETH) from {} to {}", 
                 i, amount_wei_str, amount_eth, from_addr, to_addr);
        
        calculator.track_movement(
            "denom",
            &from_addr,
            &to_addr,
            amount_eth,
            transfer_id,
            None,
        );
    }

    // Process token transfers from logs
    println!("🔍 RUST ERC20 TRANSFERS ({} total logs):", simulation_result.logs.len());
    let mut erc20_count = 0;
    for (i, log) in simulation_result.logs.iter().enumerate() {
        if log.topics().len() >= 3 && log.topics()[0] == ERC20_TRANSFER_EVENT_SIGNATURE_B256 {
            let from_addr = checksum_address(&RevmAddress::from_slice(&log.topics()[1][12..]));
            let to_addr = checksum_address(&RevmAddress::from_slice(&log.topics()[2][12..]));
            let token_addr = checksum_address(&log.address);
            
            // Extract amount from data
            let amount = if log.data.data.len() >= 32 {
                RevmU256::from_be_bytes(log.data.data[0..32].try_into().unwrap_or([0u8; 32]))
            } else {
                RevmU256::ZERO
            };
            
            let transfer_id = format!("{},{},{}", block_number, txn_index, i);
            
            // Check if this is WETH - treat as ETH movement
            if token_addr.to_lowercase() == "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" {
                let amount_wei_str = amount.to_string();
                let amount_wei_f64: f64 = amount_wei_str.parse().unwrap_or(0.0);
                let amount_eth = amount_wei_f64 / 1e18;
                println!("  ERC20 {}: WETH {} wei ({} ETH) from {} to {}", 
                         i, amount_wei_str, amount_eth, from_addr, to_addr);
                calculator.track_movement(
                    "denom",
                    &from_addr,
                    &to_addr,
                    amount_eth,
                    transfer_id,
                    None,
                );
            } else {
                // Regular token transfer - keep as raw amount
                let amount_str = amount.to_string();
                let amount_f64: f64 = amount_str.parse().unwrap_or(0.0);
                println!("  ERC20 {}: Token {} amount {} from {} to {}", 
                         i, token_addr, amount_str, from_addr, to_addr);
                calculator.track_movement(
                    "token",
                    &from_addr,
                    &to_addr,
                    amount_f64,
                    transfer_id,
                    Some(&token_addr),
                );
            }
            erc20_count += 1;
        }
    }
    println!("  Total ERC20 transfers found: {}", erc20_count);

    // Get net changes
    let state_changes = calculator.get_net_changes(&from_address);
    
    // Count tokens involved and addresses before moving state_changes
    let tokens_involved: std::collections::HashSet<String> = state_changes
        .values()
        .flat_map(|change| change.token_net.keys().cloned())
        .collect();
    let addresses_affected = state_changes.len();

    let processing_time = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(PythonCompatibleStateChanges {
        state_changes,
        metadata: ProcessingMetadata {
            tx_hash,
            block_number,
            processing_time_ms: processing_time,
            includes_internal_transfers: true,
            addresses_affected,
            tokens_involved: tokens_involved.len(),
        },
    })
}

/// Convert CalculatedAccountChanges to Python-compatible format
pub fn convert_to_python_format(
    changes: &std::collections::HashMap<RevmAddress, CalculatedAccountChanges>,
    tx_hash: String,
    block_number: u64,
    processing_time_ms: f64,
) -> PythonCompatibleStateChanges {
    let mut state_changes = std::collections::HashMap::new();
    let mut tokens_involved = std::collections::HashSet::new();

    for (address, account_changes) in changes {
        let address_str = checksum_address(address);
        
        // Format ETH net change
        let eth_net = format_eth_amount(&account_changes.eth_net_change);
        
        // Format token net changes
        let mut token_net = std::collections::HashMap::new();
        for token_info in &account_changes.token_infos {
            let amount_str = format_token_amount(
                &token_info.net_change.absolute_value,
                token_info.decimals,
                token_info.net_change.is_negative,
            );
            token_net.insert(token_info.symbol.clone(), amount_str);
            tokens_involved.insert(token_info.symbol.clone());
        }

        state_changes.insert(address_str, AddressStateChange {
            eth_net,
            token_net,
            movements: MovementsBreakdown {
                denom: DenomMovements {
                    in_transfers: HashMap::new(),
                    out_transfers: HashMap::new(),
                },
                tokens: HashMap::new(),
            },
        });
    }

    PythonCompatibleStateChanges {
        state_changes,
        metadata: ProcessingMetadata {
            tx_hash,
            block_number,
            processing_time_ms,
            includes_internal_transfers: true,
            addresses_affected: changes.len(),
            tokens_involved: tokens_involved.len(),
        },
    }
}

/// Batch processing function for multiple transactions
pub async fn extract_batch_state_changes_python_format(
    tx_hashes: Vec<String>,
    rpc_url: &str,
) -> Result<Vec<PythonCompatibleStateChanges>, ProcessTxError> {
    use tokio::task::JoinSet;

    let mut join_set = JoinSet::new();
    
    // Process transactions concurrently
    for tx_hash in tx_hashes {
        let rpc_url = rpc_url.to_string();
        join_set.spawn(async move {
            extract_state_changes_python_format(tx_hash, &rpc_url).await
        });
    }
    
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(state_changes)) => results.push(state_changes),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(ProcessTxError::SimulationError(format!("Task failed: {}", e))),
        }
    }
    
    Ok(results)
}

/// Event counting structure to match Python service format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventCounts {
    pub erc20_transfers: usize,
    pub erc721_transfers: usize,
    pub erc1155_transfers: usize,
    pub internal_transactions: usize,
    pub uniswap_v2_swaps: usize,
    pub uniswap_v2_syncs: usize,
    pub uniswap_v3_swaps: usize,
    pub uniswap_v4_swaps: usize,
    pub approvals: usize,
    pub mints: usize,
    pub burns: usize,
    pub deposits: usize,
    pub withdraws: usize,
    pub permit2_events: usize,
    pub trading_enabled_events: usize,
    pub trading_disabled_events: usize,
}

impl Default for EventCounts {
    fn default() -> Self {
        EventCounts {
            erc20_transfers: 0,
            erc721_transfers: 0,
            erc1155_transfers: 0,
            internal_transactions: 0,
            uniswap_v2_swaps: 0,
            uniswap_v2_syncs: 0,
            uniswap_v3_swaps: 0,
            uniswap_v4_swaps: 0,
            approvals: 0,
            mints: 0,
            burns: 0,
            deposits: 0,
            withdraws: 0,
            permit2_events: 0,
            trading_enabled_events: 0,
            trading_disabled_events: 0,
        }
    }
}


/// Extract event counts from transaction logs to match Python format
pub fn extract_event_counts(logs: &[RevmLog]) -> EventCounts {
    let mut counts = EventCounts::default();
    
    for log in logs {
        if log.topics().is_empty() {
            continue;
        }
        
        let topic0 = log.topics()[0];
        
        // ERC20 Transfer event: Transfer(address indexed from, address indexed to, uint256 value)
        if topic0 == ERC20_TRANSFER_EVENT_SIGNATURE_B256 {
            counts.erc20_transfers += 1;
        }
        // Add more event signatures as needed
        // TODO: Add signatures for other events like Uniswap swaps, approvals, etc.
    }
    
    counts
} 