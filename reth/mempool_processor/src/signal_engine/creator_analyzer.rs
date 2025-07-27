// signal_engine/creator_analyzer.rs
//
// Analyzes transactions from known creators/owners/LPs to detect suspicious activity
// before it impacts pools. Works with the token tracking cache to identify creators
// and track their mempool usage patterns.

use std::sync::Arc;
use tracing::{info, warn, error};
use crate::token_tracking::TokenTrackingCache;
use crate::mempool_fetcher::MempoolTransaction;
use crate::common::address::checksum_address;
use zmq::{Context, Socket};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

lazy_static! {
    /// Creator actions log file with token addresses
    static ref CREATOR_ACTIONS_LOG: Mutex<std::fs::File> = {
        // Create log directory if it doesn't exist
        let log_dir = PathBuf::from("/home/nima/code/crypto/logs/mempool/creator_actions");
        let _ = std::fs::create_dir_all(&log_dir);
        
        // Create log file with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d");
        let log_path = log_dir.join(format!("creator_actions_{}.log", timestamp));
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open creator actions log file");
        
        Mutex::new(file)
    };
    
    /// ZMQ Publisher for creator alerts
    static ref CREATOR_ALERT_PUBLISHER: Mutex<Option<Socket>> = {
        match Context::new().socket(zmq::PUB) {
            Ok(socket) => {
                let _ = socket.set_sndhwm(10000);
                let _ = socket.set_linger(0);
                
                match socket.bind("tcp://127.0.0.1:5559") {
                    Ok(_) => {
                        info!("✅ Creator alert publisher bound to tcp://127.0.0.1:5559");
                        Mutex::new(Some(socket))
                    }
                    Err(e) => {
                        error!("Failed to bind creator alert publisher: {}", e);
                        Mutex::new(None)
                    }
                }
            }
            Err(e) => {
                error!("Failed to create ZMQ socket for creator alerts: {}", e);
                Mutex::new(None)
            }
        }
    };
}

/// Alert severity levels for creator actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,  // Immediate action required (liquidity removal, pause)
    High,      // Serious concern (high fees, blacklisting)
    Medium,    // Monitor closely (ownership transfer, limits)
    Low,       // Informational (normal operations)
}

/// Creator alert for immediate notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorAlert {
    pub severity: AlertSeverity,
    pub alert_type: String,
    pub creator_address: String,
    pub token_address: Option<String>,
    pub function_name: String,
    pub tx_hash: String,
    pub to_address: String,
    pub timestamp: String,
    pub message: String,
    pub has_position: bool,
    pub position_value: Option<f64>,
}

/// Tracks creator transaction for mempool visibility
#[derive(Debug, Clone)]
pub struct CreatorTransaction {
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub function_name: String,
    pub to_address: String,
    pub gas_price: u64,
    pub seen_in_mempool: bool,
}

/// Analyzes creator transactions for suspicious activity
pub struct CreatorAnalyzer {
    token_cache: Arc<TokenTrackingCache>,
}

impl CreatorAnalyzer {
    pub fn new(token_cache: Arc<TokenTrackingCache>) -> Self {
        info!("🔍 Creator analyzer initialized");
        Self { token_cache }
    }
    
    /// Check if a function is a creator-related action
    pub fn is_creator_action(&self, function_name: &str) -> bool {
        matches!(function_name,
            "removeLiquidity" | "removeLiquidityETH" | "removeLiquidityETHSupportingFeeOnTransferTokens" |
            "pause" | "stopTrading" | "disableTrading" |
            "emergencyWithdraw" | "withdrawETH" | "withdrawToken" |
            "setFee" | "setTaxPercent" | "setSellFee" | "setBuyFee" |
            "blacklist" | "addToBlacklist" | "setBlacklisted" |
            "setMaxWalletPercent" | "setMaxTxPercent" | "setMaxTransaction" | "setMaxWallet" |
            "renounceOwnership" | "transferOwnership" |
            "unpause" | "enableTrading" | "openTrading" | "startTrading" |
            "excludeFromFee" | "includeInFee" | "setExcludeFromFee" |
            "airdrop" | "multisend" | "distributeTokens"
        )
    }
    
