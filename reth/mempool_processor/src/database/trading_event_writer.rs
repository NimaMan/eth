/// Trading Event Writer
/// 
/// Writes mempool-detected trading events to PostgreSQL database.
/// Integrates with the live_trading_db schema for comprehensive tracking.

use tokio_postgres::{NoTls, Client};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tracing::{info, error, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use eyre::{eyre, Result};

/// Trading enabled event (enableTrading, openTrading, etc.)
#[derive(Debug, Clone)]
pub struct TradingEnabledEvent {
    pub token_address: String,
    pub pool_address: String,
    pub function_name: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub detected_latency_us: u64,
}

/// Creator action event (suspicious or important activities)
#[derive(Debug, Clone)]
pub struct CreatorActionEvent {
    pub creator_address: String,
    pub token_address: String,
    pub action_type: String, // "remove_liquidity", "pause_trading", etc.
    pub severity: String, // "info", "warning", "critical"
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub pool_eth_before: Option<f64>,
    pub pool_eth_after: Option<f64>,
}

/// General mempool signal for flexible tracking
#[derive(Debug, Clone)]
pub struct MempoolSignal {
    pub signal_type: String,
    pub token_address: String,
    pub pool_address: Option<String>,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: Option<String>,
    pub function_signature: Option<String>,
    pub function_name: Option<String>,
    pub detection_latency_us: u64,
    pub block_number: Option<u64>,
    pub gas_price_gwei: Option<f64>,
    pub value_eth: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

pub struct TradingEventWriter {
    client: Arc<Mutex<Option<Client>>>,
    connection_string: String,
}

impl TradingEventWriter {
    pub async fn new(
        user: &str,
        password: &str,
        host: &str,
        port: u16,
        dbname: &str,
    ) -> Result<Self> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}/{}?application_name=mempool_event_writer",
            user, password, host, port, dbname
        );
        
        let writer = TradingEventWriter {
            client: Arc::new(Mutex::new(None)),
            connection_string,
        };
        
        writer.connect().await?;
        info!("Trading event writer initialized successfully");
        Ok(writer)
    }
    
    pub async fn default() -> Result<Self> {
        let user = std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string());
        let password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
        let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("DB_PORT")
            .unwrap_or_else(|_| "5432".to_string())
            .parse::<u16>()
            .unwrap_or(5432);
        let database = std::env::var("DB_NAME").unwrap_or_else(|_| "live_trading_db".to_string());
        
        Self::new(&user, &password, &host, port, &database).await
    }
    
    async fn connect(&self) -> Result<()> {
        let mut client_lock = self.client.lock().await;
        if client_lock.is_some() {
            return Ok(());
        }
        
        let connection_result = timeout(
            Duration::from_secs(5),
            tokio_postgres::connect(&self.connection_string, NoTls)
        ).await;
        
        let (client, connection) = match connection_result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => return Err(eyre!("Failed to connect to database: {}", e)),
            Err(_) => return Err(eyre!("Database connection timed out")),
        };
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Database connection error: {}", e);
            }
        });
        
        *client_lock = Some(client);
        info!("Connected to database successfully");
        Ok(())
    }
    
    async fn ensure_connected(&self) -> Result<()> {
        let client_lock = self.client.lock().await;
        if client_lock.is_none() {
            drop(client_lock);
            self.connect().await?;
        }
        Ok(())
    }
    
    pub async fn log_trading_enabled(&self, event: &TradingEnabledEvent) -> Result<()> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(event.timestamp);
        
        client.execute(
            r#"
            INSERT INTO trading_enabled_events 
            (token_address, pool_address, function_name, tx_hash, block_number, 
             timestamp, detected_latency_us)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            &[
                &event.token_address,
                &event.pool_address,
                &event.function_name,
                &event.tx_hash,
                &(event.block_number as i64),
                &timestamp,
                &(event.detected_latency_us as i64),
            ],
        ).await?;
        
        warn!("🚀 Logged trading enabled: {} via {} ({}μs latency)", 
              event.token_address, event.function_name, event.detected_latency_us);
        Ok(())
    }
    
    pub async fn mark_signal_sent(&self, token_address: &str, pool_address: &str) -> Result<()> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        client.execute(
            r#"
            UPDATE trading_enabled_events 
            SET signal_sent = true, signal_sent_at = NOW()
            WHERE token_address = $1 AND pool_address = $2 
                AND signal_sent = false
            "#,
            &[&token_address, &pool_address],
        ).await?;
        
        Ok(())
    }
    
    pub async fn log_creator_action(&self, event: &CreatorActionEvent) -> Result<()> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(event.timestamp);
        
        client.execute(
            r#"
            INSERT INTO creator_actions 
            (creator_address, token_address, action_type, severity, tx_hash, 
             block_number, timestamp, pool_eth_before, pool_eth_after)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            &[
                &event.creator_address,
                &event.token_address,
                &event.action_type,
                &event.severity,
                &event.tx_hash,
                &(event.block_number as i64),
                &timestamp,
                &event.pool_eth_before,
                &event.pool_eth_after,
            ],
        ).await?;
        
        match event.severity.as_str() {
            "critical" => error!("🚨 CRITICAL creator action: {} {} on {}", 
                               event.creator_address, event.action_type, event.token_address),
            "warning" => warn!("⚠️  WARNING creator action: {} {} on {}", 
                              event.creator_address, event.action_type, event.token_address),
            _ => info!("Creator action: {} {} on {}", 
                      event.creator_address, event.action_type, event.token_address),
        }
        Ok(())
    }
    
    pub async fn update_creator_response(&self, tx_hash: &str, response_action: &str, response_tx_hash: Option<&str>) -> Result<()> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        client.execute(
            r#"
            UPDATE creator_actions 
            SET response_action = $1, response_tx_hash = $2
            WHERE tx_hash = $3
            "#,
            &[&response_action, &response_tx_hash, &tx_hash],
        ).await?;
        
        info!("Updated creator action response: {} -> {}", tx_hash, response_action);
        Ok(())
    }
    
    pub async fn log_mempool_signal(&self, signal: &MempoolSignal) -> Result<()> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        client.execute(
            r#"
            INSERT INTO mempool_signals 
            (signal_type, token_address, pool_address, tx_hash, from_address, 
             to_address, function_signature, function_name, detection_latency_us, 
             block_number, gas_price_gwei, value_eth, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            &[
                &signal.signal_type,
                &signal.token_address,
                &signal.pool_address,
                &signal.tx_hash,
                &signal.from_address,
                &signal.to_address,
                &signal.function_signature,
                &signal.function_name,
                &(signal.detection_latency_us as i64),
                &signal.block_number.map(|b| b as i64),
                &signal.gas_price_gwei,
                &signal.value_eth,
                &signal.metadata.as_ref().map(|v| v.to_string()),
            ],
        ).await?;
        
        Ok(())
    }
    
    pub async fn has_trading_enabled(&self, token_address: &str) -> Result<bool> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        let row = client.query_one(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM trading_enabled_events 
                WHERE token_address = $1
            )
            "#,
            &[&token_address],
        ).await?;
        
        Ok(row.get(0))
    }
    
    pub async fn get_creator_severity(&self, creator_address: &str) -> Result<Option<String>> {
        self.ensure_connected().await?;
        
        let client_lock = self.client.lock().await;
        let client = client_lock.as_ref().ok_or_else(|| eyre!("Not connected to database"))?;
        
        let row = client.query_opt(
            r#"
            SELECT MAX(severity) as max_severity
            FROM creator_actions
            WHERE creator_address = $1
                AND timestamp > NOW() - INTERVAL '24 hours'
            GROUP BY creator_address
            "#,
            &[&creator_address],
        ).await?;
        
        Ok(row.map(|r| r.get(0)))
    }
}