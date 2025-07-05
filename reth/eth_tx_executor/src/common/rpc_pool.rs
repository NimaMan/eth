//! RPC Connection Pool with Failover
//!
//! Provides reliable RPC access with health checking and automatic failover

use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use crate::common::errors::{KartalError, NetworkError};

#[derive(Debug, Clone)]
pub struct RpcEndpoint {
    pub url: String,
    pub priority: u8,  // Lower is better
    pub max_requests_per_second: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RpcPoolConfig {
    pub endpoints: Vec<RpcEndpoint>,
    pub health_check_interval: Duration,
    pub request_timeout: Duration,
    pub max_consecutive_failures: u32,
}

impl Default for RpcPoolConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![
                RpcEndpoint {
                    url: "http://127.0.0.1:8545".to_string(),
                    priority: 0,
                    max_requests_per_second: None,
                },
            ],
            health_check_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(10),
            max_consecutive_failures: 3,
        }
    }
}

#[derive(Debug)]
struct EndpointHealth {
    endpoint: RpcEndpoint,
    provider: Arc<Provider<Http>>,
    is_healthy: bool,
    consecutive_failures: u32,
    last_check: std::time::Instant,
    latency_ms: Option<u64>,
}

/// RPC connection pool with automatic failover
pub struct RpcPool {
    config: RpcPoolConfig,
    endpoints: Arc<RwLock<Vec<EndpointHealth>>>,
    current_index: Arc<RwLock<usize>>,
}

impl RpcPool {
    pub async fn new(config: RpcPoolConfig) -> Result<Self, KartalError> {
        if config.endpoints.is_empty() {
            return Err(KartalError::Config(crate::common::errors::ConfigError::MissingField {
                field: "endpoints".to_string(),
            }));
        }
        
        // Initialize providers
        let mut endpoint_health = Vec::new();
        
        for endpoint in config.endpoints.iter() {
            let provider = Provider::<Http>::try_from(&endpoint.url)
                .map_err(|e| KartalError::Network(NetworkError::RpcError(e.to_string())))?
                .interval(Duration::from_millis(100));
            
            endpoint_health.push(EndpointHealth {
                endpoint: endpoint.clone(),
                provider: Arc::new(provider),
                is_healthy: true,  // Assume healthy initially
                consecutive_failures: 0,
                last_check: std::time::Instant::now(),
                latency_ms: None,
            });
        }
        
        // Sort by priority
        endpoint_health.sort_by_key(|e| e.endpoint.priority);
        
        let pool = Self {
            config,
            endpoints: Arc::new(RwLock::new(endpoint_health)),
            current_index: Arc::new(RwLock::new(0)),
        };
        
        // Start health checker
        pool.start_health_checker();
        
        // Do initial health check
        pool.check_all_endpoints().await;
        
        Ok(pool)
    }
    
    /// Get a healthy provider
    pub async fn get_provider(&self) -> Result<Arc<Provider<Http>>, KartalError> {
        let endpoints = self.endpoints.read().await;
        
        // Try current provider first
        let current = *self.current_index.read().await;
        if current < endpoints.len() && endpoints[current].is_healthy {
            return Ok(endpoints[current].provider.clone());
        }
        
        // Find first healthy provider
        for (idx, endpoint) in endpoints.iter().enumerate() {
            if endpoint.is_healthy {
                *self.current_index.write().await = idx;
                info!("Switched to RPC endpoint: {}", endpoint.endpoint.url);
                return Ok(endpoint.provider.clone());
            }
        }
        
        // No healthy providers - try to recover the best one
        drop(endpoints);
        self.check_all_endpoints().await;
        
        let endpoints = self.endpoints.read().await;
        let best = endpoints.iter()
            .min_by_key(|e| (e.consecutive_failures, e.endpoint.priority))
            .ok_or_else(|| KartalError::Network(NetworkError::ConnectionTimeout))?;
        
        warn!("Using potentially unhealthy endpoint {} as last resort", best.endpoint.url);
        Ok(best.provider.clone())
    }
    
    /// Execute with automatic failover
    pub async fn execute_with_failover<F, Fut, T>(&self, operation: F) -> Result<T, KartalError>
    where
        F: Fn(Arc<Provider<Http>>) -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        let mut last_error = None;
        let endpoints = self.endpoints.read().await;
        