    /// Analyze a transaction with detected functions
    pub async fn analyze_transaction(&self, tx: &MempoolTransaction) -> Option<CreatorAlert> {
        // Check if this is a token creation or trading enable transaction
        if tx.functions.iter().any(|f| f.contains("enableTrading") || f.contains("openTrading")) {
            let from_address = checksum_address(&hex::encode(&tx.from));
            
            // Record that we saw this creator's trading enable tx in mempool
            if let Some(function_name) = tx.functions.iter()
                .find(|f| f.contains("Trading") || f.contains("trading")) {
                self.record_creator_transaction(&from_address, &tx.hash, function_name, true).await;
                info!("Observed trading enable from {} in mempool - marking as public user", from_address);
            }
        }
        
        // Skip if no creator action detected
        if !tx.functions.iter().any(|f| self.is_creator_action(f)) {
            return None;
        }
        let from_address = checksum_address(&hex::encode(&tx.from));
        let to_address = tx.to.as_ref()
            .map(|addr| checksum_address(&hex::encode(addr)))
            .unwrap_or_else(|| "contract_creation".to_string());
        
        // Check if this is from a known creator
        let creator_info = self.check_if_known_creator(&from_address).await;
        
        if let Some((token_address, is_creator, is_owner, is_lp)) = creator_info {
            let function_name = tx.functions.iter()
                .find(|f| self.is_creator_action(f))
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            
            // Record this transaction for mempool visibility tracking
            self.record_creator_transaction(&from_address, &tx.hash, function_name, true).await;
            
            // Determine severity based on function
            let (severity, message) = self.determine_severity(function_name, is_creator, is_owner, is_lp);
            
            // Check if we have a position in this token
            let (has_position, position_value) = self.check_position(&token_address);
            
            // Create alert
            let alert = CreatorAlert {
                severity,
                alert_type: "creator_action".to_string(),
                creator_address: from_address.clone(),
                token_address: Some(token_address.clone()),
                function_name: function_name.to_string(),
                tx_hash: tx.hash.clone(),
                to_address: to_address.clone(),
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
                message,
                has_position,
                position_value,
            };
            
            // Log the alert
            match alert.severity {
                AlertSeverity::Critical => {
                    error!("🚨 CRITICAL CREATOR ACTION: {} by {} for token {}", 
                           function_name, from_address, token_address);
                }
                AlertSeverity::High => {
                    warn!("⚠️ HIGH RISK CREATOR ACTION: {} by {} for token {}", 
                          function_name, from_address, token_address);
                }
                AlertSeverity::Medium => {
                    warn!("⚡ MEDIUM RISK CREATOR ACTION: {} by {} for token {}", 
                          function_name, from_address, token_address);
                }
                AlertSeverity::Low => {
                    info!("ℹ️ Creator action: {} by {} for token {}", 
                          function_name, from_address, token_address);
                }
            }
            
            // Log to dedicated creator actions file with pattern-friendly format
            if let Ok(mut log_file) = CREATOR_ACTIONS_LOG.lock() {
                let severity_str = match alert.severity {
                    AlertSeverity::Critical => "CRITICAL",
                    AlertSeverity::High => "HIGH",
                    AlertSeverity::Medium => "MEDIUM",
                    AlertSeverity::Low => "LOW",
                };
                
                let _ = writeln!(log_file,
                    "[{}] [{}] {} | {} | {} | {}",
                    alert.timestamp, severity_str, function_name, token_address, from_address, tx.hash
                );
                let _ = log_file.flush();
            }
            
            // Publish alert
            self.publish_alert(&alert);
            
            return Some(alert);
        }
        
        None
    }
    
    /// Check if address is a known creator/owner/LP
    async fn check_if_known_creator(&self, address: &str) -> Option<(String, bool, bool, bool)> {
        // Check if this address is a creator of any token
        if let Some(token_address) = self.token_cache.creators.get_token_by_creator(address).await {
            return Some((token_address, true, false, false));
        }
        
        // TODO: Check if this address is an owner of any token
        // This would require tracking current owners separately from creators
        
        // TODO: Check if this address is an LP provider
        // This would require tracking LP providers from pool events
        
        None
    }
    
