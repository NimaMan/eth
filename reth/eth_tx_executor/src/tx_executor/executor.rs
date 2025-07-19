//! Transaction executor with performance metrics
//! 
//! Executes transactions with detailed latency tracking

use crate::alert_processor::{Alert, Action, Priority};
use crate::common::{validate_slippage, validate_token_address, validate_pool_address};
use crate::flashbots::{FlashbotsClient, FlashbotsConfig, BundleBuilder, RelayEndpoint};
use crate::db_writers::TradeLogger;
use crate::pools::{PoolFactory, SwapParams};
use crate::gas_ranking::{GasRanking, ExecutionPath};
use crate::risk::{RiskManager, RiskConfig, RiskDecision};
use crate::tx_executor::NonceManager;
use crate::wallet::{PositionTracker, SecureWallet, SecureWalletConfig};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use tracing::{info, error, warn, instrument};

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
    /// Risk management configuration
    pub risk_config: RiskConfig,
    /// RabbitMQ URL for block processor data (optional)
    pub rabbitmq_url: Option<String>,
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
#[derive(Debug, Clone, serde::Serialize)]
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
    /// Gas ranking system
    gas_ranking: Arc<GasRanking>,
    /// Nonce manager
    nonce_manager: Arc<NonceManager>,
    /// Risk manager
    risk_manager: Arc<tokio::sync::Mutex<RiskManager>>,
    /// Trade logger
    trade_logger: Arc<TradeLogger>,
    /// Flashbots client (optional)
    flashbots_client: Option<Arc<FlashbotsClient>>,
}

