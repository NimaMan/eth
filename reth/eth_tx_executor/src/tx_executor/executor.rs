//! Transaction executor with performance metrics
//! 
//! Executes transactions with detailed latency tracking

use crate::alert_processor::{Alert, Action};
use crate::pools::{PoolFactory, SwapParams};
use crate::ranking::{TransactionRankingSystem, ExecutionPath};
use crate::wallet::{PositionTracker, SecureWallet, SecureWalletConfig};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use std::time::Instant;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{info, error, instrument};

/// Executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Path to keystore file
    pub keystore_path: PathBuf,
    /// Chain ID (1 for mainnet)
    pub chain_id: u64,
    /// RPC endpoint
    pub rpc_url: String,
    /// Enable flashbots for critical priority
    pub flashbots_enabled: bool,
    /// Flashbots RPC endpoint
    pub flashbots_rpc: Option<String>,
    /// Reth WebSocket URL for mempool monitoring
    pub reth_ws_url: String,
}

/// Execution result with detailed metrics
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Alert ID that triggered execution
    pub alert_id: String,
    /// Transaction hash if submitted
    pub tx_hash: Option<H256>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Performance metrics
    pub metrics: ExecutionMetrics,
}

/// Detailed execution metrics
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// Time from alert receipt to execution start
    pub alert_to_start_ms: u64,
    /// Time to check position
    pub position_check_ms: u64,
    /// Time to calculate optimal gas ranking
    pub gas_ranking_ms: u64,
    /// Time to get price quote
    pub price_quote_ms: u64,
    /// Time to build transaction
    pub tx_build_ms: u64,
    /// Time to submit transaction
    pub tx_submit_ms: u64,
    /// Total execution time
    pub total_ms: u64,
}

/// High-performance transaction executor
pub struct TransactionExecutor {
    /// Ethereum provider
    provider: Arc<Provider<Http>>,
    /// Secure wallet for signing
    wallet: Arc<SecureWallet>,
    /// Pool factory
    pool_factory: PoolFactory,
    /// Position tracker
    position_tracker: Arc<PositionTracker>,
    /// Transaction ranking system
    ranking_system: Arc<TransactionRankingSystem>,
    /// Nonce manager
    nonce: Arc<RwLock<U256>>,
    /// Configuration
    config: ExecutorConfig,
}

impl TransactionExecutor {
    /// Create new executor
    pub async fn new(config: ExecutorConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Arc::new(Provider::<Http>::try_from(&config.rpc_url)?);
        
        // Load secure wallet from keystore
        let wallet_config = SecureWalletConfig {
            keystore_path: config.keystore_path.clone(),
            chain_id: config.chain_id,
        };
        let wallet = Arc::new(SecureWallet::from_keystore(wallet_config).await?);
        
        let wallet_address = wallet.address();
        
        let pool_factory = PoolFactory::new(provider.clone());
        let position_tracker = Arc::new(PositionTracker::new(provider.clone(), wallet_address)?);
        
        // Initialize ranking system
        let ranking_system = Arc::new(TransactionRankingSystem::new(config.reth_ws_url.clone()).await?);
        
        // Start ranking system background services
        ranking_system.start().await?;
        
        // Initialize nonce
        let current_nonce = provider.get_transaction_count(wallet_address, None).await?;
        let nonce = Arc::new(RwLock::new(current_nonce));
        
        Ok(Self {
            provider,
            wallet,
            pool_factory,
            position_tracker,
            ranking_system,
            nonce,
            config,
        })
    }
    