    /// Determine severity based on function and role
    fn determine_severity(&self, function_name: &str, is_creator: bool, is_owner: bool, _is_lp: bool) -> (AlertSeverity, String) {
        match function_name {
            // Critical - immediate action needed
            "removeLiquidity" | "removeLiquidityETH" | "removeLiquidityETHSupportingFeeOnTransferTokens" => {
                (AlertSeverity::Critical, "Liquidity removal detected - potential rug pull!".to_string())
            }
            "pause" | "stopTrading" | "disableTrading" => {
                (AlertSeverity::Critical, "Trading being disabled - users cannot sell!".to_string())
            }
            "emergencyWithdraw" | "withdrawETH" | "withdrawToken" => {
                (AlertSeverity::Critical, "Emergency withdrawal - funds being drained!".to_string())
            }
            
            // High - serious concern
            "setFee" | "setTaxPercent" | "setSellFee" | "setBuyFee" if is_creator || is_owner => {
                (AlertSeverity::High, "Fee manipulation - could prevent selling!".to_string())
            }
            "blacklist" | "addToBlacklist" | "setBlacklisted" => {
                (AlertSeverity::High, "Blacklisting addresses - preventing trades!".to_string())
            }
            "transferOwnership" if is_creator || is_owner => {
                (AlertSeverity::High, "Ownership transfer - control changing hands!".to_string())
            }
            
            // Medium - monitor closely
            "setMaxTx" | "setMaxWallet" | "setMaxBuyAmount" | "setMaxSellAmount" => {
                (AlertSeverity::Medium, "Transaction limits being modified".to_string())
            }
            "renounceOwnership" => {
                (AlertSeverity::Medium, "Ownership being renounced - could be positive or negative".to_string())
            }
            "unpause" => {
                (AlertSeverity::Medium, "Trading being re-enabled".to_string())
            }
            
            // Low - informational
            _ => {
                (AlertSeverity::Low, format!("Creator function {} called", function_name))
            }
        }
    }
    
    /// Check if we have a position in this token
    fn check_position(&self, _token_address: &str) -> (bool, Option<f64>) {
        // TODO: Integrate with position tracking
        // For now, return false
        (false, None)
    }
    
    /// Record creator transaction for mempool visibility tracking
    pub async fn record_creator_transaction(&self, creator_address: &str, tx_hash: &str, 
                                     function_name: &str, seen_in_mempool: bool) {
        // Record in the token cache for pattern analysis
        self.token_cache.creators.record_creator_transaction(
            creator_address, tx_hash, function_name, seen_in_mempool
        ).await;
        
        info!("Recording creator tx: {} called {} (mempool: {})", 
              creator_address, function_name, seen_in_mempool);
    }
    
    /// Mark a creator transaction as mined (for mempool visibility analysis)
    pub async fn mark_transaction_mined(&self, tx_hash: &str, block_number: u64) {
        // Update the transaction record with block number
        self.token_cache.creators.mark_transaction_mined(tx_hash, block_number).await;
    }
    
    /// Publish creator alert via ZMQ
    fn publish_alert(&self, alert: &CreatorAlert) {
        if let Ok(publisher) = CREATOR_ALERT_PUBLISHER.lock() {
            if let Some(ref socket) = *publisher {
                match serde_json::to_string(alert) {
                    Ok(json) => {
                        match socket.send(&json, zmq::DONTWAIT) {
                            Ok(_) => {
                                // Alert published successfully
                            }
                            Err(zmq::Error::EAGAIN) => {
                                warn!("Creator alert publisher buffer full, alert dropped");
                            }
                            Err(e) => {
                                error!("Failed to publish creator alert: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize creator alert: {}", e);
                    }
                }
            }
        }
    }
    
    /// Get statistics about creator monitoring
    pub async fn get_stats(&self) -> CreatorAnalyzerStats {
        CreatorAnalyzerStats {
            monitored_creators: self.token_cache.creators.get_creator_count().await as u64,
            critical_alerts: 0, // TODO: Track these
            high_alerts: 0,
            medium_alerts: 0,
            low_alerts: 0,
        }
    }
}

/// Statistics for creator monitoring
#[derive(Debug, Clone)]
pub struct CreatorAnalyzerStats {
    pub monitored_creators: u64,
    pub critical_alerts: u64,
    pub high_alerts: u64,
    pub medium_alerts: u64,
    pub low_alerts: u64,
}