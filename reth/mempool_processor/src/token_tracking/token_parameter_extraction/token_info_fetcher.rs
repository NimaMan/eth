/// Token Info Fetcher
/// 
/// Fetches ERC20 token parameters (name, symbol, decimals, totalSupply) from the blockchain
/// Similar to Python's contract_type.py get_erc20_contract_info
/// 
/// Algorithm:
/// 1. Get contract bytecode to verify it's deployed
/// 2. Call standard ERC20 methods: name(), symbol(), decimals(), totalSupply()
/// 3. Handle both string and bytes32 return types for legacy tokens
/// 4. Return None if contract is not ERC20 or calls fail

use alloy_primitives::{Address, U256, Bytes};
use alloy_sol_types::{SolCall, SolValue};
use eyre::Result;
use alloy_rpc_types::CallRequest;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_network::Ethereum;

/// ERC20 function signatures
alloy_sol_types::sol! {
    // Standard ERC20 functions returning string
    function name() external view returns (string);
    function symbol() external view returns (string);
    function decimals() external view returns (uint8);
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    
    // Legacy tokens returning bytes32
    function name() external view returns (bytes32);
    function symbol() external view returns (bytes32);
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}

pub struct TokenInfoFetcher {
    provider: Box<dyn Provider<Ethereum>>,
}

impl TokenInfoFetcher {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new()
            .on_http(rpc_url.parse()?)
            .boxed();
        Ok(Self { provider })
    }

    /// Fetch token info at specific block
    pub async fn get_token_info(
        &self,
        token_address: Address,
        block_number: Option<u64>,
    ) -> Result<Option<TokenInfo>> {
        // First check if contract exists
        let code = self.provider
            .get_code_at(token_address, block_number.map(Into::into).unwrap_or_default())
            .await?;
        
        if code.is_empty() || code.len() < 64 {
            return Ok(None); // No contract or minimal proxy
        }

        // Try to get decimals first (most reliable indicator of ERC20)
        let decimals = match self.call_decimals(token_address, block_number).await {
            Ok(d) => d,
            Err(_) => return Ok(None), // Not ERC20 if decimals() fails
        };

        // Get total supply
        let total_supply = match self.call_total_supply(token_address, block_number).await {
            Ok(ts) => ts,
            Err(_) => return Ok(None), // Not ERC20 if totalSupply() fails
        };

        // Try to get name and symbol (handle both string and bytes32)
        let name = self.call_name(token_address, block_number).await
            .unwrap_or_else(|_| "Unknown".to_string());
        
        let symbol = self.call_symbol(token_address, block_number).await
            .unwrap_or_else(|_| "UNKNOWN".to_string());

        Ok(Some(TokenInfo {
            address: token_address,
            name,
            symbol,
            decimals,
            total_supply,
        }))
    }

    async fn call_decimals(&self, token: Address, block: Option<u64>) -> Result<u8> {
        let call_data = decimalsCall {}.abi_encode();
        let request = CallRequest {
            to: Some(token),
            data: Some(call_data.into()),
            ..Default::default()
        };

        let result = self.provider
            .call(&request, block.map(Into::into).unwrap_or_default())
            .await?;

        let decimals = <u8>::abi_decode(&result, true)?;
        Ok(decimals)
    }

    async fn call_total_supply(&self, token: Address, block: Option<u64>) -> Result<U256> {
        let call_data = totalSupplyCall {}.abi_encode();
        let request = CallRequest {
            to: Some(token),
            data: Some(call_data.into()),
            ..Default::default()
        };

        let result = self.provider
            .call(&request, block.map(Into::into).unwrap_or_default())
            .await?;

        let total_supply = <U256>::abi_decode(&result, true)?;
        Ok(total_supply)
    }

    async fn call_name(&self, token: Address, block: Option<u64>) -> Result<String> {
        let call_data = nameCall {}.abi_encode();
        let request = CallRequest {
            to: Some(token),
            data: Some(call_data.into()),
            ..Default::default()
        };

        let result = self.provider
            .call(&request, block.map(Into::into).unwrap_or_default())
            .await?;

        // Try string first
        if let Ok(name) = <String>::abi_decode(&result, true) {
            return Ok(name);
        }

        // Try bytes32 for legacy tokens
        if let Ok(bytes32_name) = <[u8; 32]>::abi_decode(&result, true) {
            // Convert bytes32 to string, removing null bytes
            let name = String::from_utf8_lossy(&bytes32_name)
                .trim_end_matches('\0')
                .to_string();
            return Ok(name);
        }

        Err(eyre::eyre!("Failed to decode name"))
    }

    async fn call_symbol(&self, token: Address, block: Option<u64>) -> Result<String> {
        let call_data = symbolCall {}.abi_encode();
        let request = CallRequest {
            to: Some(token),
            data: Some(call_data.into()),
            ..Default::default()
        };

        let result = self.provider
            .call(&request, block.map(Into::into).unwrap_or_default())
            .await?;

        // Try string first
        if let Ok(symbol) = <String>::abi_decode(&result, true) {
            return Ok(symbol);
        }

        // Try bytes32 for legacy tokens
        if let Ok(bytes32_symbol) = <[u8; 32]>::abi_decode(&result, true) {
            // Convert bytes32 to string, removing null bytes
            let symbol = String::from_utf8_lossy(&bytes32_symbol)
                .trim_end_matches('\0')
                .to_string();
            return Ok(symbol);
        }

        Err(eyre::eyre!("Failed to decode symbol"))
    }

    /// Get pool reserves for a Uniswap V2 pair
    pub async fn get_pool_reserves(
        &self,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<(U256, U256)> {
        // Uniswap V2 getReserves() selector: 0x0902f1ac
        let call_data = hex::decode("0902f1ac")?;
        let request = CallRequest {
            to: Some(pool_address),
            data: Some(call_data.into()),
            ..Default::default()
        };

        let result = self.provider
            .call(&request, block_number.map(Into::into).unwrap_or_default())
            .await?;

        // getReserves returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        // We only need the first two values
        if result.len() >= 64 {
            let reserve0 = U256::from_be_slice(&result[0..32]);
            let reserve1 = U256::from_be_slice(&result[32..64]);
            Ok((reserve0, reserve1))
        } else {
            Err(eyre::eyre!("Invalid getReserves response"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_fetch_homer_token_info() {
        let fetcher = TokenInfoFetcher::new("http://localhost:8545").unwrap();
        let homer_address = Address::from_str("0x1c487BA431778C6d405023Ce9159163e9fc256D0").unwrap();
        
        // Fetch at block where liquidity was added
        let info = fetcher.get_token_info(homer_address, Some(23002112)).await.unwrap();
        
        if let Some(token_info) = info {
            println!("Token: {}", token_info.symbol);
            println!("Name: {}", token_info.name);
            println!("Decimals: {}", token_info.decimals);
            println!("Total Supply: {}", token_info.total_supply);
            
            assert_eq!(token_info.decimals, 9);
            assert_eq!(token_info.symbol, "HOMER");
        }
    }
}