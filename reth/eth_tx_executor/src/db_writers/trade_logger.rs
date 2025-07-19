//! Trade decision and execution logger
//!
//! Logs all trading activity to PostgreSQL for audit trail and analysis

use crate::alert_processor::Alert;
use crate::risk::RiskDecision;
use crate::tx_executor::{ExecutionResult, ExecutionMetrics};
use ethers::types::{H256, U256, Address};
use serde::{Serialize, Deserialize};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tracing::{info, error, warn};
use uuid::Uuid;

/// Trade event types for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeEvent {
    /// Alert received from mempool processor
    AlertReceived {
        alert: Alert,
        received_at: chrono::DateTime<chrono::Utc>,
    },
    
    /// Risk decision made
    RiskDecision {
        alert_id: String,
        decision: String, // Allow, ReduceSize, Block, EmergencyHalt
        original_amount: U256,
        final_amount: U256,
        reason: Option<String>,
    },
    
    /// Transaction submitted
    TxSubmitted {
        alert_id: String,
        tx_hash: H256,
        nonce: U256,
        gas_price: U256,
        execution_path: String, // PublicMempool, FlashbotsBundle, MultiPath
    },
    
    /// Transaction confirmed
    TxConfirmed {
        alert_id: String,
        tx_hash: H256,
        block_number: u64,
        gas_used: U256,
    },
    
    /// Transaction failed
    TxFailed {
        alert_id: String,
        tx_hash: Option<H256>,
        error: String,
        revert_reason: Option<String>,
    },
}

/// Detailed execution log matching database schema
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLog {
    pub signal_id: Uuid,
    pub alert_id: String,
    pub wallet_address: Address,
    pub token_address: Address,
    pub pool_address: Address,
    pub action: String, // BUY, SELL
    pub amount_eth: Option<f64>,
    pub amount_tokens: Option<U256>,
    pub slippage: f64,
    pub priority: String,
    pub status: String,
    pub tx_hash: Option<H256>,
    pub error_message: Option<String>,
    pub metrics: Option<ExecutionMetrics>,
    pub risk_score: Option<f64>,
    pub mev_protected: bool,
    pub gas_optimization_path: Option<serde_json::Value>,
}

/// Trade logger for comprehensive activity tracking
pub struct TradeLogger {
    pool: Option<Arc<PgPool>>,
    wallet_address: Address,
}

impl TradeLogger {
    /// Create new trade logger
    pub async fn new(database_url: Option<&str>, wallet_address: Address) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = if let Some(url) = database_url {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(url)
                .await?;
            
            info!("Trade logger connected to database");
            Some(Arc::new(pool))
        } else {
            warn!("Trade logger running without database connection");
            None
        };
        
