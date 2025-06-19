//! Strategy Module
//! 
//! Implements decision-making logic for responding to scam alerts with
//! appropriate trading strategies.

pub mod decision_engine;

pub use decision_engine::{DecisionEngine, DecisionConfig, TradingDecision};