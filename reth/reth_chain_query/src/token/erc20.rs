/// ERC20 token-related queries
/// 
/// Provides methods for querying ERC20 token state including balances, supply, and metadata

use alloy_primitives::{Address, U256, B256};
use eyre::Result;
use reth_tx_simulator::RethTxSimulator;
use std::sync::Arc;
use tiny_keccak::{Hasher, Keccak};
// use hex_literal::hex; // Commented out unused import

/// Token information structure (legacy name for compatibility)
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: Address,
    pub total_supply: U256,
    pub decimals: u8,
    pub symbol: String,
    pub name: String,
}

/// ERC20 token information
#[derive(Debug, Clone)]
pub struct ERC20Info {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}

/// Token query module
pub struct TokenQuery {
    simulator: Arc<RethTxSimulator>,
}

impl TokenQuery {
    /// Create new TokenQuery instance
    pub fn new(simulator: Arc<RethTxSimulator>) -> Self {
        Self { simulator }
    }
    
    /// Get ERC20 token balance for a holder
    pub async fn get_erc20_balance(&self, token: Address, holder: Address, block_number: Option<u64>) -> Result<U256> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.provider_factory()
            .history_by_block_number(block_number.into())
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Calculate storage slot for balance mapping
        // Standard ERC20 balance mapping is at slot 0 for most tokens
        // balanceOf[address] = keccak256(abi.encode(address, uint256(0)))
        let slot = Self::calculate_mapping_slot(holder, 0);
        
        // Read storage
        let value = provider.storage(token, slot)
            .map_err(|e| eyre::eyre!("Failed to read storage: {}", e))?;
        
        Ok(value.unwrap_or(U256::ZERO))
    }
    
    /// Get ERC20 total supply by calling totalSupply() function
    pub async fn get_erc20_total_supply(&self, token: Address, block_number: Option<u64>) -> Result<U256> {
        // Build totalSupply() call - function selector is 0x18160ddd
        let total_supply_selector = hex_literal::hex!("18160ddd");
        let data = RethTxSimulator::encode_view_function_call(total_supply_selector);
        
        // Simulate the view function call
        let result = self.simulator.simulate_view_function(
            token,
            data,
            block_number,
        ).await?;
        
        // Check if call was successful
        if !result.success {
            return Ok(U256::ZERO);
        }
        
        // Parse the result as U256
        Ok(RethTxSimulator::decode_uint256_result(&result.output))
    }
    
    /// Get ERC20 decimals by calling decimals() function
    pub async fn get_erc20_decimals(&self, token: Address, block_number: Option<u64>) -> Result<u8> {
        // Build decimals() call - function selector is 0x313ce567
        let decimals_selector = hex_literal::hex!("313ce567");
        let data = RethTxSimulator::encode_view_function_call(decimals_selector);
        
        // Simulate the view function call
        let result = self.simulator.simulate_view_function(
            token,
            data,
            block_number,
        ).await?;
        
        // Check if call was successful
        if !result.success {
            // Default to 18 if call fails (most common)
            return Ok(18);
        }
        
        // Parse the result as uint8
        let decimals = RethTxSimulator::decode_uint8_result(&result.output);
        
        // Validate decimals is in reasonable range
        if decimals > 0 && decimals <= 36 {
            Ok(decimals)
        } else {
            // Default to 18 if invalid
            Ok(18)
        }
    }
    
    /// Get ERC20 token symbol by calling symbol() function
    pub async fn get_erc20_symbol(&self, token: Address, block_number: Option<u64>) -> Result<String> {
        // Build symbol() call - function selector is 0x95d89b41
        let symbol_selector = hex_literal::hex!("95d89b41");
        let data = RethTxSimulator::encode_view_function_call(symbol_selector);
        
        // Simulate the view function call
        let result = self.simulator.simulate_view_function(
            token,
            data,
            block_number,
        ).await?;
        
        // Check if call was successful
        if !result.success {
            return Ok(String::new());
        }
        
        // Parse the result as string
        Ok(RethTxSimulator::decode_string_result(&result.output))
    }
    
    /// Get ERC20 token name by calling name() function
    pub async fn get_erc20_name(&self, token: Address, block_number: Option<u64>) -> Result<String> {
        // Build name() call - function selector is 0x06fdde03
        let name_selector = hex_literal::hex!("06fdde03");
        let data = RethTxSimulator::encode_view_function_call(name_selector);
        
        // Simulate the view function call
        let result = self.simulator.simulate_view_function(
            token,
            data,
            block_number,
        ).await?;
        
        // Check if call was successful
        if !result.success {
            return Ok(String::new());
        }
        
        // Parse the result as string
        Ok(RethTxSimulator::decode_string_result(&result.output))
    }
    
    /// Calculate storage slot for mapping[address] => value
    fn calculate_mapping_slot(key: Address, mapping_slot: u64) -> B256 {
        let mut encoded = [0u8; 64];
        // First 32 bytes: the address (padded)
        encoded[12..32].copy_from_slice(key.as_slice());
        // Second 32 bytes: the mapping slot
        let slot_bytes = mapping_slot.to_be_bytes();
        encoded[56..64].copy_from_slice(&slot_bytes);
        
        let mut hasher = Keccak::v256();
        hasher.update(&encoded);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        
        B256::from(hash)
    }
    
    /// Get all ERC20 token information in one call
    pub async fn get_erc20_info(&self, token: Address, block_number: Option<u64>) -> Result<ERC20Info> {
        // Run all queries in parallel for efficiency
        let (name_result, symbol_result, decimals_result, supply_result) = tokio::join!(
            self.get_erc20_name(token, block_number),
            self.get_erc20_symbol(token, block_number),
            self.get_erc20_decimals(token, block_number),
            self.get_erc20_total_supply(token, block_number),
        );
        
        Ok(ERC20Info {
            address: token,
            name: name_result.unwrap_or_default(),
            symbol: symbol_result.unwrap_or_default(),
            decimals: decimals_result.unwrap_or(18),
            total_supply: supply_result.unwrap_or(U256::ZERO),
        })
    }
    
    /// Get allowance for a spender
    pub async fn get_allowance(&self, token: Address, owner: Address, spender: Address, block_number: Option<u64>) -> Result<U256> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.provider_factory()
            .history_by_block_number(block_number.into())
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Allowance mapping is typically at slot 1
        // allowance[owner][spender] = keccak256(spender . keccak256(owner . 1))
        
        // First hash: keccak256(owner . 1)
        let mut first_encoded = [0u8; 64];
        first_encoded[12..32].copy_from_slice(owner.as_slice());
        first_encoded[63] = 1;  // slot 1
        
        let mut hasher = Keccak::v256();
        hasher.update(&first_encoded);
        let mut first_hash = [0u8; 32];
        hasher.finalize(&mut first_hash);
        
        // Second hash: keccak256(spender . first_hash)
        let mut second_encoded = [0u8; 64];
        second_encoded[12..32].copy_from_slice(spender.as_slice());
        second_encoded[32..64].copy_from_slice(&first_hash);
        
        let mut hasher = Keccak::v256();
        hasher.update(&second_encoded);
        let mut final_hash = [0u8; 32];
        hasher.finalize(&mut final_hash);
        
        let storage_key = B256::from(final_hash);
        
        // Read storage
        let value = provider.storage(token, storage_key)
            .map_err(|e| eyre::eyre!("Failed to read storage: {}", e))?;
        
        Ok(value.unwrap_or(U256::ZERO))
    }
}