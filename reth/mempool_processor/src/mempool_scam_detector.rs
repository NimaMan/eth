// Mempool Scam Detection System
// Combines transaction simulation with state change analysis to detect scams before block inclusion

use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use eyre::Result;
use tracing::{info, warn, debug};
use ethers::providers::{Provider, Http};

use crate::tx_simulator::DebugTraceCallSimulator;
use crate::mempool_fetcher::types::TransactionView;

#[derive(Debug, Clone)]
pub struct ScamAlert {
    pub tx_hash: H256,
    pub severity: ScamSeverity,
    pub scam_type: ScamType,
    pub affected_pool: Address,
    pub drain_amount_eth: f64,
    pub drain_percentage: f64,
    pub victim_count: usize,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScamSeverity {
    Critical,  // >80% drain
    High,      // 50-80% drain  
    Medium,    // 20-50% drain
    Low,       // <20% drain
}

#[derive(Debug, Clone)]
pub enum ScamType {
    RugPull,           // Owner draining liquidity
    HoneypotDrain,     // Draining a honeypot token pool
    FlashLoanAttack,   // Large drain via flash loan
    SandwichVictim,    // User being sandwiched
    Unknown,
}

pub struct MempoolScamDetector {
    watched_pools: HashMap<Address, PoolInfo>,
    weth_address: Address,
    provider: Arc<Provider<Http>>,
    rpc_url: String,
}

#[derive(Clone)]
pub struct PoolInfo {
    pub token0: Address,
    pub token1: Address,
    pub liquidity_eth: f64,  // Current ETH/WETH liquidity
    pub is_honeypot: bool,
}

impl MempoolScamDetector {
    pub async fn new(rpc_url: &str, _chain_id: u64) -> Result<Self> {
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?;
        
        Ok(Self {
            watched_pools: HashMap::new(),
            weth_address,
            provider,
            rpc_url: rpc_url.to_string(),
        })
    }
    
    /// Add a pool to watch for scams
    pub fn add_watched_pool(&mut self, pool: Address, info: PoolInfo) {
        self.watched_pools.insert(pool, info);
        info!("Added pool to scam watch: {:?}", pool);
    }
    
    /// Get pool info for a given pool address
    pub fn get_pool_info(&self, pool: &Address) -> Option<&PoolInfo> {
        self.watched_pools.get(pool)
    }
    
    /// Analyze a mempool transaction for potential scams
    pub async fn analyze_transaction(&self, tx: Transaction) -> Result<Option<ScamAlert>> {
        // Quick pre-filter: only analyze if involves watched addresses
        let involves_pool = self.watched_pools.keys().any(|&pool| {
            tx.to == Some(pool) || 
            self.might_affect_pool(&tx, pool)
        });
        
        if !involves_pool {
            return Ok(None);
        }
        
        debug!("Analyzing potentially suspicious transaction: {:?}", tx.hash);
        
        // Convert to TransactionView for simulator
        let tx_view = self.convert_to_tx_view(&tx);
        
        // Use DebugTraceCallSimulator to get state changes
        let debug_simulator = DebugTraceCallSimulator::new(&self.rpc_url).await?;
        
        // Note: DebugTraceCallSimulator doesn't need block_env, it uses "latest"
        match debug_simulator.process_transaction(&tx_view, &Default::default()).await {
            Ok(Some(state_changes)) => {
                // Convert HashMap<Address, CalculatedAccountChanges> to our MempoolStateDiff format
                let mut eth_changes = std::collections::HashMap::new();
                let mut token_changes = std::collections::HashMap::new();
                
                for (addr, changes) in state_changes {
                    // Convert revm Address to ethers Address for consistency
                    let addr_bytes: [u8; 20] = addr.0.into();
                    let ethers_addr = ethers::types::Address::from(addr_bytes);
                    
                    // Add ETH changes
                    if changes.eth_net_change.absolute_value > revm_primitives::U256::ZERO {
                        // Convert SignedAmount to revm_primitives::I256
                        let i256_value = if changes.eth_net_change.is_negative {
                            // Create negative I256
                            let val = changes.eth_net_change.absolute_value;
                            revm_primitives::I256::from_raw(val).wrapping_neg()
                        } else {
                            revm_primitives::I256::from_raw(changes.eth_net_change.absolute_value)
                        };
                        eth_changes.insert(ethers_addr, i256_value);
                    }
                    
                    // Add token changes
                    for (token_addr, signed_amount) in changes.token_net_changes {
                        if signed_amount.absolute_value > revm_primitives::U256::ZERO {
                            let token_bytes: [u8; 20] = token_addr.0.into();
                            let ethers_token = ethers::types::Address::from(token_bytes);
                            
                            let i256_value = if signed_amount.is_negative {
                                revm_primitives::I256::from_raw(signed_amount.absolute_value).wrapping_neg()
                            } else {
                                revm_primitives::I256::from_raw(signed_amount.absolute_value)
                            };
                            token_changes.insert((ethers_addr, ethers_token), i256_value);
                        }
                    }
                }
                
                let state_diff = crate::tx_simulator::MempoolStateDiff {
                    before: None,
                    after: None,
                    change: 0.0,
                    eth_changes,
                    token_changes,
                };
                
                // Analyze the state changes for scam patterns
                self.detect_scam_patterns(&tx, &state_diff).await
            }
            Ok(None) => {
                debug!("No state changes for transaction");
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to simulate transaction {}: {}", tx.hash, e);
                Ok(None)
            }
        }
    }
    
