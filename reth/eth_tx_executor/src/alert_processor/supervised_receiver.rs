//! Supervised Alert Receiver
//!
//! Fault-tolerant alert reception with automatic restart and recovery

use super::types::Alert;
use super::validation::{AlertValidator, SignedMessage};
use super::ReceiverConfig;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Supervised receiver with automatic restart
pub struct SupervisedReceiver {
    config: ReceiverConfig,
    tx: mpsc::Sender<Alert>,
    shutdown: Arc<AtomicBool>,
    restarts: Arc<AtomicU64>,
    validator: Arc<AlertValidator>,
}

impl SupervisedReceiver {
    pub fn new(config: ReceiverConfig, tx: mpsc::Sender<Alert>) -> Self {
        let validator = Arc::new(AlertValidator::new(&config.hmac_secret));

        Self {
            config,
            tx,
            shutdown: Arc::new(AtomicBool::new(false)),
            restarts: Arc::new(AtomicU64::new(0)),
            validator,
        }
    }

    /// Run with supervision and automatic restart
    pub async fn run_supervised(&self) {
        let mut consecutive_failures = 0;

        while !self.shutdown.load(Ordering::Relaxed) {
            match self.run_receiver().await {
                Ok(_) => {
                    info!("Alert receiver exited normally");
                    consecutive_failures = 0;
                }
                Err(e) => {
                    consecutive_failures += 1;
                    let restart_count = self.restarts.fetch_add(1, Ordering::Relaxed) + 1;

                    error!(
                        "Alert receiver crashed (restart #{}, consecutive failures: {}): {}",
                        restart_count, consecutive_failures, e
                    );

                    // Exponential backoff with max delay
                    let delay = Duration::from_millis(
                        (100 * 2_u64.pow(consecutive_failures.min(10))).min(30000),
                    );

                    if consecutive_failures > 10 {
                        error!(
                            "Too many consecutive failures, waiting {} seconds before retry",
                            delay.as_secs()
                        );
                    }

                    sleep(delay).await;
                }
            }
        }

        info!("Supervised receiver shutting down");
    }

    /// Inner receiver loop
    async fn run_receiver(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ctx = zmq::Context::new();
        let socket = ctx.socket(zmq::SUB)?;

        // Configure socket with proper timeouts
        socket.set_rcvtimeo(self.config.timeout_ms)?;
        socket.set_linger(0)?;
        socket.set_tcp_keepalive(1)?;
        socket.set_tcp_keepalive_idle(120)?;
        socket.set_tcp_keepalive_intvl(30)?;

        info!("Connecting to alert endpoint: {}", self.config.endpoint);
        socket.connect(&self.config.endpoint)?;
        socket.set_subscribe(b"")?;

        info!("Connected to alert stream");

        let mut alerts_received = 0u64;
        let mut alerts_dropped = 0u64;
        let mut validation_failures = 0u64;

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                info!("Receiver shutdown requested");
                break;
            }

            match socket.recv_msg(0) {
                Ok(msg) => {
                    let data = msg.as_str().unwrap_or("");

                    // Parse and validate
                    match SignedMessage::from_zmq_message(data) {
                        Ok(signed_msg) => {
                            match self
                                .validator
                                .validate_alert(&signed_msg.payload, &signed_msg.signature)
                            {
                                Ok(alert) => {
                                    alerts_received += 1;

                                    // Try to send with timeout
                                    match self.tx.try_send(alert) {
                                        Ok(_) => {
                                            if alerts_received % 1000 == 0 {
                                                info!("Processed {} alerts (dropped: {}, invalid: {})", 
                                                      alerts_received, alerts_dropped, validation_failures);
                                            }
                                        }
                                        Err(mpsc::error::TrySendError::Full(alert)) => {
                                            alerts_dropped += 1;
                                            warn!("Alert channel full, attempting blocking send for high priority alert");

                                            // For critical alerts, block briefly
                                            if matches!(
                                                alert.params.priority,
                                                crate::common::Priority::Critical
                                            ) {
                                                let send_timeout = Duration::from_millis(100);
                                                if let Err(_) = tokio::time::timeout(
                                                    send_timeout,
                                                    self.tx.send(alert),
                                                )
                                                .await
                                                {
                                                    error!("Failed to send critical alert even with blocking");
                                                }
                                            }
                                        }
                                        Err(mpsc::error::TrySendError::Closed(_)) => {
                                            error!("Alert channel closed, executor died");
                                            return Err("Channel closed".into());
                                        }
                                    }
                                }
                                Err(e) => {
                                    validation_failures += 1;
                                    warn!("Alert validation failed: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse message format: {}", e);
                        }
                    }
                }
                Err(zmq::Error::EAGAIN) => {
                    // Timeout - normal, continue
                    continue;
                }
                Err(e) => {
                    error!("ZMQ receive error: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> ReceiverStats {
        ReceiverStats {
            restarts: self.restarts.load(Ordering::Relaxed),
            is_running: !self.shutdown.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReceiverStats {
    pub restarts: u64,
    pub is_running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supervisor_restart() {
        // Test would verify restart behavior
    }
}
