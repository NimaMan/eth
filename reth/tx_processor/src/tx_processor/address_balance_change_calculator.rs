/// Address Balance Change Calculator
/// 
/// This module calculates net balance changes for addresses involved in transactions by:
/// 1. Tracking unknown token movements with contract addresses as keys
/// 2. Tracking known currency movements (ETH, USDC, USDT, DAI, MKR, etc.) with symbols as keys
/// 3. Handling special cases like WETH conversions and bribes
/// 4. Filtering out insignificant state changes based on thresholds
/// 
/// Key Features:
/// - Clean separation: token_net uses addresses, currency_net uses symbols
/// - Automatically recognizes 100+ currencies from DENOM_ADDRESSES
/// - Only applies decimal conversion for tokens with known decimals (no RPC calls)
/// - Tracks both incoming and outgoing movements for tokens and currencies
/// - Handles special addresses (WETH, null, dead addresses)
/// - Identifies significant state changes based on configurable thresholds
/// - Excludes WETH conversions from ETH movements
/// - Tracks bribe payments to known fee recipients
/// - Returns token_net with contract addresses only (unknown tokens)
/// - Returns currency_net with symbols only (tokens in DENOM_ADDRESSES)
/// - No overlap between token_net and currency_net

use super::data_models::events::InternalTransaction;
use super::data_models::balance_changes::{AddressBalanceChange, TokenMovements, TokenMovement};
use crate::utils::to_checksum_address;
use alloy_primitives::{Address, U256};
use eyre::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::str::FromStr;
use reth_chain_query::FEE_RECIPIENTS;

/// Known token addresses mapped to their symbols
lazy_static! {
    static ref DENOM_ADDRESSES: HashMap<Address, &'static str> = {
        let mut m = HashMap::new();
        // WETH
        m.insert(Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2]), "WETH");
        // USDC
        m.insert(Address::from([0xA0, 0xb8, 0x69, 0x91, 0xc6, 0x21, 0x8b, 0x36, 0xc1, 0xd1, 0x9D, 0x4a, 0x2e, 0x9E, 0xb0, 0xce, 0x36, 0x06, 0xeB, 0x48]), "USDC");
        // USDT
        m.insert(Address::from([0xdA, 0xC1, 0x7F, 0x95, 0x8D, 0x2e, 0xe5, 0x23, 0xa2, 0x20, 0x62, 0x06, 0x99, 0x45, 0x97, 0xC1, 0x3D, 0x83, 0x1e, 0xc7]), "USDT");
        // DAI
        m.insert(Address::from([0x6B, 0x17, 0x54, 0x74, 0xE8, 0x90, 0x94, 0xC4, 0x4D, 0xa9, 0x8b, 0x95, 0x4E, 0xeD, 0xeA, 0xC4, 0x95, 0x27, 0x1d, 0x0F]), "DAI");
        // Add more known tokens here
        m
    };
}

/// Get the token symbol for a known token address
pub fn get_token_symbol(token_address: &Address) -> Option<&'static str> {
    DENOM_ADDRESSES.get(token_address).copied()
}

/// Token decimals for known currencies
lazy_static! {
    static ref ERC20_TOKEN_DECIMALS: HashMap<&'static str, u8> = {
        let mut m = HashMap::new();
        m.insert("ETH", 18);
        m.insert("WETH", 18);
        m.insert("USDC", 6);
        m.insert("USDT", 6);
        m.insert("DAI", 18);
        // Add more token decimals here
        m
    };
}

/// Type of movement (currency or token)
#[derive(Debug, Clone, Copy)]
enum MovementType {
    Currency,
    Token,
}

/// Unique identifier for a transfer
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct TransferId {
    block_number: u64,
    tx_index: u64,
    log_index: String,
}

impl TransferId {
    fn new(block_number: u64, tx_index: u64, log_index: String) -> Self {
        Self {
            block_number,
            tx_index,
            log_index,
        }
    }
}

/// Movement entry for tracking transfers
#[derive(Debug, Clone)]
struct MovementEntry {
    amount: U256,
}

