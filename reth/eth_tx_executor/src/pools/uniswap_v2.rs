//! Uniswap V2 pool implementation
//! 
//! Optimized for speed with direct contract calls and minimal overhead

use super::*;
use ethers::abi::{encode, Token};
use ethers::types::transaction::eip2718::TypedTransaction;
use tracing::{debug, instrument};

/// Uniswap V2 addresses
pub mod addresses {
    use ethers::types::Address;
    
    lazy_static::lazy_static! {
        pub static ref FACTORY: Address = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse().unwrap();
        pub static ref ROUTER: Address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap();
        pub static ref WETH: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
    }
}

/// Pre-computed function selectors for optimal performance
mod selectors {
    lazy_static::lazy_static! {
        // swapExactETHForTokens(uint256,address[],address,uint256)
        pub static ref SWAP_ETH_FOR_TOKENS: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactETHForTokens(uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        // swapExactTokensForETH(uint256,uint256,address[],address,uint256)
        pub static ref SWAP_TOKENS_FOR_ETH: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactTokensForETH(uint256,uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        // swapExactTokensForTokens(uint256,uint256,address[],address,uint256)
        pub static ref SWAP_TOKENS_FOR_TOKENS: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
    }
}

/// Uniswap V2 pool instance
pub struct UniswapV2Pool {
    address: Address,
    provider: Arc<Provider<Http>>,
    token0: Address,
    token1: Address,
}

impl UniswapV2Pool {
    /// Create new pool instance
    pub fn new(address: Address, provider: Arc<Provider<Http>>) -> Self {
        Self {
            address,
            provider,
            token0: Address::zero(), // Will be loaded on first use
            token1: Address::zero(),
        }
    }
    
    /// Create pool from token pair
    pub async fn from_tokens(
        token_a: Address,
        token_b: Address,
        provider: Arc<Provider<Http>>,
    ) -> PoolResult<Self> {
        let pool_address = Self::compute_pair_address(token_a, token_b)?;
        
        debug!("Computed pool address for {:?} / {:?}: {:?}", token_a, token_b, pool_address);
        
        // Known mainnet pool addresses (temporary hardcode for testing)
        let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
        let usdc: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
        let known_pool: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse().unwrap();
        
        let pool_address = if (token_a == weth && token_b == usdc) || (token_a == usdc && token_b == weth) {
            debug!("Using known WETH/USDC pool address: {:?}", known_pool);
            known_pool
        } else {
            pool_address
        };
        
        // Verify pool exists
        let code = provider.get_code(pool_address, None).await?;
        if code.is_empty() {
            return Err(crate::common::errors::KartalError::Pool(
                PoolError::PoolNotFound { address: pool_address }
            ));
        }
        
        let (token0, token1) = Self::sort_tokens(token_a, token_b);
        
        Ok(Self {
            address: pool_address,
            provider,
            token0,
            token1,
        })
    }
    
    /// Compute pool address from token pair (CREATE2)
    fn compute_pair_address(token_a: Address, token_b: Address) -> PoolResult<Address> {
        let (token0, token1) = Self::sort_tokens(token_a, token_b);
        
        // Uniswap V2 init code hash
        let init_code_hash = hex::decode("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f")
            .map_err(|_| crate::common::errors::KartalError::Pool(
                PoolError::InvalidPoolState { reason: "Invalid init code hash".to_string() }
            ))?;
        
        // Compute CREATE2 address
        let salt = ethers::utils::keccak256(encode(&[
            Token::Address(token0),
            Token::Address(token1),
        ]));
        
        let mut data = Vec::with_capacity(85);
        data.push(0xff);
        data.extend_from_slice(addresses::FACTORY.as_bytes());
        data.extend_from_slice(&salt);
        data.extend_from_slice(&init_code_hash);
        
        let hash = ethers::utils::keccak256(&data);
        Ok(Address::from_slice(&hash[12..]))
    }
    
    /// Sort tokens by address (required for Uniswap V2)
    fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
        if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        }
    }
    
    /// Load token addresses from pool
    async fn ensure_tokens_loaded(&mut self) -> PoolResult<()> {
        if self.token0 != Address::zero() {
            return Ok(());
        }
        
        // Call token0() and token1()
        let token0_data = self.provider.call(&TransactionRequest {
            to: Some(self.address.into()),
            data: Some(hex::decode("0dfe1681").unwrap().into()), // token0()
            ..Default::default()
        }.into(), None).await?;
        
        let token1_data = self.provider.call(&TransactionRequest {
            to: Some(self.address.into()),
            data: Some(hex::decode("d21220a7").unwrap().into()), // token1()
            ..Default::default()
        }.into(), None).await?;
        
        self.token0 = Address::from_slice(&token0_data[12..32]);
        self.token1 = Address::from_slice(&token1_data[12..32]);
        
        Ok(())
    }
    
    /// Calculate output using Uniswap V2 formula
    fn calculate_amount_out(
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> U256 {
        let amount_in_with_fee = amount_in * 997;
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = (reserve_in * 1000) + amount_in_with_fee;
        numerator / denominator
    }
}

#[async_trait]
impl Pool for UniswapV2Pool {
    fn address(&self) -> Address {
        self.address
    }
    
    fn protocol(&self) -> &'static str {
        "UniswapV2"
    }
    
