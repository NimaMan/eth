// ERC20 method decoding utilities
//
// This module provides utilities for decoding ERC20 token contract interactions
// from transaction input data.

use ethers::abi::{decode, ParamType, Token};
use ethers::types::{Address, U256};
use hex;
use tracing::{debug, trace};

/// Common ERC20 method signatures (first 4 bytes of keccak256 hash)
pub mod method_ids {
    pub const TRANSFER: &[u8] = &[0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
    pub const APPROVE: &[u8] = &[0x09, 0x5e, 0xa7, 0xb3]; // approve(address,uint256)
    pub const TRANSFER_FROM: &[u8] = &[0x23, 0xb8, 0x72, 0xdd]; // transferFrom(address,address,uint256)
    pub const BALANCE_OF: &[u8] = &[0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
    pub const TOTAL_SUPPLY: &[u8] = &[0x18, 0x16, 0x0d, 0xdd]; // totalSupply()
}

/// Represents a decoded ERC20 method call
#[derive(Debug, Clone)]
pub enum ERC20Method {
    Transfer { to: Address, amount: U256 },
    Approve { spender: Address, amount: U256 },
    TransferFrom { from: Address, to: Address, amount: U256 },
    BalanceOf { account: Address },
    TotalSupply,
    Unknown,
}

/// Decode ERC20 method from transaction input data
pub fn decode_erc20_method(input_data: &[u8]) -> Option<ERC20Method> {
    if input_data.len() < 4 {
        return None;
    }
    
    let method_id = &input_data[0..4];
    let params = &input_data[4..];
    
    match method_id {
        method_ids::TRANSFER => {
            if params.len() < 64 {
                debug!("Transfer method detected but insufficient data");
                return None;
            }
            
            // Decode transfer(address to, uint256 amount)
            let param_types = vec![ParamType::Address, ParamType::Uint(256)];
            match decode(&param_types, params) {
                Ok(tokens) => {
                    if let (Some(Token::Address(to)), Some(Token::Uint(amount))) = 
                        (tokens.get(0), tokens.get(1)) {
                        trace!("Decoded transfer: to={:?}, amount={}", to, amount);
                        return Some(ERC20Method::Transfer { 
                            to: *to, 
                            amount: *amount 
                        });
                    }
                }
                Err(e) => {
                    debug!("Failed to decode transfer params: {}", e);
                }
            }
        }
        
        method_ids::APPROVE => {
            if params.len() < 64 {
                debug!("Approve method detected but insufficient data");
                return None;
            }
            
            // Decode approve(address spender, uint256 amount)
            let param_types = vec![ParamType::Address, ParamType::Uint(256)];
            match decode(&param_types, params) {
                Ok(tokens) => {
                    if let (Some(Token::Address(spender)), Some(Token::Uint(amount))) = 
                        (tokens.get(0), tokens.get(1)) {
                        trace!("Decoded approve: spender={:?}, amount={}", spender, amount);
                        return Some(ERC20Method::Approve { 
                            spender: *spender, 
                            amount: *amount 
                        });
                    }
                }
                Err(e) => {
                    debug!("Failed to decode approve params: {}", e);
                }
            }
        }
        
        method_ids::TRANSFER_FROM => {
            if params.len() < 96 {
                debug!("TransferFrom method detected but insufficient data");
                return None;
            }
            
            // Decode transferFrom(address from, address to, uint256 amount)
            let param_types = vec![ParamType::Address, ParamType::Address, ParamType::Uint(256)];
            match decode(&param_types, params) {
                Ok(tokens) => {
                    if let (Some(Token::Address(from)), Some(Token::Address(to)), 
                            Some(Token::Uint(amount))) = 
                        (tokens.get(0), tokens.get(1), tokens.get(2)) {
                        trace!("Decoded transferFrom: from={:?}, to={:?}, amount={}", 
                               from, to, amount);
                        return Some(ERC20Method::TransferFrom { 
                            from: *from, 
                            to: *to, 
                            amount: *amount 
                        });
                    }
                }
                Err(e) => {
                    debug!("Failed to decode transferFrom params: {}", e);
                }
            }
        }
        
        method_ids::BALANCE_OF => {
            if params.len() < 32 {
                debug!("BalanceOf method detected but insufficient data");
                return None;
            }
            
            // Decode balanceOf(address account)
            let param_types = vec![ParamType::Address];
            match decode(&param_types, params) {
                Ok(tokens) => {
                    if let Some(Token::Address(account)) = tokens.get(0) {
                        trace!("Decoded balanceOf: account={:?}", account);
                        return Some(ERC20Method::BalanceOf { account: *account });
                    }
                }
                Err(e) => {
                    debug!("Failed to decode balanceOf params: {}", e);
                }
            }
        }
        
        method_ids::TOTAL_SUPPLY => {
            trace!("Decoded totalSupply call");
            return Some(ERC20Method::TotalSupply);
        }
        
        _ => {
            trace!("Unknown ERC20 method: 0x{}", hex::encode(method_id));
        }
    }
    
    None
}

/// Check if the input data represents an ERC20 transfer
pub fn is_erc20_transfer(input_data: &[u8]) -> bool {
    if input_data.len() < 4 {
        return false;
    }
    
    let method_id = &input_data[0..4];
    method_id == method_ids::TRANSFER || method_id == method_ids::TRANSFER_FROM
}

/// Extract transfer details if this is a transfer method
pub fn extract_transfer_details(input_data: &[u8]) -> Option<(Address, U256)> {
    match decode_erc20_method(input_data) {
        Some(ERC20Method::Transfer { to, amount }) => Some((to, amount)),
        Some(ERC20Method::TransferFrom { to, amount, .. }) => Some((to, amount)),
        _ => None,
    }
}

/// Format token amount with decimals (assumes 18 decimals by default)
pub fn format_token_amount(amount: U256, decimals: u8) -> String {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    if fraction.is_zero() {
        format!("{}", whole)
    } else {
        // Convert fraction to string with leading zeros
        let fraction_str = format!("{:0width$}", fraction, width = decimals as usize);
        // Trim trailing zeros
        let trimmed = fraction_str.trim_end_matches('0');
        if trimmed.is_empty() {
            format!("{}", whole)
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::abi::encode;
    use ethers::types::Address;
    
    #[test]
    fn test_decode_transfer() {
        // Create a transfer call: transfer(0x742d35Cc6634C0532925a3b844Bc9e7595f01234, 1000000000000000000)
        let to_addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f01234".parse::<Address>().unwrap();
        let amount = U256::from_dec_str("1000000000000000000").unwrap(); // 1 token
        
        let mut input = Vec::new();
        input.extend_from_slice(method_ids::TRANSFER);
        input.extend_from_slice(&encode(&[
            Token::Address(to_addr),
            Token::Uint(amount),
        ]));
        
        let decoded = decode_erc20_method(&input);
        assert!(matches!(decoded, Some(ERC20Method::Transfer { .. })));
        
        if let Some(ERC20Method::Transfer { to, amount: decoded_amount }) = decoded {
            assert_eq!(to, to_addr);
            assert_eq!(decoded_amount, amount);
        }
    }
    
    #[test]
    fn test_format_token_amount() {
        let amount = U256::from_dec_str("1234567890123456789").unwrap();
        assert_eq!(format_token_amount(amount, 18), "1.234567890123456789");
        
        let amount = U256::from_dec_str("1000000000000000000").unwrap();
        assert_eq!(format_token_amount(amount, 18), "1");
        
        let amount = U256::from_dec_str("1500000000000000000").unwrap();
        assert_eq!(format_token_amount(amount, 18), "1.5");
    }
}