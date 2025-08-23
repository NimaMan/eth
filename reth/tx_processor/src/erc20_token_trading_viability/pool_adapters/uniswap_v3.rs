/// Uniswap V3 pool adapter

use super::PoolAdapter;
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use reth_tx_simulator::CallRequest;

pub struct UniswapV3Adapter {
    pool_address: Address,
    router_address: Address,
    fee_tier: u32,
    gas_limit: u64,
    gas_price: u128,
}

impl UniswapV3Adapter {
    pub fn new(pool_address: Address, fee_tier: u32) -> Self {
        Self {
            pool_address,
            router_address: Address::from([
                0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D,
                0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15, 0x7C,
                0x05, 0x86, 0x15, 0x64
            ]),
            fee_tier, // 500, 3000, or 10000
            gas_limit: 350_000, // V3 needs more gas
            gas_price: 100_000_000_000,
        }
    }
    
    /// Encode exactInputSingle for V3
    fn encode_exact_input_single(
        &self,
        token_in: Address,
        token_out: Address,
        fee: u32,
        recipient: Address,
        deadline: U256,
        amount_in: U256,
        amount_out_minimum: U256,
        sqrt_price_limit_x96: U256,
    ) -> Bytes {
        // Function selector: 0x414bf389 for exactInputSingle
        let mut data = vec![0x41, 0x4b, 0xf3, 0x89];
        
        eprintln!("V3 Debug - Building exactInputSingle:");
        eprintln!("  token_in: {:?}", token_in);
        eprintln!("  token_out: {:?}", token_out);
        eprintln!("  fee: {}", fee);
        eprintln!("  amount_in: {}", amount_in);
        
        // Encode struct ExactInputSingleParams
        // tokenIn
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(token_in.as_slice());
        
        // tokenOut
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(token_out.as_slice());
        
        // fee (uint24, but padded to 32 bytes)
        data.extend_from_slice(&[0u8; 28]);
        data.extend_from_slice(&fee.to_be_bytes()[1..]); // Skip first byte for uint24
        
        // recipient
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(recipient.as_slice());
        
        // deadline
        let deadline_bytes = deadline.to_be_bytes::<32>();
        data.extend_from_slice(&deadline_bytes);
        
        // amountIn
        let amount_in_bytes = amount_in.to_be_bytes::<32>();
        data.extend_from_slice(&amount_in_bytes);
        
        // amountOutMinimum
        let amount_out_bytes = amount_out_minimum.to_be_bytes::<32>();
        data.extend_from_slice(&amount_out_bytes);
        
        // sqrtPriceLimitX96 (0 = no limit)
        let price_limit_bytes = sqrt_price_limit_x96.to_be_bytes::<32>();
        data.extend_from_slice(&price_limit_bytes);
        
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

impl PoolAdapter for UniswapV3Adapter {
    fn build_buy_transaction(
        &self,
        token_address: Address,
        eth_amount: U256,
        buyer_address: Address,
        _slippage: f64, // Unused for now - simplified implementation
    ) -> Result<CallRequest> {
        let calldata = self.encode_exact_input_single(
            self.weth_address(),  // tokenIn (WETH)
            token_address,         // tokenOut
            self.fee_tier,         // fee
            buyer_address,         // recipient
            U256::from(u64::MAX),  // deadline
            eth_amount,            // amountIn
            U256::ZERO,           // amountOutMinimum (accept any)
            U256::ZERO,           // sqrtPriceLimitX96 (no limit)
        );
        
        Ok(CallRequest {
            from: Some(buyer_address),
            to: Some(self.router_address),
            value: Some(eth_amount), // V3 router handles WETH wrapping
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
    ) -> Result<CallRequest> {
        let calldata = self.encode_approve(self.router_address, amount);
        
        Ok(CallRequest {
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
    ) -> Result<CallRequest> {
        let calldata = self.encode_exact_input_single(
            token_address,         // tokenIn
            self.weth_address(),   // tokenOut (WETH)
            self.fee_tier,         // fee
            buyer_address,         // recipient
            U256::from(u64::MAX),  // deadline
            token_amount,          // amountIn (exact tokens to sell)
            U256::ZERO,           // amountOutMinimum (accept any)
            U256::ZERO,           // sqrtPriceLimitX96 (no limit)
        );
        
        Ok(CallRequest {
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