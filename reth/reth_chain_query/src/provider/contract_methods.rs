/// Contract method calls - Token queries via contract calls
/// 
/// These methods use the TxSimulator to execute view functions on contracts.
/// This is the universal way to get token balances that works with any implementation,
/// including proxies, upgradeable contracts, and non-standard tokens.

use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use super::RethQueryProvider;
use tx_simulator::contract_method_simulator::{
    encode_view_function_with_address,
    decode_uint256_result,
    decode_string_result,
};

impl RethQueryProvider {
    /// Get ERC20 token balance via balanceOf(address) view function
    pub(crate) async fn get_token_balance_internal(
        &self, 
        token: Address, 
        holder: Address, 
        block: Option<u64>
    ) -> Result<U256> {
        // balanceOf(address) selector: 0x70a08231
        let selector = [0x70, 0xa0, 0x82, 0x31];
        let data = encode_view_function_with_address(selector, holder);
        
        let result = self.tx_simulator.simulate_view_function(
            token,
            data,
            block
        ).await?;
        
        if result.success {
            Ok(decode_uint256_result(&result.output))
        } else {
            Ok(U256::ZERO) // Failed calls typically mean 0 balance
        }
    }

    /// Get ERC20 allowance via allowance(owner, spender) view function
    pub async fn get_token_allowance(&self, token: Address, owner: Address, spender: Address, block: Option<u64>) -> Result<U256> {
        // allowance(address,address) selector: 0xdd62ed3e
        let mut data = Vec::with_capacity(4 + 32 + 32);
        data.extend_from_slice(&[0xdd, 0x62, 0xed, 0x3e]);
        // owner (left-padded to 32 bytes)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(owner.as_slice());
        // spender (left-padded to 32 bytes)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_slice());

        let result = self.tx_simulator.simulate_view_function(
            token,
            Bytes::from(data),
            block
        ).await?;

        if result.success {
            Ok(decode_uint256_result(&result.output))
        } else {
            Ok(U256::ZERO)
        }
    }

    /// Get ERC20 total supply via totalSupply() view function
    pub async fn get_token_total_supply(&self, token: Address, block: Option<u64>) -> Result<U256> {
        // totalSupply() selector: 0x18160ddd
        let selector = [0x18, 0x16, 0x0d, 0xdd];
        let data = Bytes::from(selector.to_vec());
        
        let result = self.tx_simulator.simulate_view_function(
            token,
            data,
            block
        ).await?;
        
        if result.success {
            Ok(decode_uint256_result(&result.output))
        } else {
            Err(eyre::eyre!("Failed to get total supply"))
        }
    }
    
    /// Get ERC20 decimals via decimals() view function
    pub async fn get_token_decimals(&self, token: Address, block: Option<u64>) -> Result<u8> {
        // decimals() selector: 0x313ce567
        let selector = [0x31, 0x3c, 0xe5, 0x67];
        let data = Bytes::from(selector.to_vec());
        
        let result = self.tx_simulator.simulate_view_function(
            token,
            data,
            block
        ).await?;
        
        if result.success && result.output.len() >= 32 {
            Ok(result.output[31])
        } else {
            Ok(18) // Default to 18 decimals if call fails
        }
    }
    
    /// Get ERC20 symbol via symbol() view function
    pub async fn get_token_symbol(&self, token: Address, block: Option<u64>) -> Result<String> {
        // symbol() selector: 0x95d89b41
        let selector = [0x95, 0xd8, 0x9b, 0x41];
        let data = Bytes::from(selector.to_vec());
        
        let result = self.tx_simulator.simulate_view_function(
            token,
            data,
            block
        ).await?;
        
        if result.success {
            Ok(decode_string_result(&result.output))
        } else {
            Ok("UNKNOWN".to_string())
        }
    }
    
    /// Get ERC20 name via name() view function
    pub async fn get_token_name(&self, token: Address, block: Option<u64>) -> Result<String> {
        // name() selector: 0x06fdde03
        let selector = [0x06, 0xfd, 0xde, 0x03];
        let data = Bytes::from(selector.to_vec());
        
        let result = self.tx_simulator.simulate_view_function(
            token,
            data,
            block
        ).await?;
        
        if result.success {
            Ok(decode_string_result(&result.output))
        } else {
            Ok("Unknown Token".to_string())
        }
    }
    
    /// Get complete token metadata in one call
    pub async fn get_token_metadata(&self, token: Address) -> Result<TokenMetadata> {
        let (name, symbol, decimals, total_supply) = tokio::try_join!(
            self.get_token_name(token, None),
            self.get_token_symbol(token, None),
            self.get_token_decimals(token, None),
            self.get_token_total_supply(token, None),
        )?;
        
        Ok(TokenMetadata {
            address: token,
            name,
            symbol,
            decimals,
            total_supply,
        })
    }
    
    /// Check if an address is a contract by checking for code
    pub async fn is_contract(&self, address: Address, block: Option<u64>) -> Result<bool> {
        // Try to get code size - contracts have code, EOAs don't
        let call = tx_simulator::UnsignedTransaction {
            from: Some(Address::ZERO),
            to: Some(address),
            value: Some(U256::ZERO),
            data: Some(Bytes::new()),
            gas: Some(21000),
            ..Default::default()
        };
        
        match self.tx_simulator.simulate_call_at_block(
            call, 
            block.unwrap_or(self.get_latest_block()?)
        ).await {
            Ok(_) => {
                // If we can call it with empty data and it doesn't revert, 
                // check if it actually has code
                // TODO: Properly check code_hash from PlainAccountState
                Ok(false)
            },
            Err(_) => Ok(false),
        }
    }
    
    /// Execute a generic contract method (view/pure function)
    /// This is a flexible method to call any contract function
    pub async fn execute_contract_method(
        &self,
        contract: Address,
        method_name: &str,
        args: impl AsRef<[u8]>,
        block: Option<u64>
    ) -> Result<U256> {
        // For now, we'll handle common methods. In the future, this could use ABI encoding
        let data = match method_name {
            "totalSupply" => {
                // totalSupply() selector: 0x18160ddd
                Bytes::from(vec![0x18, 0x16, 0x0d, 0xdd])
            }
            "decimals" => {
                // decimals() selector: 0x313ce567
                Bytes::from(vec![0x31, 0x3c, 0xe5, 0x67])
            }
            "balanceOf" => {
                // balanceOf(address) selector: 0x70a08231
                // This would need the address argument encoded
                return Err(eyre::eyre!("balanceOf requires proper argument encoding"));
            }
            _ => {
                return Err(eyre::eyre!("Method {} not yet supported", method_name));
            }
        };
        
        let result = self.tx_simulator.simulate_view_function(
            contract,
            data,
            block
        ).await?;
        
        if result.success {
            Ok(decode_uint256_result(&result.output))
        } else {
            Err(eyre::eyre!("Contract method {} failed", method_name))
        }
    }
}

/// Token metadata
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}

// Helper functions are now imported from tx_simulator::contract_method_simulator
