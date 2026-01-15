//! Resilience and error recovery mechanisms for production use

use crate::{QarqaError, QarqaResult};
use std::time::Duration;
use tracing::{warn, error, info};

/// Retry configuration for resilient operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 - 1.0)
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

/// Circuit breaker for protecting external dependencies
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Failure threshold before opening circuit
    pub failure_threshold: u32,
    /// Success threshold before closing circuit
    pub success_threshold: u32,
    /// Duration to wait before attempting half-open
    pub timeout: Duration,
    /// Current state
    state: CircuitBreakerState,
    /// Failure count
    failure_count: u32,
    /// Success count
    success_count: u32,
    /// Last state change time
    last_state_change: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_state_change: std::time::Instant::now(),
        }
    }
    
    /// Check if operation should be allowed
    pub fn should_allow(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if self.last_state_change.elapsed() >= self.timeout {
                    info!("Circuit breaker moving to half-open state");
                    self.state = CircuitBreakerState::HalfOpen;
                    self.last_state_change = std::time::Instant::now();
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }
    
    /// Record successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    info!("Circuit breaker closing after successful recovery");
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_state_change = std::time::Instant::now();
                }
            }
            CircuitBreakerState::Open => {
                // Should not happen
                warn!("Success recorded while circuit is open");
            }
        }
    }
    
    /// Record failed operation
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    error!("Circuit breaker opening due to {} failures", self.failure_count);
                    self.state = CircuitBreakerState::Open;
                    self.last_state_change = std::time::Instant::now();
                }
            }
            CircuitBreakerState::HalfOpen => {
                error!("Circuit breaker reopening due to failure in half-open state");
                self.state = CircuitBreakerState::Open;
                self.failure_count = 0;
                self.success_count = 0;
                self.last_state_change = std::time::Instant::now();
            }
            CircuitBreakerState::Open => {
                // Already open, ignore
            }
        }
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_with_backoff<F, T, Fut>(
    config: &RetryConfig,
    mut operation: F,
) -> QarqaResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = QarqaResult<T>>,
{
    let mut attempt = 0;
    let mut backoff = config.initial_backoff;
    
    loop {
        attempt += 1;
        
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts => {
                error!("All {} retry attempts failed: {}", config.max_attempts, e);
                return Err(e);
            }
            Err(e) => {
                warn!("Attempt {}/{} failed: {}. Retrying in {:?}", 
                      attempt, config.max_attempts, e, backoff);
                
                // Add jitter
                let jitter = if config.jitter_factor > 0.0 {
                    let jitter_range = backoff.as_secs_f64() * config.jitter_factor;
                    let jitter = rand::random::<f64>() * jitter_range - (jitter_range / 2.0);
                    Duration::from_secs_f64(jitter.abs())
                } else {
                    Duration::ZERO
                };
                
                tokio::time::sleep(backoff + jitter).await;
                
                // Exponential backoff
                backoff = Duration::from_secs_f64(
                    (backoff.as_secs_f64() * config.backoff_multiplier)
                        .min(config.max_backoff.as_secs_f64())
                );
            }
        }
    }
}

/// Fallback handler for degraded operations
pub trait FallbackHandler<T> {
    /// Provide fallback value when primary operation fails
    fn fallback(&self, error: &QarqaError) -> QarqaResult<T>;
}

/// Default fallback that just propagates the error
pub struct NoFallback;

impl<T> FallbackHandler<T> for NoFallback {
    fn fallback(&self, error: &QarqaError) -> QarqaResult<T> {
        Err(error.clone())
    }
}

/// Timeout wrapper for async operations
pub async fn with_timeout<F, T>(
    duration: Duration,
    future: F,
) -> QarqaResult<T>
where
    F: std::future::Future<Output = QarqaResult<T>>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(QarqaError::Timeout(format!(
            "Operation timed out after {:?}", duration
        ))),
    }
}

/// Rate limiter for protecting resources
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum requests per window
    max_requests: u32,
    /// Time window
    window: Duration,
    /// Request timestamps
    requests: std::collections::VecDeque<std::time::Instant>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: std::collections::VecDeque::new(),
        }
    }
    
    /// Check if request should be allowed
    pub fn should_allow(&mut self) -> bool {
        let now = std::time::Instant::now();
        
        // Remove old requests outside the window
        while let Some(&front) = self.requests.front() {
            if now.duration_since(front) > self.window {
                self.requests.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we can allow new request
        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }
    
    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> u32 {
        let current = self.requests.len() as u32;
        self.max_requests.saturating_sub(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(2, 2, Duration::from_millis(100));
        
        // Initially closed
        assert!(cb.should_allow());
        
        // Record failures
        cb.record_failure();
        assert!(cb.should_allow()); // Still closed
        
        cb.record_failure();
        assert!(!cb.should_allow()); // Now open
        
        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));
        assert!(cb.should_allow()); // Half-open
        
        // Success in half-open
        cb.record_success();
        cb.record_success();
        assert!(cb.should_allow()); // Closed again
    }
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(1));
        
        // Allow first 3 requests
        assert!(limiter.should_allow());
        assert!(limiter.should_allow());
        assert!(limiter.should_allow());
        
        // Deny 4th request
        assert!(!limiter.should_allow());
        
        // Wait and try again
        std::thread::sleep(Duration::from_secs(1));
        assert!(limiter.should_allow());
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
        };
        
        let mut attempts = 0;
        let result = retry_with_backoff(&config, || async {
            attempts += 1;
            if attempts < 3 {
                Err(QarqaError::Network("Simulated failure".to_string()))
            } else {
                Ok(42)
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }
}