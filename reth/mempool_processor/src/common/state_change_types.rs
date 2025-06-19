//! Defines the data structures that represent the state changes resulting from a transaction simulation.
//! These types were originally from the `revm_tx_simulator` crate, brought in to resolve a build issue.

use revm_primitives::{Address as RevmAddress, U256 as RevmU256};
use std::collections::HashMap;
use std::ops::{Add, Sub};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignedAmount {
    pub absolute_value: RevmU256,
    pub is_negative: bool,
}

impl SignedAmount {
    pub fn new(value: RevmU256, is_negative: bool) -> Self {
        Self {
            absolute_value: value,
            is_negative,
        }
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
    pub eth: EthMovementsInOut,                           // ETH Movements (including WETH transfers)
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
} 