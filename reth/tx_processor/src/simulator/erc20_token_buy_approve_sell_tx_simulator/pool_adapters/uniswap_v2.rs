/// Uniswap V2 pool adapter

use super::PoolAdapter;
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use tx_simulator::UnsignedTransaction;

pub struct UniswapV2Adapter {
    pool_address: Address,
    router_address: Address,
    gas_limit: u64,
    gas_price: u128,
}

impl UniswapV2Adapter {
    pub fn new(pool_address: Address) -> Self {
        Self {
            pool_address,
            router_address: Address::from([
                0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 
                0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6, 
                0x59, 0xF2, 0x48, 0x8D
            ]),
            gas_limit: 500_000, // Increased for complex token logic
            gas_price: 100_000_000_000,
        }
    }
    
    /// Get WETH address (mainnet)
    fn weth_address(&self) -> Address {
        Address::from([
            0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D,
            0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
            0x3C, 0x75, 0x6C, 0xc2
        ])
    }
    
    /// Encode swapExactETHForTokens
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
        data.extend_from_slice(self.weth_address().as_slice());
        
        // Token address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(path[1].as_slice());
        
        Bytes::from(data)
    }
    
    /// Encode swapExactTokensForETH
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
        data.extend_from_slice(self.weth_address().as_slice());
        
        Bytes::from(data)
    }
    
    /// Encode ERC20 approve
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

impl PoolAdapter for UniswapV2Adapter {
    fn build_buy_transaction(
        &self,
        token_address: Address,
        eth_amount: U256,
        buyer_address: Address,
        _slippage: f64, // Unused for now - simplified implementation
    ) -> Result<UnsignedTransaction> {
        // Calculate minimum output with slippage
        // For simplicity, we accept any amount (0) since we're testing
        let amount_out_min = U256::ZERO;
        
        let calldata = self.encode_swap_exact_eth_for_tokens(
            amount_out_min,
            vec![self.weth_address(), token_address],
            buyer_address,
            U256::from(u64::MAX), // Far future deadline
        );
        
        Ok(UnsignedTransaction {
            from: Some(buyer_address),
            to: Some(self.router_address),
            value: Some(eth_amount),
            data: Some(calldata),
            gas: Some(self.gas_limit),
            gas_price: Some(self.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    fn build_approve_transaction(
        &self,
        token_address: Address,
        amount: U256,
        buyer_address: Address,
    ) -> Result<UnsignedTransaction> {
        let calldata = self.encode_approve(self.router_address, amount);
        
        Ok(UnsignedTransaction {
            from: Some(buyer_address),
            to: Some(token_address),
            value: Some(U256::ZERO),
            data: Some(calldata),
            gas: Some(self.gas_limit),
            gas_price: Some(self.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    fn build_sell_transaction(
        &self,
        token_address: Address,
        token_amount: U256,
        buyer_address: Address,
        _slippage: f64, // Unused for now - simplified implementation
    ) -> Result<UnsignedTransaction> {
        // Calculate minimum output with slippage
        // For simplicity, we accept any amount (0) since we're testing
        let amount_out_min = U256::ZERO;
        
        let calldata = self.encode_swap_exact_tokens_for_eth(
            token_amount,
            amount_out_min,
            vec![token_address, self.weth_address()],
            buyer_address,
            U256::from(u64::MAX), // Far future deadline
        );
        
        Ok(UnsignedTransaction {
            from: Some(buyer_address),
            to: Some(self.router_address),
            value: Some(U256::ZERO),
            data: Some(calldata),
            gas: Some(self.gas_limit),
            gas_price: Some(self.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        })
    }
    
    fn router_address(&self) -> Address {
        self.router_address
    }
}