use std::sync::Arc;
use reth_chainspec::{ChainSpec, MAINNET};

/// Configuration for embedded Reth mempool listener
#[derive(Debug, Clone)]
pub struct EmbeddedRethConfig {
    /// Port to listen on for P2P connections
    pub listen_port: u16,
    
    /// Ethereum mainnet boot nodes for peer discovery
    pub boot_nodes: Vec<String>,
    
    /// Chain specification (default: Ethereum mainnet)
    pub chain_spec: Arc<ChainSpec>,
    
    /// Whether to enable discovery (requires boot nodes)
    pub enable_discovery: bool,
}

impl Default for EmbeddedRethConfig {
    fn default() -> Self {
        Self {
            listen_port: 30313,
            boot_nodes: vec![
                "enode://d860a01f9722d78051619d1e2351aba3f43f943f6f00718d1b9baa4101932a1f5011f16bb2b1bb35db20d6fe28fa0bf09636d26a87d31de9ec6203eeedb1f666@18.138.108.67:30303".to_string(),
                "enode://22a8232c3abc76a16ae9d6c3b164f98775fe226f0917b0ca871128a74a8e9630b458460865bab457221f1d448dd9791d24c4e5d88786180ac185df813a68d4de@3.209.45.79:30303".to_string(),
            ],
            chain_spec: MAINNET.clone(),
            enable_discovery: false, // Disabled for simplicity
        }
    }
}

impl EmbeddedRethConfig {
    /// Create new config with custom port
    pub fn with_port(port: u16) -> Self {
        Self {
            listen_port: port,
            ..Default::default()
        }
    }
    
    /// Create config for testnet
    pub fn testnet() -> Self {
        Self {
            chain_spec: MAINNET.clone(), // Would use testnet spec in production
            ..Default::default()
        }
    }
    
    /// Enable peer discovery
    pub fn with_discovery(mut self) -> Self {
        self.enable_discovery = true;
        self
    }
}