    /// Detect scam patterns from state changes
    async fn detect_scam_patterns(
        &self, 
        tx: &Transaction,
        state_diff: &crate::tx_simulator::MempoolStateDiff
    ) -> Result<Option<ScamAlert>> {
        // Check each watched pool
        for (&pool_addr, pool_info) in &self.watched_pools {
            // Check ETH/WETH drain from pool
            let eth_change = state_diff.eth_changes.get(&pool_addr)
                .map(|i256| {
                    // Convert revm_primitives::I256 to i128 for calculation
                    if i256.is_negative() {
                        -(i256.abs().to_string().parse::<i128>().unwrap_or(0))
                    } else {
                        i256.to_string().parse::<i128>().unwrap_or(0)
                    }
                })
                .unwrap_or(0);
            
            if eth_change < 0 {  // Pool lost ETH/WETH
                let drain_amount = (-eth_change) as f64 / 1e18;
                let drain_percentage = (drain_amount / pool_info.liquidity_eth) * 100.0;
                
                // Significant drain detected
                if drain_percentage > 20.0 {
                    // Find who received the funds
                    let mut recipients = Vec::new();
                    for (addr, change) in &state_diff.eth_changes {
                        let change_val = if change.is_negative() {
                            -(change.abs().to_string().parse::<i128>().unwrap_or(0))
                        } else {
                            change.to_string().parse::<i128>().unwrap_or(0)
                        };
                        
                        if change_val > 0 && addr != &pool_addr {
                            let amount = change_val as f64 / 1e18;
                            recipients.push((*addr, amount));
                        }
                    }
                    
                    // Determine scam type and severity
                    let (scam_type, severity) = self.classify_scam(
                        drain_percentage,
                        &recipients,
                        tx,
                        pool_info
                    );
                    
                    return Ok(Some(ScamAlert {
                        tx_hash: tx.hash,
                        severity,
                        scam_type,
                        affected_pool: pool_addr,
                        drain_amount_eth: drain_amount,
                        drain_percentage,
                        victim_count: 1, // Could be enhanced
                        details: format!(
                            "Pool {:?} drained {:.2} ETH ({:.1}%)", 
                            pool_addr, drain_amount, drain_percentage
                        ),
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Classify the type and severity of scam
    fn classify_scam(
        &self,
        drain_percentage: f64,
        recipients: &[(Address, f64)],
        tx: &Transaction,
        pool_info: &PoolInfo,
    ) -> (ScamType, ScamSeverity) {
        // Determine severity
        let severity = match drain_percentage {
            x if x > 80.0 => ScamSeverity::Critical,
            x if x > 50.0 => ScamSeverity::High,
            x if x > 20.0 => ScamSeverity::Medium,
            _ => ScamSeverity::Low,
        };
        
        // Determine type
        let scam_type = if pool_info.is_honeypot {
            ScamType::HoneypotDrain
        } else if recipients.len() == 1 && recipients[0].0 == tx.from {
            ScamType::RugPull  // Sender draining to themselves
        } else if tx.value == U256::zero() && drain_percentage > 90.0 {
            ScamType::FlashLoanAttack  // Large drain with no ETH sent
        } else {
            ScamType::Unknown
        };
        
        (scam_type, severity)
    }
    
    /// Check if transaction might affect a pool (simplified)
    fn might_affect_pool(&self, tx: &Transaction, pool: Address) -> bool {
        // Check if transaction is to a router that might interact with pool
        let routers = [
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", // Uniswap V2
            "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3
        ];
        
        if let Some(to) = tx.to {
            routers.iter().any(|&router| {
                to == router.parse::<Address>().unwrap_or_default()
            })
        } else {
            false
        }
    }
    
    /// Convert ethers Transaction to our TransactionView
    fn convert_to_tx_view(&self, tx: &Transaction) -> crate::mempool_fetcher::types::TransactionView {
        use crate::mempool_fetcher::types::TransactionView;
        
        TransactionView {
            hash: tx.hash.as_bytes().to_vec(),
            from: tx.from.as_bytes().to_vec(),
            to: tx.to.map(|addr| addr.as_bytes().to_vec()),
            value: tx.value,
            gas_price: tx.gas_price,
            gas_limit: Some(tx.gas),
            nonce: Some(tx.nonce),
            input_data: Some(tx.input.to_vec()),
        }
    }
    
    /// Get current block information
    async fn get_current_block(&self) -> Result<(u64, u64)> {
        use ethers::types::{BlockNumber, BlockId};
        
        let block = self.provider
            .get_block(BlockId::Number(BlockNumber::Latest))
            .await?
            .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
        
        let block_number = block.number
            .ok_or_else(|| eyre::eyre!("Block has no number"))?
            .as_u64();
        let timestamp = block.timestamp.as_u64();
        
        Ok((block_number, timestamp))
    }
}

/// Example usage
pub async fn monitor_mempool_for_scams(
    rpc_url: &str,
    pool_address: Address,
) -> Result<()> {
    let mut detector = MempoolScamDetector::new(rpc_url, 1).await?;
    
    // Add pool to watch
    detector.add_watched_pool(pool_address, PoolInfo {
        token0: "0x2e32f96a4FbB9cD7CDC751971c015E282414B956".parse()?,
        token1: detector.weth_address,
        liquidity_eth: 14.2,  // Current liquidity
        is_honeypot: false,
    });
    
    // In production, would subscribe to mempool stream
    // For now, just show the structure
    info!("Mempool scam detector initialized for pool: {:?}", pool_address);
    
    Ok(())
}