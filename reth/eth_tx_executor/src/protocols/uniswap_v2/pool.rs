//! Uniswap V2 pool implementation

use crate::common::{Result, TokenAddress, TokenAmount, PoolAddress, utils};
use crate::protocols::traits::{DexPool, PoolInfo, SwapQuote, SwapRoute};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::abi::{encode, Token};
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::instrument;

use super::{addresses, constants, selectors};

/// Uniswap V2 pool instance
#[derive(Debug, Clone)]
pub struct UniswapV2Pool {
    info: PoolInfo,
    provider: Arc<Provider<Http>>,
}

impl UniswapV2Pool {
    /// Create new pool instance
    pub fn new(
        address: PoolAddress,
        token_a: TokenAddress,
        token_b: TokenAddress,
        provider: Arc<Provider<Http>>,
    ) -> Self {
        // Sort tokens to match pool ordering
        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        
        let info = PoolInfo {
            address,
            protocol: "UniswapV2".to_string(),
            token_a: token0,
            token_b: token1,
            fee_bps: constants::FEE_BPS,
            reserves: None,
            tvl_usd: None,
        };
        
        Self { info, provider }
    }
    
    /// Check if this is an ETH swap (involves WETH)
    fn involves_eth(&self, token_in: TokenAddress, token_out: TokenAddress) -> bool {
        token_in == *addresses::WETH || token_out == *addresses::WETH
    }
}

#[async_trait]
impl DexPool for UniswapV2Pool {
    fn info(&self) -> &PoolInfo {
        &self.info
    }
    
    #[instrument(skip(self))]
    async fn get_reserves(&self) -> Result<(U256, U256)> {
        // Call getReserves() on the pool
        let call_data = selectors::GET_RESERVES.to_vec();
        
        let tx = TransactionRequest::new()
            .to(self.info.address)
            .data(call_data);
        
        let result = self.provider.call(&tx.into(), None).await?;
        
        // Decode reserves (uint112, uint112, uint32)
        if result.len() < 64 {
            return Err(crate::common::errors::KartalError::Pool(
                crate::common::errors::PoolError::InvalidPoolState {
                    reason: "Invalid reserves response".to_string(),
                }
            ));
        }
        
        let reserve0 = U256::from_big_endian(&result[0..32]);
        let reserve1 = U256::from_big_endian(&result[32..64]);
        
        Ok((reserve0, reserve1))
    }
    
    #[instrument(skip(self))]
    async fn get_swap_quote(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        amount_in: TokenAmount,
    ) -> Result<SwapQuote> {
        let (reserve0, reserve1) = self.get_reserves().await?;
        
        // Determine which reserve corresponds to which token
        let (reserve_in, reserve_out) = if token_in == self.info.token_a {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };
        
        // Calculate output amount
        let amount_out = super::calculate_amount_out(amount_in, reserve_in, reserve_out);
        
        // Calculate price impact
        let price_impact = utils::calculate_price_impact(
            amount_in,
            amount_out,
            reserve_in,
            reserve_out,
        );
        
        // Apply standard slippage for min amount
        let amount_out_min = utils::apply_slippage(amount_out, 0.01); // 1% slippage
        
        // Estimate gas
        let gas_estimate = if self.involves_eth(token_in, token_out) {
            U256::from(150_000) // ETH swaps
        } else {
            U256::from(200_000) // Token-to-token swaps
        };
        
        // Get current gas price
        let gas_price = self.provider.get_gas_price().await?;
        
        Ok(SwapQuote {
            amount_out,
            amount_out_min,
            price_impact,
            gas_estimate,
            gas_price,
            route: SwapRoute {
                path: vec![token_in, token_out],
                pools: vec![self.info.address],
                fees: vec![constants::FEE_BPS],
            },
            metadata: None,
        })
    }
    
    #[instrument(skip(self))]
    async fn build_swap_transaction(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        amount_in: TokenAmount,
        amount_out_min: TokenAmount,
        recipient: Address,
        deadline: U256,
    ) -> Result<TypedTransaction> {
        let is_eth_input = token_in == *addresses::WETH;
        let is_eth_output = token_out == *addresses::WETH;
        
        let (selector, data, value) = if is_eth_input {
            // swapExactETHForTokens
            let path = vec![*addresses::WETH, token_out];
            let data = encode(&[
                Token::Uint(amount_out_min),
                Token::Array(path.into_iter().map(Token::Address).collect()),
                Token::Address(recipient),
                Token::Uint(deadline),
            ]);
            
            (selectors::SWAP_ETH_FOR_TOKENS.as_ref(), data, amount_in)
        } else if is_eth_output {
            // swapExactTokensForETH
            let path = vec![token_in, *addresses::WETH];
            let data = encode(&[
                Token::Uint(amount_in),
                Token::Uint(amount_out_min),
                Token::Array(path.into_iter().map(Token::Address).collect()),
                Token::Address(recipient),
                Token::Uint(deadline),
            ]);
            
            (selectors::SWAP_TOKENS_FOR_ETH.as_ref(), data, U256::zero())
        } else {
            // swapExactTokensForTokens
            let path = vec![token_in, token_out];
            let data = encode(&[
                Token::Uint(amount_in),
                Token::Uint(amount_out_min),
                Token::Array(path.into_iter().map(Token::Address).collect()),
                Token::Address(recipient),
                Token::Uint(deadline),
            ]);
            
            (selectors::SWAP_TOKENS_FOR_TOKENS.as_ref(), data, U256::zero())
        };
        
        // Build transaction
        let mut call_data = Vec::with_capacity(4 + data.len());
        call_data.extend_from_slice(selector);
        call_data.extend(data);
        
        let tx = TransactionRequest::new()
            .to(*addresses::ROUTER)
            .data(call_data)
            .value(value);
        
        Ok(tx.into())
    }
    
    async fn estimate_swap_gas(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        _amount_in: TokenAmount,
    ) -> Result<U256> {
        // Return pre-calculated estimates based on swap type
        if self.involves_eth(token_in, token_out) {
            Ok(U256::from(150_000))
        } else {
            Ok(U256::from(200_000))
        }
    }
    
    fn router_address(&self) -> Address {
        *addresses::ROUTER
    }
    
    fn clone_box(&self) -> Box<dyn DexPool> {
        Box::new(self.clone())
    }
}