    /// Execute alert with performance tracking
    #[instrument(skip(self), fields(alert_id = %alert.id))]
    pub async fn execute_alert(&self, alert: Alert) -> ExecutionResult {
        let start_time = Instant::now();
        let mut metrics = ExecutionMetrics {
            alert_to_start_ms: 0,
            position_check_ms: 0,
            gas_ranking_ms: 0,
            price_quote_ms: 0,
            tx_build_ms: 0,
            tx_submit_ms: 0,
            total_ms: 0,
        };
        
        // Validate alert is still valid
        if !alert.is_valid() {
            return ExecutionResult {
                alert_id: alert.id,
                tx_hash: None,
                success: false,
                error: Some("Alert expired".to_string()),
                metrics,
            };
        }
        
        // Execute based on action
        let result = match alert.action {
            Action::Sell => self.execute_sell(alert.clone(), &mut metrics).await,
            Action::Buy => self.execute_buy(alert.clone(), &mut metrics).await,
            _ => Err("Action not implemented".into()),
        };
        
        metrics.total_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(tx_hash) => {
                info!("Execution successful: {:?} in {}ms", tx_hash, metrics.total_ms);
                ExecutionResult {
                    alert_id: alert.id,
                    tx_hash: Some(tx_hash),
                    success: true,
                    error: None,
                    metrics,
                }
            }
            Err(e) => {
                error!("Execution failed: {} in {}ms", e, metrics.total_ms);
                ExecutionResult {
                    alert_id: alert.id,
                    tx_hash: None,
                    success: false,
                    error: Some(e.to_string()),
                    metrics,
                }
            }
        }
    }
    
    /// Execute sell order
    async fn execute_sell(
        &self,
        alert: Alert,
        metrics: &mut ExecutionMetrics,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        let checkpoint = Instant::now();
        
        // 1. Check position
        let position = self.position_tracker.get_position(alert.token_address).await?;
        metrics.position_check_ms = checkpoint.elapsed().as_millis() as u64;
        
        if position.balance.is_zero() {
            return Err("No tokens to sell".into());
        }
        
        // Determine amount to sell
        let amount_to_sell = if alert.params.amount == U256::MAX {
            position.balance // Sell all
        } else {
            alert.params.amount.min(position.balance)
        };
        
        let checkpoint = Instant::now();
        
        // 2. Calculate optimal gas price using ranking system
        let ranking_result = self.ranking_system.calculate_ranking(&alert).await?;
        metrics.gas_ranking_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Ranking result: gas_price={}, position={}, confidence={:.2}", 
            ranking_result.optimal_gas_price, 
            ranking_result.expected_position, 
            ranking_result.confidence);
        
        let checkpoint = Instant::now();
        
        // 3. Get pool and quote
        let pool = self.pool_factory.find_best_pool(
            alert.token_address,
            *crate::pools::uniswap_v2::addresses::WETH,
        ).await?;
        
        let amount_out = pool.get_amount_out(amount_to_sell, alert.token_address).await?;
        let min_amount_out = self.apply_slippage(amount_out, alert.params.slippage);
        
        metrics.price_quote_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 4. Build swap parameters
        let swap_params = SwapParams {
            token_in: alert.token_address,
            token_out: *crate::pools::uniswap_v2::addresses::WETH,
            amount_in: amount_to_sell,
            amount_out_min: min_amount_out,
            recipient: self.wallet.address(),
            deadline: alert.deadline_timestamp(),
        };
        
        // 5. Build transaction
        let mut tx = pool.build_swap_tx(swap_params).await?;
        
        // Set optimal gas price from ranking system
        tx.set_gas_price(ranking_result.optimal_gas_price);
        
        // Set nonce
        let nonce = self.get_next_nonce().await;
        tx.set_nonce(nonce);
        
        // Set from address
        tx.set_from(self.wallet.address());
        
        metrics.tx_build_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 6. Submit using execution path from ranking
        let tx_hash = self.submit_transaction(tx, &ranking_result.execution_path).await?;
        
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Sell transaction submitted: {:?}", tx_hash);
        
        Ok(tx_hash)
    }
    
    /// Execute buy order
    async fn execute_buy(
        &self,
        alert: Alert,
        metrics: &mut ExecutionMetrics,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        let checkpoint = Instant::now();
        
        // 1. Check ETH balance for purchase
        let eth_balance = self.provider.get_balance(self.wallet.address(), None).await?;
        metrics.position_check_ms = checkpoint.elapsed().as_millis() as u64;
        
        if eth_balance.is_zero() {
            return Err("No ETH to buy tokens".into());
        }
        
        // Determine amount of ETH to spend
        let eth_to_spend = if alert.params.amount == U256::MAX {
            // Use 90% of ETH balance (leave some for gas)
            eth_balance * 90 / 100
        } else {
            alert.params.amount.min(eth_balance)
        };
        
        if eth_to_spend.is_zero() {
            return Err("Insufficient ETH for purchase".into());
        }
        
        let checkpoint = Instant::now();
        
        // 2. Calculate optimal gas price using ranking system
        let ranking_result = self.ranking_system.calculate_ranking(&alert).await?;
        metrics.gas_ranking_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Ranking result: gas_price={}, position={}, confidence={:.2}", 
            ranking_result.optimal_gas_price, 
            ranking_result.expected_position, 
            ranking_result.confidence);
        
        let checkpoint = Instant::now();
        
        // 3. Get pool and quote (ETH -> Token)
        let pool = self.pool_factory.find_best_pool(
            *crate::pools::uniswap_v2::addresses::WETH,
            alert.token_address,
        ).await?;
        
        let tokens_out = pool.get_amount_out(eth_to_spend, *crate::pools::uniswap_v2::addresses::WETH).await?;
        let min_tokens_out = self.apply_slippage(tokens_out, alert.params.slippage);
        
        metrics.price_quote_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 4. Build swap parameters (ETH -> Token)
        let swap_params = SwapParams {
            token_in: *crate::pools::uniswap_v2::addresses::WETH,
            token_out: alert.token_address,
            amount_in: eth_to_spend,
            amount_out_min: min_tokens_out,
            recipient: self.wallet.address(),
            deadline: alert.deadline_timestamp(),
        };
        
        // 5. Build transaction
        let mut tx = pool.build_swap_tx(swap_params).await?;
        
        // Set value for ETH purchase (need to send ETH with transaction)
        tx.set_value(eth_to_spend);
        
        // Set optimal gas price from ranking system
        tx.set_gas_price(ranking_result.optimal_gas_price);
        
        // Set nonce
        let nonce = self.get_next_nonce().await;
        tx.set_nonce(nonce);
        
        // Set from address
        tx.set_from(self.wallet.address());
        
        // Set from address
        tx.set_from(self.wallet.address());
        
        metrics.tx_build_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 6. Submit using execution path from ranking
        let tx_hash = self.submit_transaction(tx, &ranking_result.execution_path).await?;
        
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Buy transaction submitted: {:?}", tx_hash);
        
        Ok(tx_hash)
    }
    
    /// Apply slippage to amount
    fn apply_slippage(&self, amount: U256, slippage: f64) -> U256 {
        let factor = 1.0 - slippage;
        let adjusted = amount.as_u128() as f64 * factor;
        U256::from(adjusted as u128)
    }
    
    /// Submit transaction using specified execution path
    async fn submit_transaction(
        &self,
        tx: TypedTransaction,
        execution_path: &ExecutionPath,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        // Ensure wallet is unlocked before signing
        if !self.wallet.is_unlocked().await {
            return Err("Wallet is locked - please unlock before executing transactions".into());
        }
        
        match execution_path {
            ExecutionPath::PublicMempool => {
                // Standard mempool submission
                let signature = self.wallet.sign_transaction(&tx).await?;
                let raw_tx = tx.rlp_signed(&signature);
                let pending_tx = self.provider.send_raw_transaction(raw_tx).await?;
                Ok(pending_tx.tx_hash())
            }
            ExecutionPath::FlashbotsBundle { max_block_number: _ } => {
                // TODO: Implement Flashbots submission
                info!("Flashbots submission not yet implemented, falling back to public mempool");
                let signature = self.wallet.sign_transaction(&tx).await?;
                let raw_tx = tx.rlp_signed(&signature);
                let pending_tx = self.provider.send_raw_transaction(raw_tx).await?;
                Ok(pending_tx.tx_hash())
            }
            ExecutionPath::MultiPath { timeout_ms: _ } => {
                // Try public mempool first, fallback to Flashbots
                let signature = self.wallet.sign_transaction(&tx).await?;
                let raw_tx = tx.rlp_signed(&signature);
                let pending_tx = self.provider.send_raw_transaction(raw_tx).await?;
                Ok(pending_tx.tx_hash())
            }
        }
    }
    
    /// Get next nonce atomically
    async fn get_next_nonce(&self) -> U256 {
        let mut nonce = self.nonce.write().await;
        let current = *nonce;
        *nonce = current + 1;
        current
    }
    
    /// Reset nonce from chain
    pub async fn reset_nonce(&self) -> Result<(), Box<dyn std::error::Error>> {
        let current_nonce = self.provider
            .get_transaction_count(self.wallet.address(), None)
            .await?;
        
        let mut nonce = self.nonce.write().await;
        *nonce = current_nonce;
        
        Ok(())
    }
    
    /// Unlock wallet with password
    pub async fn unlock_wallet(&self, password: secrecy::Secret<String>) -> Result<(), Box<dyn std::error::Error>> {
        self.wallet.unlock(password).await?;
        info!("Wallet unlocked for address: {}", self.wallet.address());
        Ok(())
    }
    
    /// Lock wallet (clear from memory)
    pub async fn lock_wallet(&self) {
        self.wallet.lock().await;
        info!("Wallet locked");
    }
    
    /// Check if wallet is unlocked
    pub async fn is_wallet_unlocked(&self) -> bool {
        self.wallet.is_unlocked().await
    }
}