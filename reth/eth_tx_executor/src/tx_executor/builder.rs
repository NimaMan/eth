//! Transaction Builder for DEX interactions
//!
//! Builds transactions for Uniswap V2/V3 and other DEX protocols

use ethers::prelude::*;
use ethers::abi::{encode, Token};
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::{info, debug};

/// Common DEX router addresses on Ethereum mainnet
pub mod routers {
    use ethers::types::Address;
    
    lazy_static::lazy_static! {
        pub static ref UNISWAP_V2_ROUTER: Address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap();
        pub static ref UNISWAP_V3_ROUTER: Address = "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap();
        pub static ref SUSHISWAP_ROUTER: Address = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".parse().unwrap();
        pub static ref WETH: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
    }
}

/// Transaction builder for DEX swaps
pub struct TransactionBuilder {
    /// Provider for gas estimation
    provider: Arc<Provider<Http>>,
    /// Default router address
    router_address: Address,
    /// Wallet address for recipient
    wallet_address: Address,
    /// Slippage tolerance (e.g., 0.05 for 5%)
    slippage_tolerance: f64,
}

impl TransactionBuilder {
    /// Create new transaction builder
    pub fn new(
        provider: Arc<Provider<Http>>,
        router_address: Address,
        wallet_address: Address,
        slippage_tolerance: f64,
    ) -> Self {
        Self {
            provider,
            router_address,
            wallet_address,
            slippage_tolerance,
        }
    }
    
    /// Build emergency sell transaction (token -> ETH)
    pub async fn build_emergency_sell(
        &self,
        token_address: Address,
        amount: U256,
        min_eth_out: U256,
    ) -> Result<TypedTransaction, Box<dyn std::error::Error>> {
        info!("Building emergency sell for {} tokens", amount);
        
        // Build path: token -> WETH
        let path = vec![token_address, *routers::WETH];
        
        // Get deadline (10 minutes from now)
        let deadline = U256::from(chrono::Utc::now().timestamp() + 600);
        
        // Build function data for swapExactTokensForETH
        let function_data = self.encode_swap_exact_tokens_for_eth(
            amount,
            min_eth_out,
            path,
            self.wallet_address,
            deadline,
        )?;
        
        // Build transaction
        let tx = TransactionRequest::new()
            .to(self.router_address)
            .data(function_data)
            .from(self.wallet_address);
        
        // Estimate gas with 20% buffer
        let mut typed_tx: TypedTransaction = tx.into();
        let gas_estimate = self.provider.estimate_gas(&typed_tx, None).await?;
        let gas_limit = gas_estimate * 120 / 100; // 20% buffer
        
        debug!("Gas estimate: {}, using limit: {}", gas_estimate, gas_limit);
        typed_tx.set_gas(gas_limit);
        
        Ok(typed_tx)
    }
    
    /// Build partial sell transaction with custom parameters
    pub async fn build_partial_sell(
        &self,
        token_address: Address,
        amount: U256,
        min_eth_out: U256,
        custom_path: Option<Vec<Address>>,
    ) -> Result<TypedTransaction, Box<dyn std::error::Error>> {
        info!("Building partial sell for {} tokens", amount);
        
        // Use custom path or default token -> WETH
        let path = custom_path.unwrap_or_else(|| vec![token_address, *routers::WETH]);
        
        // Get deadline (5 minutes for partial sells)
        let deadline = U256::from(chrono::Utc::now().timestamp() + 300);
        
        // Build function data
        let function_data = self.encode_swap_exact_tokens_for_eth(
            amount,
            min_eth_out,
            path,
            self.wallet_address,
            deadline,
        )?;
        
        // Build transaction
        let tx = TransactionRequest::new()
            .to(self.router_address)
            .data(function_data)
            .from(self.wallet_address);
        
        // Estimate gas
        let mut typed_tx: TypedTransaction = tx.into();
        let gas_estimate = self.provider.estimate_gas(&typed_tx, None).await?;
        let gas_limit = gas_estimate * 115 / 100; // 15% buffer for partial sells
        typed_tx.set_gas(gas_limit);
        
        Ok(typed_tx)
    }
    
    /// Build token approval transaction
    pub fn build_token_approval(
        &self,
        token_address: Address,
        spender: Address,
        amount: U256,
    ) -> Result<TypedTransaction, Box<dyn std::error::Error>> {
        // ERC20 approve function signature
        let function_signature = "approve(address,uint256)";
        let function_selector = &ethers::utils::keccak256(function_signature.as_bytes())[0..4];
        
        // Encode parameters
        let encoded_params = encode(&[
            Token::Address(spender),
            Token::Uint(amount),
        ]);
        
        // Combine selector and params
        let mut data = function_selector.to_vec();
        data.extend_from_slice(&encoded_params);
        
        // Build transaction
        let tx = TransactionRequest::new()
            .to(token_address)
            .data(data)
            .from(self.wallet_address);
        
        Ok(tx.into())
    }
    
    /// Calculate minimum output with slippage
    pub fn calculate_min_output(&self, expected_output: U256) -> U256 {
        let slippage_factor = 1.0 - self.slippage_tolerance;
        let min_output = expected_output.as_u128() as f64 * slippage_factor;
        U256::from(min_output as u128)
    }
    
    /// Encode swapExactTokensForETH function call
    fn encode_swap_exact_tokens_for_eth(
        &self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Function signature for Uniswap V2
        let function_signature = "swapExactTokensForETH(uint256,uint256,address[],address,uint256)";
        let function_selector = &ethers::utils::keccak256(function_signature.as_bytes())[0..4];
        
        // Convert path to tokens
        let path_tokens: Vec<Token> = path.into_iter()
            .map(Token::Address)
            .collect();
        
        // Encode parameters
        let encoded_params = encode(&[
            Token::Uint(amount_in),
            Token::Uint(amount_out_min),
            Token::Array(path_tokens),
            Token::Address(to),
            Token::Uint(deadline),
        ]);
        
        // Combine selector and params
        let mut data = function_selector.to_vec();
        data.extend_from_slice(&encoded_params);
        
        Ok(data)
    }
    
    
    /// Get the provider
    pub fn provider(&self) -> &Arc<Provider<Http>> {
        &self.provider
    }
    
    /// Get the router address
    pub fn router_address(&self) -> Address {
        self.router_address
    }
    
    /// Get the wallet address
    pub fn wallet_address(&self) -> Address {
        self.wallet_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_slippage_calculation() {
        let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
        let builder = TransactionBuilder::new(
            Arc::new(provider),
            *routers::UNISWAP_V2_ROUTER,
            Address::zero(),
            0.05, // 5% slippage
        );
        
        let expected = U256::from(1000);
        let min_output = builder.calculate_min_output(expected);
        assert_eq!(min_output, U256::from(950)); // 95% of expected
    }
    
    #[test]
    fn test_router_addresses() {
        // Verify router addresses are valid
        assert_ne!(*routers::UNISWAP_V2_ROUTER, Address::zero());
        assert_ne!(*routers::WETH, Address::zero());
    }
}