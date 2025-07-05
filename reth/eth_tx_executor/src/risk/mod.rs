//! Risk Management Module
//! 
//! Provides circuit breakers, loss limits, and MEV protection

pub mod manager;
pub mod mev_protection;
pub mod circuit_breaker;
pub mod simulation;
pub mod state_persistence;

pub use manager::{RiskManager, RiskConfig, RiskDecision};
pub use mev_protection::{MEVProtector, MEVStrategy, PriorityConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
pub use simulation::{SimulationEngine, SimulationMode, SimulationResult};