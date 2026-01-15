//! Risk Management Module
//! 
//! Provides essential risk checks for trade execution

pub mod manager;
pub mod mev_protection;
pub mod circuit_breaker;
pub mod simulation;
pub mod state_persistence;

pub use manager::{RiskManager, RiskConfig, RiskDecision};