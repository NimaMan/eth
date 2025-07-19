//! Retry Executor
//!
//! Implements retry logic with exponential backoff for reliable execution

use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use crate::common::errors::{KartalError, ExecutionError};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

/// Retry executor for transaction operations
pub struct RetryExecutor {
    config: RetryConfig,
}

impl RetryExecutor {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
    
    /// Execute with retry logic
    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, KartalError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, KartalError>>,
    {
        let mut attempt = 0;
        let mut last_error = None;
        
        loop {
            attempt += 1;
            
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        info!("{} succeeded after {} attempts", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    // Check if error is retryable
                    if !self.is_retryable(&err) {
                        error!("{} failed with non-retryable error: {}", operation_name, err);
                        return Err(err);
                    }
                    
                    last_error = Some(err);
                    
                    if attempt >= self.config.max_attempts {
                        error!("{} failed after {} attempts", operation_name, attempt);
                        break;
                    }
                    
                    // Calculate delay with exponential backoff and jitter
                    let delay = self.calculate_delay(attempt);
                    
                    warn!("{} failed (attempt {}/{}), retrying in {}ms: {}", 
                          operation_name, 
                          attempt, 
                          self.config.max_attempts,
                          delay.as_millis(),
                          last_error.as_ref().unwrap());
                    
                    sleep(delay).await;
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| KartalError::Generic("Max retries exceeded".to_string())))
    }
    
    /// Execute transaction with retry
    pub async fn execute_transaction_with_retry(
        &self,
        provider: Arc<Provider<Http>>,
        tx: TypedTransaction,
        wallet: Arc<LocalWallet>,
    ) -> Result<Option<TransactionReceipt>, KartalError> {
        self.execute_with_retry("transaction execution", || async {
            // Clone transaction for each attempt
            let tx = tx.clone();
            
            // Sign transaction
            let signed_tx = provider.sign_transaction(&tx, wallet.address())
                .await
                .map_err(|e| KartalError::Execution(ExecutionError::TransactionFailed {
                    reason: format!("Failed to sign: {}", e)
                }))?;
            
            // Send transaction
            let raw_tx = tx.rlp_signed(&signed_tx);
            let pending_tx = provider.send_raw_transaction(raw_tx)
                .await
                .map_err(|e| self.map_provider_error(e))?;
            
            // Wait for confirmation with timeout
            let receipt = tokio::time::timeout(
                Duration::from_secs(120),
                pending_tx
            )
            .await
            .map_err(|_| KartalError::Execution(ExecutionError::DeadlineExceeded))?
            .map_err(|e| KartalError::Network(crate::common::errors::NetworkError::ProviderError(e)))?;
            
            Ok(receipt)
        }).await
    }
    
    /// Check if error is retryable
    fn is_retryable(&self, error: &KartalError) -> bool {
        match error {
            // Network errors are generally retryable
            KartalError::Network(_) => true,
            
            // Some execution errors are retryable
            KartalError::Execution(exec_err) => match exec_err {
                ExecutionError::DeadlineExceeded => true,
                ExecutionError::SimulationFailed { .. } => true,
                ExecutionError::NonceTooLow { .. } => false, // Don't retry nonce errors
                ExecutionError::InsufficientBalance { .. } => false, // Don't retry balance errors
                _ => true,
            },
            
            // Risk errors are not retryable
            KartalError::Risk(_) => false,
            
            // Validation errors are not retryable
            KartalError::Validation(_) => false,
            
            // Database errors might be retryable
            KartalError::DatabaseError(_) => true,
            
            // Default to retryable for unknown errors
            _ => true,
        }
    }
    
    /// Calculate delay with exponential backoff and jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay_ms as f64 
            * self.config.backoff_multiplier.powi(attempt as i32 - 1);
        
        let capped_delay = base_delay.min(self.config.max_delay_ms as f64);
        
        // Add jitter
        let jitter_range = capped_delay * self.config.jitter_factor;
        let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
        
        let final_delay = (capped_delay + jitter).max(0.0) as u64;
        
        Duration::from_millis(final_delay)
    }
    
    /// Map provider errors to KartalError
    fn map_provider_error(&self, err: ProviderError) -> KartalError {
        match &err {
            ProviderError::JsonRpcClientError(rpc_err) => {
                let err_str = rpc_err.to_string();
                if err_str.contains("nonce too low") {
                    KartalError::Execution(ExecutionError::NonceTooLow {
                        expected: U256::zero(),
                        actual: U256::zero(),
                    })
                } else if err_str.contains("insufficient funds") {
                    KartalError::Execution(ExecutionError::InsufficientBalance {
                        required: U256::zero(),
                        available: U256::zero(),
                    })
                } else {
                    KartalError::Network(crate::common::errors::NetworkError::ProviderError(err))
                }
            }
            _ => KartalError::Network(crate::common::errors::NetworkError::ProviderError(err)),
        }
    }
}

/// Retry a specific operation with custom config
pub async fn retry<F, Fut, T>(
    config: RetryConfig,
    operation_name: &str,
    operation: F,
) -> Result<T, KartalError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, KartalError>>,
{
    let executor = RetryExecutor::new(config);
    executor.execute_with_retry(operation_name, operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_success() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            ..Default::default()
        };
        
        let executor = RetryExecutor::new(config);
        let mut attempts = 0;
        
        let result = executor.execute_with_retry("test operation", || async {
            attempts += 1;
            if attempts < 2 {
                Err(KartalError::Generic("Temporary failure".to_string()))
            } else {
                Ok("Success")
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }
    
    #[tokio::test]
    async fn test_non_retryable_error() {
        let config = RetryConfig::default();
        let executor = RetryExecutor::new(config);
        
        let result = executor.execute_with_retry("test operation", || async {
            Err(KartalError::Validation("Invalid input".to_string()))
        }).await;
        
        assert!(result.is_err());
    }
}