        Ok(Self {
            pool,
            wallet_address,
        })
    }
    
    /// Log alert received
    pub async fn log_alert_received(&self, alert: &Alert) -> Uuid {
        let signal_id = Uuid::new_v4();
        
        info!(
            "📨 Alert received: {} | Token: {} | Action: {:?} | Amount: {} | Priority: {:?}",
            alert.id,
            alert.token_address,
            alert.action,
            alert.params.amount,
            alert.params.priority
        );
        
        if let Some(pool) = &self.pool {
            let action_str = format!("{:?}", alert.action).to_uppercase();
            let priority_str = format!("{:?}", alert.params.priority).to_uppercase();
            
            let result = sqlx::query(
                r#"
                INSERT INTO trade_signals (
                    signal_id, alert_id, wallet_id, token_address, pool_id,
                    action, amount_tokens, max_slippage_percent, priority,
                    deadline_timestamp, status, signal_timestamp
                ) VALUES (
                    $1, $2, 
                    (SELECT wallet_id FROM wallets WHERE address = $3 LIMIT 1),
                    $4,
                    (SELECT pool_id FROM pools WHERE address = $5 LIMIT 1),
                    $6, $7, $8, $9, 
                    to_timestamp($10), 'PENDING', NOW()
                )
                "#
            )
            .bind(signal_id)
            .bind(&alert.id)
            .bind(format!("{:?}", self.wallet_address))
            .bind(format!("{:?}", alert.token_address))
            .bind(format!("{:?}", alert.pool_address))
            .bind(action_str)
            .bind(alert.params.amount.to_string())
            .bind(alert.params.slippage * 100.0) // Convert to percentage
            .bind(priority_str)
            .bind(alert.timestamp as i64 + alert.params.deadline_seconds as i64)
            .execute(pool.as_ref())
            .await;
            
            if let Err(e) = result {
                error!("Failed to log alert to database: {}", e);
            }
        }
        
        signal_id
    }
    
    /// Log risk decision
    pub async fn log_risk_decision(
        &self, 
        signal_id: Uuid,
        alert_id: &str,
        decision: &RiskDecision,
        original_amount: U256,
    ) {
        let (decision_str, final_amount, reason) = match decision {
            RiskDecision::Allow => ("ALLOW", original_amount, None),
            RiskDecision::Block { reason } => 
                ("BLOCK", U256::zero(), Some(reason.clone())),
        };
        
        info!(
            "⚖️ Risk decision: {} | Alert: {} | Original: {} | Final: {} | Reason: {:?}",
            decision_str, alert_id, original_amount, final_amount, reason
        );
        
        if let Some(pool) = &self.pool {
            let risk_details = json!({
                "decision": decision_str,
                "original_amount": original_amount.to_string(),
                "final_amount": final_amount.to_string(),
                "reason": reason,
            });
            
            let result = sqlx::query(
                r#"
                UPDATE trade_signals 
                SET 
                    risk_assessment = $1,
                    amount_tokens = $2,
                    updated_at = NOW()
                WHERE signal_id = $3
                "#
            )
            .bind(risk_details)
            .bind(final_amount.to_string())
            .bind(signal_id)
            .execute(pool.as_ref())
            .await;
            
            if let Err(e) = result {
                error!("Failed to log risk decision: {}", e);
            }
        }
    }
    
    /// Log transaction submission
    pub async fn log_tx_submitted(
        &self,
        signal_id: Uuid,
        alert_id: &str,
        tx_hash: H256,
        nonce: U256,
        gas_price: U256,
        execution_path: &str,
    ) {
        info!(
            "📤 Transaction submitted: {} | Alert: {} | Nonce: {} | Gas: {} | Path: {}",
            tx_hash, alert_id, nonce, gas_price, execution_path
        );
        
        if let Some(pool) = &self.pool {
            // Update signal status
            let _ = sqlx::query(
                r#"
                UPDATE trade_signals 
                SET 
                    status = 'SUBMITTED',
                    tx_hash = $1,
                    submission_timestamp = NOW(),
                    updated_at = NOW()
                WHERE signal_id = $2
                "#
            )
            .bind(format!("{:?}", tx_hash))
            .bind(signal_id)
            .execute(pool.as_ref())
            .await;
            
            // Create execution record
            let _ = sqlx::query(
                r#"
                INSERT INTO executions (
                    execution_id, signal_id, attempt_number, tx_hash,
                    nonce, gas_price, mev_protected, execution_timestamp
                ) VALUES (
                    gen_random_uuid(), $1, 1, $2, $3, $4, $5, NOW()
                )
                "#
            )
            .bind(signal_id)
            .bind(format!("{:?}", tx_hash))
            .bind(nonce.as_u64() as i32)
            .bind(gas_price.to_string())
            .bind(execution_path.contains("Flashbots"))
            .execute(pool.as_ref())
            .await;
        }
    }
    
    /// Log execution result
    pub async fn log_execution_result(
        &self,
        signal_id: Uuid,
        alert_id: &str,
        result: &ExecutionResult,
    ) {
        if result.success {
            info!(
                "✅ Execution successful: {} | Alert: {} | Latency: {}ms",
                result.tx_hash.unwrap_or_default(), alert_id, result.metrics.total_ms
            );
            
            // Log detailed metrics
            info!("  📊 Performance breakdown:");
            info!("    • Alert→Start: {}ms", result.metrics.alert_to_start_ms);
            info!("    • Position check: {}ms", result.metrics.position_check_ms);
            info!("    • Gas ranking: {}ms", result.metrics.gas_ranking_ms);
            info!("    • Price quote: {}ms", result.metrics.price_quote_ms);
            info!("    • TX build: {}ms", result.metrics.tx_build_ms);
            info!("    • TX submit: {}ms", result.metrics.tx_submit_ms);
        } else {
            error!(
                "❌ Execution failed: Alert: {} | Error: {}",
                alert_id, result.error.as_ref().unwrap_or(&"Unknown".to_string())
            );
        }
        
        if let Some(pool) = &self.pool {
            let status = if result.success { "CONFIRMED" } else { "FAILED" };
            
            // Update signal status
            let _ = sqlx::query(
                r#"
                UPDATE trade_signals 
                SET 
                    status = $1,
                    error_message = $2,
                    confirmation_timestamp = CASE WHEN $3 THEN NOW() ELSE NULL END,
                    updated_at = NOW()
                WHERE signal_id = $4
                "#
            )
            .bind(status)
            .bind(result.error.as_deref())
            .bind(result.success)
            .bind(signal_id)
            .execute(pool.as_ref())
            .await;
            
            // Update execution with metrics
            if let Some(tx_hash) = result.tx_hash {
                let _ = sqlx::query(
                    r#"
                    UPDATE executions 
                    SET 
                        success = $1,
                        alert_to_start_ms = $2,
                        position_check_ms = $3,
                        gas_ranking_ms = $4,
                        price_quote_ms = $5,
                        tx_build_ms = $6,
                        tx_submit_ms = $7,
                        total_execution_ms = $8,
                        error_details = $9
                    WHERE tx_hash = $10
                    "#
                )
                .bind(result.success)
                .bind(result.metrics.alert_to_start_ms as i32)
                .bind(result.metrics.position_check_ms as i32)
                .bind(result.metrics.gas_ranking_ms as i32)
                .bind(result.metrics.price_quote_ms as i32)
                .bind(result.metrics.tx_build_ms as i32)
                .bind(result.metrics.tx_submit_ms as i32)
                .bind(result.metrics.total_ms as i32)
                .bind(result.error.as_ref().map(|e| json!({"error": e})))
                .bind(format!("{:?}", tx_hash))
                .execute(pool.as_ref())
                .await;
            }
        }
    }
    
    /// Log custom event
    pub async fn log_event(&self, event: TradeEvent) {
        match &event {
            TradeEvent::AlertReceived { alert, received_at } => {
                info!("📨 Event: Alert {} received at {}", alert.id, received_at);
            }
            TradeEvent::RiskDecision { alert_id, decision, .. } => {
                info!("⚖️ Event: Risk {} for alert {}", decision, alert_id);
            }
            TradeEvent::TxSubmitted { tx_hash, .. } => {
                info!("📤 Event: Transaction {} submitted", tx_hash);
            }
            TradeEvent::TxConfirmed { tx_hash, block_number, .. } => {
                info!("✅ Event: Transaction {} confirmed in block {}", tx_hash, block_number);
            }
            TradeEvent::TxFailed { alert_id, error, .. } => {
                error!("❌ Event: Transaction failed for alert {}: {}", alert_id, error);
            }
        }
        
        // Could extend this to log custom events to a separate table
    }
    
    /// Get wallet statistics
    pub async fn get_wallet_stats(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        if let Some(pool) = &self.pool {
            let row = sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>, Option<f64>)>(
                r#"
                SELECT 
                    COUNT(*) as total_signals,
                    COUNT(CASE WHEN status = 'CONFIRMED' THEN 1 END) as successful,
                    COUNT(CASE WHEN status = 'FAILED' THEN 1 END) as failed,
                    AVG(CASE WHEN status = 'CONFIRMED' 
                        THEN EXTRACT(EPOCH FROM (confirmation_timestamp - signal_timestamp))
                    END) as avg_execution_time
                FROM trade_signals 
                WHERE wallet_id = (SELECT wallet_id FROM wallets WHERE address = $1 LIMIT 1)
                AND signal_timestamp > NOW() - INTERVAL '24 hours'
                "#
            )
            .bind(format!("{:?}", self.wallet_address))
            .fetch_one(pool.as_ref())
            .await?;
            
            let (total_signals, successful, failed, avg_execution_time) = row;
            
            Ok(json!({
                "24h_stats": {
                    "total_trades": total_signals.unwrap_or(0),
                    "successful": successful.unwrap_or(0),
                    "failed": failed.unwrap_or(0),
                    "success_rate": successful.unwrap_or(0) as f64 / total_signals.unwrap_or(1) as f64,
                    "avg_execution_time_sec": avg_execution_time,
                }
            }))
        } else {
            Ok(json!({"error": "No database connection"}))
        }
    }
}