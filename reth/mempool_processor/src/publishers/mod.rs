//! Unified Publisher Module
//! 
//! This module provides a unified publishing system for all signal types in the mempool processor.
//! It consolidates multiple disparate publishers into a single, coherent system with:
//! - Common signal base structure
//! - Topic-based routing
//! - Multipart ZMQ messaging
//! - Extensible signal types

pub mod message_types;
pub mod unified_publisher;
pub mod routing;
pub mod config;

pub use message_types::*;
pub use unified_publisher::UnifiedPublisher;
pub use routing::SignalRouter;
pub use config::PublisherConfig;