        // Try each healthy endpoint
        for (idx, endpoint) in endpoints.iter().enumerate() {
            if !endpoint.is_healthy && endpoint.consecutive_failures >= self.config.max_consecutive_failures {
                continue;
            }
            
            let provider = endpoint.provider.clone();
            drop(endpoints);  // Release lock before async operation
            
            match tokio::time::timeout(
                self.config.request_timeout,
                operation(provider)
            ).await {
                Ok(Ok(result)) => {
                    // Mark endpoint as healthy
                    self.mark_endpoint_result(idx, true).await;
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    warn!("RPC request failed on {}: {}", 
                          self.endpoints.read().await[idx].endpoint.url, e);
                    last_error = Some(NetworkError::ProviderError(e));
                    self.mark_endpoint_result(idx, false).await;
                }
                Err(_) => {
                    warn!("RPC request timed out on {}", 
                          self.endpoints.read().await[idx].endpoint.url);
                    last_error = Some(NetworkError::ConnectionTimeout);
                    self.mark_endpoint_result(idx, false).await;
                }
            }
            
            // Re-acquire lock for next iteration
            let endpoints_guard = self.endpoints.read().await;
        }
        
        Err(KartalError::Network(last_error.unwrap_or(NetworkError::ConnectionTimeout)))
    }
    
    /// Mark endpoint result
    async fn mark_endpoint_result(&self, index: usize, success: bool) {
        let mut endpoints = self.endpoints.write().await;
        if index < endpoints.len() {
            if success {
                endpoints[index].consecutive_failures = 0;
                endpoints[index].is_healthy = true;
            } else {
                endpoints[index].consecutive_failures += 1;
                if endpoints[index].consecutive_failures >= self.config.max_consecutive_failures {
                    endpoints[index].is_healthy = false;
                    error!("Endpoint {} marked unhealthy after {} failures", 
                           endpoints[index].endpoint.url,
                           endpoints[index].consecutive_failures);
                }
            }
        }
    }
    
    /// Start background health checker
    fn start_health_checker(&self) {
        let endpoints = self.endpoints.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                // Check each endpoint
                let mut endpoints_guard = endpoints.write().await;
                for endpoint in endpoints_guard.iter_mut() {
                    let start = std::time::Instant::now();
                    
                    match tokio::time::timeout(
                        Duration::from_secs(5),
                        endpoint.provider.get_block_number()
                    ).await {
                        Ok(Ok(block)) => {
                            let latency = start.elapsed().as_millis() as u64;
                            endpoint.latency_ms = Some(latency);
                            endpoint.is_healthy = true;
                            endpoint.consecutive_failures = 0;
                            endpoint.last_check = std::time::Instant::now();
                            
                            if endpoint.consecutive_failures > 0 {
                                info!("Endpoint {} recovered (block: {}, latency: {}ms)", 
                                      endpoint.endpoint.url, block, latency);
                            }
                        }
                        Ok(Err(e)) => {
                            endpoint.consecutive_failures += 1;
                            endpoint.is_healthy = false;
                            endpoint.last_check = std::time::Instant::now();
                            warn!("Health check failed for {}: {}", endpoint.endpoint.url, e);
                        }
                        Err(_) => {
                            endpoint.consecutive_failures += 1;
                            endpoint.is_healthy = false;
                            endpoint.last_check = std::time::Instant::now();
                            warn!("Health check timed out for {}", endpoint.endpoint.url);
                        }
                    }
                }
            }
        });
    }
    
    /// Check all endpoints immediately
    async fn check_all_endpoints(&self) {
        let mut endpoints = self.endpoints.write().await;
        
        for endpoint in endpoints.iter_mut() {
            let start = std::time::Instant::now();
            
            match tokio::time::timeout(
                Duration::from_secs(5),
                endpoint.provider.get_block_number()
            ).await {
                Ok(Ok(_)) => {
                    endpoint.latency_ms = Some(start.elapsed().as_millis() as u64);
                    endpoint.is_healthy = true;
                    endpoint.consecutive_failures = 0;
                }
                _ => {
                    endpoint.is_healthy = false;
                }
            }
            
            endpoint.last_check = std::time::Instant::now();
        }
    }
    
    /// Get current pool status
    pub async fn get_status(&self) -> PoolStatus {
        let endpoints = self.endpoints.read().await;
        
        PoolStatus {
            total_endpoints: endpoints.len(),
            healthy_endpoints: endpoints.iter().filter(|e| e.is_healthy).count(),
            current_endpoint: endpoints.get(*self.current_index.read().await)
                .map(|e| e.endpoint.url.clone()),
            endpoint_stats: endpoints.iter().map(|e| EndpointStatus {
                url: e.endpoint.url.clone(),
                is_healthy: e.is_healthy,
                consecutive_failures: e.consecutive_failures,
                latency_ms: e.latency_ms,
            }).collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub total_endpoints: usize,
    pub healthy_endpoints: usize,
    pub current_endpoint: Option<String>,
    pub endpoint_stats: Vec<EndpointStatus>,
}

#[derive(Debug, Clone)]
pub struct EndpointStatus {
    pub url: String,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub latency_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pool_creation() {
        let config = RpcPoolConfig::default();
        let pool = RpcPool::new(config).await.unwrap();
        let status = pool.get_status().await;
        assert_eq!(status.total_endpoints, 1);
    }
}