impl TransactionExecutor {
    /// Create new executor
    pub async fn new(config: ExecutorConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Arc::new(Provider::<Http>::try_from(&config.rpc_url)?);
        
        // Load secure wallet from keystore
        let wallet_config = SecureWalletConfig {
            keystore_path: config.keystore_path.clone(),
            chain_id: config.chain_id,
            auto_lock_timeout: Some(Duration::from_secs(300)), // 5 minutes auto-lock
        };
        let wallet = Arc::new(SecureWallet::from_keystore(wallet_config).await?);
        
        let wallet_address = wallet.address();
        
        let pool_factory = PoolFactory::new(provider.clone());
        let position_tracker = Arc::new(PositionTracker::new(provider.clone(), wallet_address)?);
        
        // Initialize gas ranking system
        let gas_ranking = if let Some(rabbitmq_url) = config.rabbitmq_url.as_ref() {
            info!("Initializing gas ranking with RabbitMQ and RPC");
            Arc::new(GasRanking::new(rabbitmq_url, &config.rpc_url).await?)
        } else {
            return Err("RabbitMQ URL is required for gas ranking system".into());
        };
        
        // Initialize nonce manager
        let nonce_config = crate::tx_executor::nonce_manager::NonceManagerConfig::default();
        let nonce_manager = Arc::new(NonceManager::new(nonce_config, wallet_address, provider.clone()).await?);
        
        // Initialize risk manager
        let risk_manager = Arc::new(tokio::sync::Mutex::new(RiskManager::new(config.risk_config.clone())));
        
        // Initialize trade logger
        let database_url = std::env::var("DATABASE_URL").ok();
        let trade_logger = Arc::new(
            TradeLogger::new(database_url.as_deref(), wallet_address).await?
        );
        
        // Initialize Flashbots client if enabled
        let flashbots_client = if config.flashbots_enabled {
            let flashbots_config = FlashbotsConfig {
                relay_endpoints: vec![
                    RelayEndpoint::Flashbots,
                    // Add more relays as needed
                ],
                signer: Arc::new(crate::flashbots::BundleSigner::random()), // Should use proper key
                timeout: std::time::Duration::from_secs(5),
                simulate_before_submit: true,
                max_retries: 3,
                retry_delay: std::time::Duration::from_millis(100),
            };
            
            match FlashbotsClient::new(flashbots_config, provider.clone()) {
                Ok(client) => {
                    info!("Flashbots client initialized");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    error!("Failed to initialize Flashbots client: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            provider,
            wallet,
            pool_factory,
            position_tracker,
            gas_ranking,
            nonce_manager,
            risk_manager,
            trade_logger,
            flashbots_client,
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
        
        // Log alert received
        let signal_id = self.trade_logger.log_alert_received(&alert).await;
        
        // Validate inputs
        if let Err(e) = self.validate_alert_inputs(&alert) {
            let error_msg = format!("Invalid alert inputs: {}", e);
            self.trade_logger.log_execution_result(signal_id, &alert.id, &ExecutionResult {
                alert_id: alert.id.clone(),
                tx_hash: None,
                success: false,
                error: Some(error_msg.clone()),
                metrics: metrics.clone(),
            }).await;
            
            return ExecutionResult {
                alert_id: alert.id,
                tx_hash: None,
                success: false,
                error: Some(error_msg),
                metrics,
            };
        }
        
        // Validate alert is still valid
        if !alert.is_valid() {
            let result = ExecutionResult {
                alert_id: alert.id.clone(),
                tx_hash: None,
                success: false,
                error: Some("Alert expired".to_string()),
                metrics,
            };
            self.trade_logger.log_execution_result(signal_id, &alert.id, &result).await;
            return result;
        }
        
        // Execute based on action
        let result = match alert.action {
            Action::Sell => self.execute_sell(signal_id, alert.clone(), &mut metrics).await,
            Action::Buy => self.execute_buy(signal_id, alert.clone(), &mut metrics).await,
            Action::AddLiquidity => {
                // TODO: Implement liquidity operations
                return ExecutionResult {
                    alert_id: alert.id.clone(),
                    tx_hash: None,
                    success: false,
                    error: Some("AddLiquidity not yet implemented".to_string()),
                    metrics,
                };
            }
            Action::RemoveLiquidity => {
                // TODO: Implement liquidity operations
                return ExecutionResult {
                    alert_id: alert.id.clone(),
                    tx_hash: None,
                    success: false,
                    error: Some("RemoveLiquidity not yet implemented".to_string()),
                    metrics,
                };
            }
        };
        
        metrics.total_ms = start_time.elapsed().as_millis() as u64;
        
        let execution_result = match result {
            Ok(tx_hash) => {
                info!("Execution successful: {:?} in {}ms", tx_hash, metrics.total_ms);
                ExecutionResult {
                    alert_id: alert.id.clone(),
                    tx_hash: Some(tx_hash),
                    success: true,
                    error: None,
                    metrics,
                }
            }
            Err(e) => {
                error!("Execution failed: {} in {}ms", e, metrics.total_ms);
                ExecutionResult {
                    alert_id: alert.id.clone(),
                    tx_hash: None,
                    success: false,
                    error: Some(e.to_string()),
                    metrics,
                }
            }
        };
        
        // Log final execution result
        self.trade_logger.log_execution_result(signal_id, &alert.id, &execution_result).await;
        
        execution_result
    }
    
    /// Validate alert inputs
    fn validate_alert_inputs(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        // Validate token address
        validate_token_address(alert.token_address)?;
        
        // Validate pool address
        validate_pool_address(alert.pool_address)?;
        
        // Validate and normalize slippage
        let _validated_slippage = validate_slippage(alert.params.slippage)?;
        
        Ok(())
    }
    
    /// Execute sell order
    async fn execute_sell(
        &self,
        signal_id: uuid::Uuid,
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
        
        // 2. Calculate optimal gas price using gas ranking
        let gas_recommendation = match alert.params.priority {
            Priority::Critical => self.gas_ranking.get_gas_for_position(1), // Top priority
            Priority::High => self.gas_ranking.get_gas_for_position(10),    // High priority
            Priority::Normal => self.gas_ranking.get_gas_for_position(50),  // Normal priority
        };
        metrics.gas_ranking_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Gas recommendation: priority_fee={}, max_fee={}, position={}, confidence={:.2}", 
            gas_recommendation.priority_fee, 
            gas_recommendation.max_fee,
            gas_recommendation.expected_position, 
            gas_recommendation.confidence);
        
        let checkpoint = Instant::now();
        
        // 3. Get pool and quote
        let pool = self.pool_factory.find_best_pool(
            alert.token_address,
            *crate::pools::uniswap_v2::addresses::WETH,
        ).await?;
        
        let amount_out = pool.get_amount_out(amount_to_sell, alert.token_address).await?;
        let validated_slippage = validate_slippage(alert.params.slippage)?;
        let min_amount_out = self.apply_slippage(amount_out, validated_slippage);
        
        metrics.price_quote_ms = checkpoint.elapsed().as_millis() as u64;
        
        // 3.5. Risk check
        let trade_amount_eth = amount_out.as_u128() as f64 / 1e18;
        let risk_decision = {
            let risk_mgr = self.risk_manager.lock().await;
            risk_mgr.evaluate_trade_risk(&alert, trade_amount_eth, alert.token_address)
        };
        
        // Log risk decision
        self.trade_logger.log_risk_decision(
            signal_id,
            &alert.id,
            &risk_decision,
            amount_to_sell,
        ).await;
        
        match risk_decision {
            RiskDecision::Allow => {
                info!("Risk check passed for sell order");
            }
            RiskDecision::Block { reason } => {
                error!("Risk manager blocked trade: {}", reason);
                return Err(format!("Trade blocked by risk manager: {}", reason).into());
            }
        };
        
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
        tx.set_gas_price(gas_recommendation.max_fee);
        
        // Reserve nonce
        let nonce = self.nonce_manager.reserve_nonce().await?;
        tx.set_nonce(nonce);
        
        // Set from address
        tx.set_from(self.wallet.address());
        
        metrics.tx_build_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 6. Submit using execution path from ranking
        let tx_hash = self.submit_transaction(tx, &gas_recommendation.execution_path).await?;
        
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        // Log transaction submission
        self.trade_logger.log_tx_submitted(
            signal_id,
            &alert.id,
            tx_hash,
            nonce,
            gas_recommendation.max_fee,
            &format!("{:?}", gas_recommendation.execution_path),
        ).await;
        
        info!("Sell transaction submitted: {:?}", tx_hash);
        
        Ok(tx_hash)
    }
    
    /// Execute buy order
    async fn execute_buy(
        &self,
        signal_id: uuid::Uuid,
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
        
        // 2. Calculate optimal gas price using gas ranking
        let gas_recommendation = match alert.params.priority {
            Priority::Critical => self.gas_ranking.get_gas_for_position(1), // Top priority
            Priority::High => self.gas_ranking.get_gas_for_position(10),    // High priority
            Priority::Normal => self.gas_ranking.get_gas_for_position(50),  // Normal priority
        };
        metrics.gas_ranking_ms = checkpoint.elapsed().as_millis() as u64;
        
        info!("Gas recommendation: priority_fee={}, max_fee={}, position={}, confidence={:.2}", 
            gas_recommendation.priority_fee, 
            gas_recommendation.max_fee,
            gas_recommendation.expected_position, 
            gas_recommendation.confidence);
        
        let checkpoint = Instant::now();
        
        // 3. Get pool and quote (ETH -> Token)
        let pool = self.pool_factory.find_best_pool(
            *crate::pools::uniswap_v2::addresses::WETH,
            alert.token_address,
        ).await?;
        
        let tokens_out = pool.get_amount_out(eth_to_spend, *crate::pools::uniswap_v2::addresses::WETH).await?;
        let validated_slippage = validate_slippage(alert.params.slippage)?;
        let min_tokens_out = self.apply_slippage(tokens_out, validated_slippage);
        
        metrics.price_quote_ms = checkpoint.elapsed().as_millis() as u64;
        
        // 3.5. Risk check
        let trade_amount_eth = eth_to_spend.as_u128() as f64 / 1e18;
        let risk_decision = {
            let risk_mgr = self.risk_manager.lock().await;
            risk_mgr.evaluate_trade_risk(&alert, trade_amount_eth, alert.token_address)
        };
        
        // Log risk decision
        self.trade_logger.log_risk_decision(
            signal_id,
            &alert.id,
            &risk_decision,
            eth_to_spend,
        ).await;
        
        match risk_decision {
            RiskDecision::Allow => {
                info!("Risk check passed for buy order");
            }
            RiskDecision::Block { reason } => {
                error!("Risk manager blocked trade: {}", reason);
                return Err(format!("Trade blocked by risk manager: {}", reason).into());
            }
        };
        
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
        
        // Note: For ETH swaps, the value is already set in build_swap_tx
        // Only override if not already set
        if tx.value().is_none() || tx.value() == Some(&U256::zero()) {
            tx.set_value(eth_to_spend);
        }
        
        // Set optimal gas price from ranking system
        tx.set_gas_price(gas_recommendation.max_fee);
        
        // Set gas limit for optimal performance
        // For critical alerts, use pre-calculated safe gas limit to avoid estimation delay
        let gas_limit = if alert.params.priority == Priority::Critical {
            U256::from(300_000) // Safe gas limit for ETH->token swaps
        } else {
            // For non-critical, we can afford the ~10ms to estimate
            self.provider.estimate_gas(&tx, None).await.unwrap_or(U256::from(250_000))
        };
        tx.set_gas(gas_limit);
        
        // Reserve nonce
        let nonce = self.nonce_manager.reserve_nonce().await?;
        tx.set_nonce(nonce);
        
        // Set from address
        tx.set_from(self.wallet.address());
        
        metrics.tx_build_ms = checkpoint.elapsed().as_millis() as u64;
        
        let checkpoint = Instant::now();
        
        // 6. Submit using execution path from ranking
        let tx_hash = self.submit_transaction(tx, &gas_recommendation.execution_path).await?;
        
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        // Log transaction submission
        self.trade_logger.log_tx_submitted(
            signal_id,
            &alert.id,
            tx_hash,
            nonce,
            gas_recommendation.max_fee,
            &format!("{:?}", gas_recommendation.execution_path),
        ).await;
        
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
        
        // Get the nonce from the transaction
        let nonce = tx.nonce().ok_or("Transaction must have nonce set")?;
        
        match execution_path {
            ExecutionPath::Public => {
                // Standard mempool submission
                let signature = self.wallet.sign_transaction(&tx).await?;
                let raw_tx = tx.rlp_signed(&signature);
                
                match self.provider.send_raw_transaction(raw_tx).await {
                    Ok(pending_tx) => {
                        let tx_hash = pending_tx.tx_hash();
                        // Mark as submitted
                        self.nonce_manager.mark_submitted(*nonce, tx_hash).await;
                        Ok(tx_hash)
                    }
                    Err(e) => {
                        // Mark as failed
                        self.nonce_manager.mark_failed(*nonce, e.to_string()).await;
                        Err(e.into())
                    }
                }
            }
            ExecutionPath::Flashbots => {
                // Use Flashbots for critical transactions
                if let Some(flashbots) = &self.flashbots_client {
                    info!("Submitting transaction via Flashbots");
                    
                    // Sign transaction
                    let signature = self.wallet.sign_transaction(&tx).await?;
                    let signed_tx = tx.rlp_signed(&signature);
                    
                    // Get current block
                    let current_block = self.provider.get_block_number().await?.as_u64();
                    let target_block = current_block + 1; // Next block
                    
                    // Build bundle
                    let bundle = BundleBuilder::new()
                        .add_transaction(signed_tx.clone())
                        .block_number(target_block)
                        .time_window(12) // 12 seconds (1 block time)
                        .protect_transaction(tx.hash(&signature))
                        .build()?;
                    
                    // Submit bundle
                    match flashbots.submit_bundle(bundle).await? {
                        crate::flashbots::BundleResult::Included { block_hash, .. } => {
                            info!("Bundle included via Flashbots in block {:?}", block_hash);
                            let tx_hash = tx.hash(&signature);
                            self.nonce_manager.mark_submitted(*nonce, tx_hash).await;
                            Ok(tx_hash)
                        }
                        crate::flashbots::BundleResult::NotIncluded { reason } => {
                            warn!("Flashbots bundle not included: {:?}, falling back to public mempool", reason);
                            // Fallback to public mempool
                            match self.provider.send_raw_transaction(signed_tx).await {
                                Ok(pending_tx) => {
                                    let tx_hash = pending_tx.tx_hash();
                                    self.nonce_manager.mark_submitted(*nonce, tx_hash).await;
                                    Ok(tx_hash)
                                }
                                Err(e) => {
                                    self.nonce_manager.mark_failed(*nonce, e.to_string()).await;
                                    Err(e.into())
                                }
                            }
                        }
                        crate::flashbots::BundleResult::Failed { error } => {
                            error!("Flashbots submission failed: {}", error);
                            self.nonce_manager.mark_failed(*nonce, error.clone()).await;
                            return Err(error.into());
                        }
                    }
                } else {
                    warn!("Flashbots requested but not available, using public mempool");
                    let signature = self.wallet.sign_transaction(&tx).await?;
                    let raw_tx = tx.rlp_signed(&signature);
                    match self.provider.send_raw_transaction(raw_tx).await {
                        Ok(pending_tx) => {
                            let tx_hash = pending_tx.tx_hash();
                            self.nonce_manager.mark_submitted(*nonce, tx_hash).await;
                            Ok(tx_hash)
                        }
                        Err(e) => {
                            self.nonce_manager.mark_failed(*nonce, e.to_string()).await;
                            Err(e.into())
                        }
                    }
                }
            }
            ExecutionPath::DirectBuilder => {
                // Direct builder submission (future implementation)
                // For now, fallback to public mempool
                warn!("DirectBuilder not yet implemented, using public mempool");
                let signature = self.wallet.sign_transaction(&tx).await?;
                let raw_tx = tx.rlp_signed(&signature);
                
                match self.provider.send_raw_transaction(raw_tx).await {
                    Ok(pending_tx) => {
                        let tx_hash = pending_tx.tx_hash();
                        self.nonce_manager.mark_submitted(*nonce, tx_hash).await;
                        Ok(tx_hash)
                    }
                    Err(e) => {
                        self.nonce_manager.mark_failed(*nonce, e.to_string()).await;
                        Err(e.into())
                    }
                }
            }
        }
    }
    
    /// Sync nonce manager with chain
    pub async fn sync_nonce_with_chain(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.nonce_manager.sync_with_chain().await?;
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
    
    /* TODO: Implement liquidity operations
    // These functions are commented out due to:
    // 1. RankingResult field mismatches (execution_rank, estimated_arrival_ms don't exist)
    // 2. Missing self.tx_builder field
    // 3. Incomplete implementation
    
    /*
    /// Execute add liquidity action
    async fn execute_add_liquidity(
        &self,
        signal_id: Uuid,
        alert: Alert,
        metrics: &mut ExecutionMetrics,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        info!("Executing add liquidity for token {} on pool {}", 
              alert.token_address, alert.pool_address);
        
        let checkpoint = Instant::now();
        
        // 1. Get pool information
        let pool_info = self.pool_factory.get_pool_info(&alert.pool_address).await?;
        // Note: pool_query_ms metric removed - add to ExecutionMetrics if needed
        
        // 2. Get optimal amounts based on current pool state
        let target_liquidity = alert.params.amount;
        let (amount0, amount1) = match &pool_info {
            PoolInfo::UniswapV2(info) => {
                // Calculate proportional amounts for V2
                let total_supply = info.total_supply;
                
                if total_supply == U256::zero() {
                    // First liquidity provider
                    (target_liquidity, target_liquidity)
                } else {
                    // Calculate proportional amounts
                    let amount0 = target_liquidity * info.reserve0 / total_supply;
                    let amount1 = target_liquidity * info.reserve1 / total_supply;
                    (amount0, amount1)
                }
            }
            PoolInfo::UniswapV3(info) => {
                // For V3, use current price to calculate amounts
                // This is simplified - real implementation would consider tick ranges
                let sqrt_price = info.sqrt_price_x96;
                let amount0 = target_liquidity;
                let amount1 = target_liquidity * sqrt_price * sqrt_price / (U256::from(1) << 192);
                (amount0, amount1)
            }
            _ => return Err("Unsupported pool type for liquidity provision".into()),
        };
        
        // 3. Build transaction based on pool type
        let tx = match &pool_info {
            PoolInfo::UniswapV2(_) => {
                // Build V2 add liquidity transaction
                self.tx_builder.build_v2_add_liquidity(
                    alert.pool_address,
                    alert.token_address,
                    amount0,
                    amount1,
                    alert.params.slippage,
                    alert.params.deadline,
                )?
            }
            PoolInfo::UniswapV3(_) => {
                // Build V3 mint position transaction
                // Note: This would need tick range parameters
                return Err("V3 liquidity provision not yet implemented".into());
            }
            _ => return Err("Unsupported pool type".into()),
        };
        
        // 4. Execute transaction
        let checkpoint = Instant::now();
        // Note: gas_recommendation should be created for liquidity operations too
        // For now, using default public mempool path
        let tx_hash = self.submit_transaction(tx, &ExecutionPath::Public).await?;
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        Ok(tx_hash)
    }
    
    // End of commented liquidity functions */
}
        info!("Executing remove liquidity for token {} on pool {}", 
              alert.token_address, alert.pool_address);
        
        let checkpoint = Instant::now();
        
        // 1. Get pool information
        let pool_info = self.pool_factory.get_pool_info(&alert.pool_address).await?;
        // Note: pool_query_ms metric removed - add to ExecutionMetrics if needed
        
        // 2. Build transaction based on pool type
        let tx = match &pool_info {
            PoolInfo::UniswapV2(_) => {
                // Build V2 remove liquidity transaction
                self.tx_builder.build_v2_remove_liquidity(
                    alert.pool_address,
                    alert.token_address,
                    alert.params.amount, // LP token amount
                    alert.params.slippage,
                    alert.params.deadline,
                )?
            }
            PoolInfo::UniswapV3(_) => {
                // Build V3 burn position transaction
                // Note: This would need position NFT ID
                return Err("V3 liquidity removal not yet implemented".into());
            }
            _ => return Err("Unsupported pool type".into()),
        };
        
        // 3. Execute transaction
        let checkpoint = Instant::now();
        // Note: gas_recommendation should be created for liquidity operations too
        // For now, using default public mempool path
        let tx_hash = self.submit_transaction(tx, &ExecutionPath::Public).await?;
        metrics.tx_submit_ms = checkpoint.elapsed().as_millis() as u64;
        
        Ok(tx_hash)
    }
    */
}