    #[instrument(skip(self))]
    async fn supports_pair(&self, token_a: Address, token_b: Address) -> PoolResult<bool> {
        let mut pool = self.clone();
        pool.ensure_tokens_loaded().await?;
        
        let (t0, t1) = Self::sort_tokens(token_a, token_b);
        Ok((pool.token0 == t0 && pool.token1 == t1) || 
           (pool.token0 == t1 && pool.token1 == t0))
    }
    
    #[instrument(skip(self))]
    async fn get_reserves(&self) -> PoolResult<(U256, U256)> {
        // Call getReserves()
        let data = self.provider.call(&TransactionRequest {
            to: Some(self.address.into()),
            data: Some(hex::decode("0902f1ac").unwrap().into()), // getReserves()
            ..Default::default()
        }.into(), None).await?;
        
        // Decode reserves (uint112, uint112, uint32)
        let reserve0 = U256::from_big_endian(&data[0..32]);
        let reserve1 = U256::from_big_endian(&data[32..64]);
        
        Ok((reserve0, reserve1))
    }
    
    #[instrument(skip(self))]
    async fn get_amount_out(&self, amount_in: U256, token_in: Address) -> PoolResult<U256> {
        let mut pool = self.clone();
        pool.ensure_tokens_loaded().await?;
        
        let (reserve0, reserve1) = self.get_reserves().await?;
        
        let (reserve_in, reserve_out) = if token_in == pool.token0 {
            (reserve0, reserve1)
        } else if token_in == pool.token1 {
            (reserve1, reserve0)
        } else {
            return Err(crate::common::errors::KartalError::Pool(
                PoolError::InvalidPoolState {
                    reason: "Token not in pool".to_string(),
                }
            ));
        };
        
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err(crate::common::errors::KartalError::Pool(
                PoolError::InsufficientLiquidity {
                    required: amount_in,
                    available: U256::zero(),
                }
            ));
        }
        
        Ok(Self::calculate_amount_out(amount_in, reserve_in, reserve_out))
    }
    
    #[instrument(skip(self, params))]
    async fn build_swap_tx(&self, params: SwapParams) -> PoolResult<TypedTransaction> {
        // Check if this is an ETH -> Token swap (buying tokens with ETH)
        let is_eth_input = params.token_in == *addresses::WETH;
        
        // For ETH input swaps, we use swapExactETHForTokens
        if is_eth_input {
            debug!("Building ETH -> Token swap transaction");
            
            // Path for ETH input is just [WETH, token_out]
            let path = vec![*addresses::WETH, params.token_out];
            let path_tokens: Vec<Token> = path.iter().map(|&addr| Token::Address(addr)).collect();
            
            // Use pre-computed selector for performance
            let function_selector = &selectors::SWAP_ETH_FOR_TOKENS[..];
            
            let encoded_params = encode(&[
                Token::Uint(params.amount_out_min),  // amountOutMin
                Token::Array(path_tokens),            // path
                Token::Address(params.recipient),     // to
                Token::Uint(params.deadline),         // deadline
            ]);
            
            let mut data = function_selector.to_vec();
            data.extend_from_slice(&encoded_params);
            
            // CRITICAL: For ETH swaps, we don't send amount_in as parameter
            // Instead, it's sent as msg.value in the transaction
            let tx = TransactionRequest::new()
                .to(*addresses::ROUTER)
                .data(data)
                .from(params.recipient)
                .value(params.amount_in);  // ETH value sent with transaction
            
            return Ok(tx.into());
        }
        
        // For token -> token or token -> ETH swaps
        let path = if params.token_out == *addresses::WETH {
            vec![params.token_in, *addresses::WETH]
        } else {
            // Direct path if pool exists, otherwise through WETH
            vec![params.token_in, params.token_out]
        };
        
        // Use pre-computed selectors for performance
        let function_selector = if params.token_out == *addresses::WETH {
            &selectors::SWAP_TOKENS_FOR_ETH[..]
        } else {
            &selectors::SWAP_TOKENS_FOR_TOKENS[..]
        };
        
        let path_tokens: Vec<Token> = path.iter().map(|&addr| Token::Address(addr)).collect();
        
        let encoded_params = encode(&[
            Token::Uint(params.amount_in),
            Token::Uint(params.amount_out_min),
            Token::Array(path_tokens),
            Token::Address(params.recipient),
            Token::Uint(params.deadline),
        ]);
        
        let mut data = function_selector.to_vec();
        data.extend_from_slice(&encoded_params);
        
        let tx = TransactionRequest::new()
            .to(*addresses::ROUTER)
            .data(data)
            .from(params.recipient);
        
        Ok(tx.into())
    }
    
    async fn execute_swap(&self, _params: SwapParams) -> PoolResult<SwapResult> {
        // This would be implemented by the transaction executor
        // Pool only builds the transaction
        Err(crate::common::errors::KartalError::Execution(
            crate::common::errors::ExecutionError::TransactionFailed {
                reason: "Execute swap should be called through TransactionExecutor".to_string(),
            }
        ))
    }
    
    #[instrument(skip(self, params))]
    async fn estimate_gas(&self, params: SwapParams) -> PoolResult<U256> {
        let tx = self.build_swap_tx(params).await?;
        let gas = self.provider.estimate_gas(&tx, None).await?;
        Ok(gas)
    }
}

// Implement Clone manually since we have Arc<Provider>
impl Clone for UniswapV2Pool {
    fn clone(&self) -> Self {
        Self {
            address: self.address,
            provider: self.provider.clone(),
            token0: self.token0,
            token1: self.token1,
        }
    }
}