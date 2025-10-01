/// Address Balance Change Data Structures
///
/// OBJECTIVE: Define proper Rust types for balance changes with full U256 precision
/// No JSON, no f64 precision loss, direct field access
///
/// Structure:
/// - AddressBalanceChange: Top-level balance change for an address
///   - token_net: Net change for unknown tokens (checksum address -> signed amount)
///   - currency_net: Net change for known currencies (symbol -> signed amount in appropriate units)
///   - movements: Detailed incoming/outgoing movements
///
/// Note: currency_net stores ETH in wei, USDC/USDT with 6 decimals applied, etc.
use alloy_primitives::{Address, I256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete balance change information for an address
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddressBalanceChange {
    /// Net token changes for unknown tokens (checksum address -> raw amount with decimals)
    pub token_net: HashMap<String, I256>,

    /// Net currency changes for known tokens (symbol -> amount)
    /// ETH is in wei, USDC/USDT have decimals applied
    pub currency_net: HashMap<String, I256>,

    /// Detailed movement information
    pub movements: TokenMovements,
}

/// Detailed token movements (incoming and outgoing)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenMovements {
    /// Token movements by checksum address
    pub tokens: HashMap<String, TokenMovement>,

    /// Currency movements by symbol
    pub currencies: HashMap<String, TokenMovement>,
}

/// Individual token/currency movement details
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenMovement {
    /// Incoming transfers (transfer_id -> amount)
    pub incoming: HashMap<String, U256>,

    /// Outgoing transfers (transfer_id -> amount)
    pub outgoing: HashMap<String, U256>,
}

impl AddressBalanceChange {
    /// Create a new empty balance change
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any balance changes
    pub fn has_changes(&self) -> bool {
        !self.token_net.is_empty() || !self.currency_net.is_empty()
    }

    /// Get token net change (signed)
    pub fn get_token_net(&self, token_address: &str) -> Option<I256> {
        self.token_net.get(token_address).copied()
    }

    /// Get currency net change (signed)
    pub fn get_currency_net(&self, symbol: &str) -> Option<I256> {
        self.currency_net.get(symbol).copied()
    }
}
