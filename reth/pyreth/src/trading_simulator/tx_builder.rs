/// Transaction Builder for Buy/Sell/Approve sequences
/// 
/// Builds CallRequest objects for Uniswap V2 operations

use eyre::Result;
use alloy_primitives::{Address, Bytes, U256};
use reth_tx_simulator::CallRequest;
use super::config::BuySellConfig;

/// Transaction builder for buy/sell sequences
pub struct TransactionBuilder {
    config: BuySellConfig,
}

impl TransactionBuilder {
    /// Create new transaction builder
    pub fn new(config: BuySellConfig) -> Self {
        Self { config }
    }
    
    /// Build buy transaction (swapExactETHForTokens)
    pub fn build_buy_transaction(
        &self,
        token_address: Address,
        _pool_address: Address, // Not used for V2, but kept for V3 compatibility
        eth_amount: U256,
    ) -> Result<CallRequest> {
        let calldata = self.encode_swap_exact_eth_for_tokens(
            U256::ZERO, // amountOutMin - accept any amount
            vec![self.config.weth_address, token_address],
            self.config.buyer_address,
            self.get_deadline(),
        );
        
        Ok(CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(self.config.router_address),
            value: Some(eth_amount),
            data: Some(calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    /// Build approve transaction for router
    pub fn build_approve_transaction(
        &self,
        token_address: Address,
        spender: Address,
    ) -> Result<CallRequest> {
        let calldata = self.encode_approve(
            spender,
            U256::MAX, // Approve max amount
        );
        
        Ok(CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(token_address),
            value: Some(U256::ZERO),
            data: Some(calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    /// Build sell transaction (swapExactTokensForETH)
    pub fn build_sell_transaction(
        &self,
        token_address: Address,
        _pool_address: Address,
        token_amount: Option<U256>,
    ) -> Result<CallRequest> {
        // If no amount specified, use a large amount (will be limited by balance)
        let amount = token_amount.unwrap_or(U256::from(1_000_000_000_000_000_000u128));
        
        let calldata = self.encode_swap_exact_tokens_for_eth(
            amount,
            U256::ZERO, // amountOutMin - accept any amount
            vec![token_address, self.config.weth_address],
            self.config.buyer_address,
            self.get_deadline(),
        );
        
        Ok(CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(self.config.router_address),
            value: Some(U256::ZERO),
            data: Some(calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    /// Get deadline timestamp
    fn get_deadline(&self) -> U256 {
        // Use a large deadline for simulation
        U256::from(u64::MAX)
    }
    
    /// Encode swapExactETHForTokens function call
    fn encode_swap_exact_eth_for_tokens(
        &self,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Bytes {
        // Function selector: 0x7ff36ab5
        let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5];
        
        // amountOutMin
        let amount_bytes = amount_out_min.to_be_bytes::<32>();
        data.extend_from_slice(&amount_bytes);
        
        // path offset (dynamic array)
        data.extend_from_slice(&[0u8; 28]);
        data.extend_from_slice(&[0, 0, 0, 0x80]); // offset to path
        
        // to address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_slice());
        
        // deadline
        let deadline_bytes = deadline.to_be_bytes::<32>();
        data.extend_from_slice(&deadline_bytes);
        
        // path array
        data.extend_from_slice(&[0u8; 28]);
        data.extend_from_slice(&[0, 0, 0, 0x02]); // array length = 2
        
        // WETH address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(path[0].as_slice());
        
        // Token address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(path[1].as_slice());
        
        Bytes::from(data)
    }
    
    /// Encode swapExactTokensForETH function call
    fn encode_swap_exact_tokens_for_eth(
        &self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Bytes {
        // Function selector: 0x18cbafe5
        let mut data = vec![0x18, 0xcb, 0xaf, 0xe5];
        
        // amountIn
        let amount_bytes = amount_in.to_be_bytes::<32>();
        data.extend_from_slice(&amount_bytes);
        
        // amountOutMin
        let min_bytes = amount_out_min.to_be_bytes::<32>();
        data.extend_from_slice(&min_bytes);
        
        // path offset
        data.extend_from_slice(&[0u8; 28]);
        data.extend_from_slice(&[0, 0, 0, 0xa0]); // offset to path
        
        // to address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_slice());
        
        // deadline
        let deadline_bytes = deadline.to_be_bytes::<32>();
        data.extend_from_slice(&deadline_bytes);
        
        // path array
        data.extend_from_slice(&[0u8; 28]);
        data.extend_from_slice(&[0, 0, 0, 0x02]); // array length = 2
        
        // Token address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(path[0].as_slice());
        
        // WETH address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(path[1].as_slice());
        
        Bytes::from(data)
    }
    
    /// Encode approve function call
    fn encode_approve(&self, spender: Address, amount: U256) -> Bytes {
        // Function selector: 0x095ea7b3
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
        
        // spender address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_slice());
        
        // amount
        let amount_bytes = amount.to_be_bytes::<32>();
        data.extend_from_slice(&amount_bytes);
        
        Bytes::from(data)
    }
}