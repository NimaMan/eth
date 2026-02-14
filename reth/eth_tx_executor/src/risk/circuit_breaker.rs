//! Circuit Breaker Module
//!
//! Implements circuit breaker pattern for transaction execution safety

use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    /// Normal operation - all transactions allowed
    Closed,
    /// Partial failure mode - reduced operation
    HalfOpen,
    /// Complete failure mode - no transactions allowed
    Open,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening
    pub failure_threshold: u32,
    /// Time window for failure counting (seconds)
    pub failure_window_seconds: u64,
    /// Timeout before attempting to close from open state (seconds)
    pub recovery_timeout_seconds: u64,
    /// Success threshold to close from half-open
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,          // 5 failures
            failure_window_seconds: 300,   // in 5 minutes
            recovery_timeout_seconds: 600, // wait 10 minutes
            success_threshold: 3,          // 3 successes to recover
        }
    }
}

/// Failure tracking
#[derive(Debug, Clone)]
struct FailureRecord {
    timestamp: u64,
    reason: String,
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitBreakerState,
    failures: Vec<FailureRecord>,
    consecutive_successes: u32,
    last_state_change: u64,
    total_requests: u64,
    total_failures: u64,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitBreakerState::Closed,
            failures: Vec::new(),
            consecutive_successes: 0,
            last_state_change: Self::get_current_timestamp(),
            total_requests: 0,
            total_failures: 0,
        }
    }

    /// Check if request should be allowed
    pub fn allow_request(&mut self) -> bool {
        self.total_requests += 1;
        self.cleanup_old_failures();

        match self.state {
            CircuitBreakerState::Closed => {
                // Check if we should transition to open
                if self.failures.len() >= self.config.failure_threshold as usize {
                    self.transition_to_open("Failure threshold exceeded");
                    false
                } else {
                    true
                }
            }

            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                let now = Self::get_current_timestamp();
                if now - self.last_state_change >= self.config.recovery_timeout_seconds {
                    self.transition_to_half_open();
                    true
                } else {
                    false
                }
            }

            CircuitBreakerState::HalfOpen => {
                // Allow limited requests to test recovery
                true
            }
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::HalfOpen => {
                self.consecutive_successes += 1;

                if self.consecutive_successes >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }

            CircuitBreakerState::Closed => {
                // Reset consecutive successes in closed state
                self.consecutive_successes = 0;
            }

            CircuitBreakerState::Open => {
                // Shouldn't happen, but handle gracefully
                warn!("Received success while circuit breaker is open");
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&mut self, reason: String) {
        self.total_failures += 1;

        let now = Self::get_current_timestamp();
        self.failures.push(FailureRecord {
            timestamp: now,
            reason: reason.clone(),
        });

        // Reset consecutive successes on any failure
        self.consecutive_successes = 0;

        match self.state {
            CircuitBreakerState::Closed => {
                if self.failures.len() >= self.config.failure_threshold as usize {
                    self.transition_to_open(&reason);
                }
            }

            CircuitBreakerState::HalfOpen => {
                // Any failure in half-open immediately goes back to open
                self.transition_to_open(&reason);
            }

            CircuitBreakerState::Open => {
                // Already open, just record the failure
                error!("Additional failure while circuit breaker open: {}", reason);
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitBreakerState {
        self.state.clone()
    }

    /// Get statistics
    pub fn get_stats(&self) -> CircuitBreakerStats {
        let now = Self::get_current_timestamp();
        let recent_failures = self
            .failures
            .iter()
            .filter(|f| now - f.timestamp <= self.config.failure_window_seconds)
            .count();

        let failure_rate = if self.total_requests > 0 {
            self.total_failures as f64 / self.total_requests as f64
        } else {
            0.0
        };

        CircuitBreakerStats {
            state: self.state.clone(),
            recent_failures: recent_failures as u32,
            total_failures: self.total_failures,
            total_requests: self.total_requests,
            failure_rate,
            consecutive_successes: self.consecutive_successes,
            time_in_current_state: now - self.last_state_change,
        }
    }

    /// Force circuit breaker to open state
    pub fn force_open(&mut self, reason: String) {
        self.transition_to_open(&reason);
        warn!("Circuit breaker manually forced open: {}", reason);
    }

    /// Force circuit breaker to closed state (manual recovery)
    pub fn force_close(&mut self) {
        self.transition_to_closed();
        self.failures.clear();
        warn!("Circuit breaker manually forced closed");
    }

    fn transition_to_open(&mut self, reason: &str) {
        if self.state != CircuitBreakerState::Open {
            self.state = CircuitBreakerState::Open;
            self.last_state_change = Self::get_current_timestamp();
            self.consecutive_successes = 0;

            error!(
                "🔴 Circuit breaker OPENED: {} (failures: {}/{})",
                reason,
                self.failures.len(),
                self.config.failure_threshold
            );
        }
    }

    fn transition_to_half_open(&mut self) {
        self.state = CircuitBreakerState::HalfOpen;
        self.last_state_change = Self::get_current_timestamp();
        self.consecutive_successes = 0;

        info!("🟡 Circuit breaker HALF-OPEN: Testing recovery");
    }

    fn transition_to_closed(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.last_state_change = Self::get_current_timestamp();
        self.consecutive_successes = 0;
        self.failures.clear(); // Clear failure history on successful recovery

        info!("🟢 Circuit breaker CLOSED: Normal operation resumed");
    }

    fn cleanup_old_failures(&mut self) {
        let now = Self::get_current_timestamp();
        let cutoff = now - self.config.failure_window_seconds;

        self.failures.retain(|failure| failure.timestamp > cutoff);
    }

    fn get_current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs()
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitBreakerState,
    pub recent_failures: u32,
    pub total_failures: u64,
    pub total_requests: u64,
    pub failure_rate: f64,
    pub consecutive_successes: u32,
    pub time_in_current_state: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };

        let mut breaker = CircuitBreaker::new(config);

        // Should allow requests in closed state
        assert!(breaker.allow_request());
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window_seconds: 60,
            ..Default::default()
        };

        let mut breaker = CircuitBreaker::new(config);

        // Record failures
        breaker.allow_request();
        breaker.record_failure("Test failure 1".to_string());

        breaker.allow_request();
        breaker.record_failure("Test failure 2".to_string());

        // Should be open now
        assert!(!breaker.allow_request());
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);
    }

    #[test]
    fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            failure_window_seconds: 60,
            recovery_timeout_seconds: 1, // Short timeout for testing
            ..Default::default()
        };

        let mut breaker = CircuitBreaker::new(config);

        // Trigger open state
        breaker.allow_request();
        breaker.record_failure("Test failure".to_string());
        assert_eq!(breaker.get_state(), CircuitBreakerState::Open);

        // Wait for recovery timeout
        thread::sleep(Duration::from_secs(2));

        // Should transition to half-open
        assert!(breaker.allow_request());
        assert_eq!(breaker.get_state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            recovery_timeout_seconds: 1,
            ..Default::default()
        };

        let mut breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.allow_request();
        breaker.record_failure("Test failure".to_string());

        // Wait and transition to half-open
        thread::sleep(Duration::from_secs(2));
        breaker.allow_request();

        // Record successful operations
        breaker.record_success();
        breaker.record_success();

        // Should be closed now
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_failure_cleanup() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window_seconds: 1, // Very short window
            ..Default::default()
        };

        let mut breaker = CircuitBreaker::new(config);

        // Record failures
        breaker.allow_request();
        breaker.record_failure("Old failure".to_string());

        // Wait for failure to expire
        thread::sleep(Duration::from_secs(2));

        // Should still be closed because old failures are cleaned up
        assert!(breaker.allow_request());
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_stats_collection() {
        let config = CircuitBreakerConfig::default();
        let mut breaker = CircuitBreaker::new(config);

        // Make some requests
        for i in 0..10 {
            breaker.allow_request();
            if i % 3 == 0 {
                breaker.record_failure(format!("Failure {}", i));
            } else {
                breaker.record_success();
            }
        }

        let stats = breaker.get_stats();
        assert_eq!(stats.total_requests, 10);
        assert!(stats.total_failures > 0);
        assert!(stats.failure_rate > 0.0);
    }
}
