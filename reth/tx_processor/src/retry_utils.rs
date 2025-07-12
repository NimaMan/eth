/// Retry utilities for handling transient MDBX errors
use std::time::Duration;
use std::thread;
use eyre::Result;
use tracing::{warn, debug};

/// Retry configuration for database operations
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}

/// Execute a closure with retry logic for EAGAIN errors
pub fn retry_on_eagain<F, T>(mut f: F, config: &RetryConfig) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;
    
    loop {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_str = e.to_string();
                
                // Check if this is an EAGAIN error (code 11)
                if error_str.contains("error code: 11") || 
                   error_str.contains("EAGAIN") ||
                   error_str.contains("resource temporarily unavailable") {
                    
                    attempt += 1;
                    
                    if attempt > config.max_retries {
                        warn!("Max retries ({}) exceeded for EAGAIN error", config.max_retries);
                        return Err(e);
                    }
                    
                    warn!(
                        "EAGAIN error encountered (attempt {}/{}), retrying in {:?}",
                        attempt, config.max_retries, delay
                    );
                    
                    // Sleep before retry
                    thread::sleep(delay);
                    
                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_multiplier).min(config.max_delay.as_secs_f64())
                    );
                    
                    continue;
                }
                
                // Not an EAGAIN error, return immediately
                return Err(e);
            }
        }
    }
}

/// Async version of retry_on_eagain
pub async fn retry_on_eagain_async<F, Fut, T>(mut f: F, config: &RetryConfig) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;
    
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_str = e.to_string();
                
                // Check if this is an EAGAIN error (code 11)
                if error_str.contains("error code: 11") || 
                   error_str.contains("EAGAIN") ||
                   error_str.contains("resource temporarily unavailable") {
                    
                    attempt += 1;
                    
                    if attempt > config.max_retries {
                        warn!("Max retries ({}) exceeded for EAGAIN error", config.max_retries);
                        return Err(e);
                    }
                    
                    debug!(
                        "EAGAIN error encountered (attempt {}/{}), retrying in {:?}",
                        attempt, config.max_retries, delay
                    );
                    
                    // Async sleep before retry
                    tokio::time::sleep(delay).await;
                    
                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_multiplier).min(config.max_delay.as_secs_f64())
                    );
                    
                    continue;
                }
                
                // Not an EAGAIN error, return immediately
                return Err(e);
            }
        }
    }
}