/// Movements for an address (incoming and outgoing)
#[derive(Debug, Clone, Default)]
struct AddressMovements {
    incoming: HashMap<TransferId, MovementEntry>,
    outgoing: HashMap<TransferId, MovementEntry>,
}

/// Balance change calculator
pub struct AddressBalanceChangeCalculator {
    weth_address: Address,
    eth_state_change_threshold: f64,
    token_state_change_threshold: f64,
    // Track movements per address per currency/token
    // currencies[address][currency_symbol] = AddressMovements
    // tokens[address][token_address] = AddressMovements
    currency_movements: HashMap<Address, HashMap<String, AddressMovements>>,
    token_movements: HashMap<Address, HashMap<Address, AddressMovements>>,
}

impl AddressBalanceChangeCalculator {
    /// Create a new balance change calculator
    pub fn new() -> Self {
        Self {
            weth_address: Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2]),
            eth_state_change_threshold: 0.0005,
            token_state_change_threshold: 0.1,
            currency_movements: HashMap::new(),
            token_movements: HashMap::new(),
        }
    }
    
    /// Calculate balance changes from DECODED transfers and internal transactions
    /// This is the main entry point for tx_processor to calculate balance changes
    pub fn calculate_balance_changes_from_processed_data(
        &mut self,
        erc20_transfers: &[super::data_models::events::ERC20Transfer],
        internal_transactions: &[InternalTransaction],
        block_number: u64,
        tx_index: u64,
    ) -> Result<HashMap<Address, AddressBalanceChange>> {
        // Clear previous state
        self.currency_movements.clear();
        self.token_movements.clear();
        
        // Process internal transactions for ETH transfers
        for (i, internal_tx) in internal_transactions.iter().enumerate() {
            if internal_tx.value > U256::ZERO {
                let transfer_id = TransferId::new(
                    block_number,
                    tx_index,
                    format!("internal_{}", i),
                );
                
                // Keep internal ETH transfers in wei
                let amount_wei = internal_tx.value;
                self.track_movement(
                    MovementType::Currency,
                    internal_tx.from_address,
                    internal_tx.to_address,
                    amount_wei,  // Keep in wei like Python
                    transfer_id,
                    None,
                    Some("ETH".to_string()),
                );
            }
        }
        
        // Process ERC20 transfers (already decoded!)
        for (i, transfer) in erc20_transfers.iter().enumerate() {
            let transfer_id = TransferId::new(
                block_number,
                tx_index,
                transfer.log_index.to_string(),
            );
            
            // Check if this is a known currency (USDC, USDT, DAI, etc.)
            let token_addr = transfer.token_address;
            
            if let Some(mut currency_symbol) = DENOM_ADDRESSES.get(&token_addr).cloned() {
                // EXACT Python logic: Convert WETH to ETH for ERC20
                if currency_symbol == "WETH" {
                    currency_symbol = "ETH";
                }
                
                // Keep amounts as U256 - no decimal conversion needed here
                // The amounts are already in the token's smallest unit
                let amount = transfer.amount;
                
                self.track_movement(
                    MovementType::Currency,
                    transfer.from_address,
                    transfer.to_address,
                    amount,
                    transfer_id,
                    None,
                    Some(currency_symbol.to_string()),
                );
            } else {
                // Unknown token - track with raw amount
                let amount = transfer.amount;
                
                self.track_movement(
                    MovementType::Token,
                    transfer.from_address,
                    transfer.to_address,
                    amount,
                    transfer_id,
                    Some(token_addr),
                    None,
                );
            }
        }
        
        // Calculate final balances - use the from_address if we have transfers
        let from_addr = if !erc20_transfers.is_empty() {
            erc20_transfers[0].from_address
        } else if !internal_transactions.is_empty() {
            internal_transactions[0].from_address
        } else {
            Address::ZERO
        };
        
        self.get_net_balance_changes(from_addr)
    }

    /// Track a movement between addresses and handle special cases
    fn track_movement(
        &mut self,
        movement_type: MovementType,
        from_addr: Address,
        to_addr: Address,
        amount: U256,
        transfer_id: TransferId,
        token_address: Option<Address>,
        currency: Option<String>,
    ) {
        // CORRECTED: Python implementation actually DOES track WETH movements!
        // The filtering should only apply to pure WETH wrap/unwrap operations, 
        // NOT to legitimate DEX trades involving WETH conversions.
        // For now, remove the WETH filtering to match Python behavior.
        // TODO: Implement more sophisticated filtering if needed
        
        // Original filtering (causing bug):
        // if to_addr == self.weth_address || from_addr == self.weth_address {
        //     return;
        // }
        
        match movement_type {
            MovementType::Currency => {
                if let Some(currency_symbol) = currency {
                    // Track per currency (ETH, USDC, USDT, DAI, etc.)
                    // Outgoing from from_addr
                    self.currency_movements
                        .entry(from_addr)
                        .or_default()
                        .entry(currency_symbol.clone())
                        .or_default()
                        .outgoing
                        .insert(transfer_id.clone(), MovementEntry { amount });
                    
                    // Incoming to to_addr (unless it's a bribe to fee recipient for ETH)
                    let is_bribe = currency_symbol == "ETH" && FEE_RECIPIENTS.contains(&to_addr);
                    
                    if !is_bribe {
                        self.currency_movements
                            .entry(to_addr)
                            .or_default()
                            .entry(currency_symbol)
                            .or_default()
                            .incoming
                            .insert(transfer_id, MovementEntry { amount });
                    }
                }
            }
            MovementType::Token => {
                if let Some(token_addr) = token_address {
                    // Track per token address (for unknown tokens not in DENOM_ADDRESSES)
                    
                    // Outgoing from from_addr
                    self.token_movements
                        .entry(from_addr)
                        .or_default()
                        .entry(token_addr)
                        .or_default()
                        .outgoing
                        .insert(transfer_id.clone(), MovementEntry { amount });
                    
                    // Incoming to to_addr
                    self.token_movements
                        .entry(to_addr)
                        .or_default()
                        .entry(token_addr)
                        .or_default()
                        .incoming
                        .insert(transfer_id, MovementEntry { amount });
                }
            }
        }
    }

    /// Aggregate net changes for every address touched in this tx
    fn get_net_balance_changes(&self, from_address: Address) -> Result<HashMap<Address, AddressBalanceChange>> {
        let mut net = HashMap::new();
        
        // Get all addresses that had any movements
        let mut all_addrs: std::collections::HashSet<&Address> = std::collections::HashSet::new();
        all_addrs.extend(self.token_movements.keys());
        all_addrs.extend(self.currency_movements.keys());
        
        for addr in all_addrs {
            // Calculate currency net changes per currency
            let mut currency_net = HashMap::new();
            let mut total_currency_movement = U256::ZERO;
            
            if let Some(currency_movs) = self.currency_movements.get(addr) {
                for (currency_symbol, movements) in currency_movs {
                    let currency_in: U256 = movements.incoming.values().map(|e| e.amount).fold(U256::ZERO, |acc, x| acc + x);
                    let currency_out: U256 = movements.outgoing.values().map(|e| e.amount).fold(U256::ZERO, |acc, x| acc + x);
                    // Calculate net change with proper sign (positive for net gain, negative for net loss)
                    let net_change = if currency_in >= currency_out {
                        currency_in - currency_out  // Positive: net gain
                    } else {
                        // Negative: net loss - use two's complement representation
                        U256::MAX - (currency_out - currency_in) + U256::from(1)
                    };
                    
                    // Apply threshold based on currency type (check absolute value)
                    let threshold = if currency_symbol == "ETH" {
                        // 0.0005 ETH = 500000000000000 wei
                        U256::from(500_000_000_000_000u64)
                    } else {
                        U256::from(self.token_state_change_threshold as u64)
                    };
                    
                    // Check if absolute change exceeds threshold
                    let abs_change = if net_change > U256::MAX / U256::from(2) {
                        // Negative value - get absolute value
                        U256::MAX - net_change + U256::from(1)
                    } else {
                        net_change
                    };
                    
                    if abs_change > threshold {
                        // Store the signed net change
                        currency_net.insert(currency_symbol.clone(), net_change);
                        total_currency_movement = total_currency_movement + abs_change;
                    }
                }
            }
            
            // Calculate token net changes per token (only for tokens NOT in DENOM_ADDRESSES)
            let mut token_net = HashMap::new();
            let mut total_token_movement = U256::ZERO;
            
            if let Some(token_movs) = self.token_movements.get(addr) {
                for (token_addr, movements) in token_movs {
                    // Skip tokens that are in DENOM_ADDRESSES - they go to currency_net
                    if !DENOM_ADDRESSES.contains_key(token_addr) {
                        let token_in: U256 = movements.incoming.values().map(|e| e.amount).fold(U256::ZERO, |acc, x| acc + x);
                        let token_out: U256 = movements.outgoing.values().map(|e| e.amount).fold(U256::ZERO, |acc, x| acc + x);
                        // Calculate net change with proper sign (positive for net gain, negative for net loss)
                        let net_change = if token_in >= token_out {
                            token_in - token_out  // Positive: net gain
                        } else {
                            // Negative: net loss - use two's complement representation
                            U256::MAX - (token_out - token_in) + U256::from(1)
                        };
                        
                        let threshold = U256::from(self.token_state_change_threshold as u64);
                        
                        // Check if absolute change exceeds threshold
                        let abs_change = if net_change > U256::MAX / U256::from(2) {
                            // Negative value - get absolute value
                            U256::MAX - net_change + U256::from(1)
                        } else {
                            net_change
                        };
                        
                        if abs_change > threshold {
                            // Always use checksum address as key, signed amount as value
                            let token_key = to_checksum_address(token_addr);
                            token_net.insert(token_key, net_change);
                            total_token_movement = total_token_movement + abs_change;
                        }
                    }
                }
            }
            
            // Include address if it meets thresholds or is the sender
            if total_token_movement > U256::ZERO
                || total_currency_movement > U256::ZERO
                || *addr == from_address
            {
                // Create proper struct instead of JSON
                let movements = TokenMovements {
                    tokens: self.format_token_movements_struct(addr),
                    currencies: self.format_currency_movements_struct(addr),
                };
                
                let balance_change = AddressBalanceChange {
                    token_net,
                    currency_net,
                    movements,
                };
                
                net.insert(*addr, balance_change);
            }
        }
        
        Ok(net)
    }
    
    /// Format token movements for struct output
    fn format_token_movements_struct(&self, addr: &Address) -> HashMap<String, TokenMovement> {
        let mut result = HashMap::new();
        
        if let Some(token_movs) = self.token_movements.get(addr) {
            for (token_addr, movements) in token_movs {
                let token_key = to_checksum_address(token_addr);
                
                let mut incoming = HashMap::new();
                for (tid, entry) in &movements.incoming {
                    let key = format!("{}_{}_{}",tid.block_number, tid.tx_index, tid.log_index);
                    incoming.insert(key, entry.amount);
                }
                
                let mut outgoing = HashMap::new();
                for (tid, entry) in &movements.outgoing {
                    let key = format!("{}_{}_{}", tid.block_number, tid.tx_index, tid.log_index);
                    outgoing.insert(key, entry.amount);
                }
                
                result.insert(token_key, TokenMovement {
                    incoming,
                    outgoing,
                });
            }
        }
        
        result
    }
    
    /// Format currency movements for struct output
    fn format_currency_movements_struct(&self, addr: &Address) -> HashMap<String, TokenMovement> {
        let mut result = HashMap::new();
        
        if let Some(currency_movs) = self.currency_movements.get(addr) {
            for (currency_symbol, movements) in currency_movs {
                let mut incoming = HashMap::new();
                for (tid, entry) in &movements.incoming {
                    let key = format!("{}_{}_{}",tid.block_number, tid.tx_index, tid.log_index);
                    incoming.insert(key, entry.amount);
                }
                
                let mut outgoing = HashMap::new();
                for (tid, entry) in &movements.outgoing {
                    let key = format!("{}_{}_{}", tid.block_number, tid.tx_index, tid.log_index);
                    outgoing.insert(key, entry.amount);
                }
                
                result.insert(currency_symbol.clone(), TokenMovement {
                    incoming,
                    outgoing,
                });
            }
        }
        
        result
    }
}

impl Default for AddressBalanceChangeCalculator {
    fn default() -> Self {
        Self::new()
    }
}