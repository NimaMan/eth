/// Core transaction simulator implementation
/// 
/// This module contains the main TxSimulator struct and its core methods
/// for initialization and database access.

use std::sync::Arc;
use std::path::Path;
use eyre::Result;

// Core Reth imports
use reth_chainspec::{ChainSpecBuilder, ChainSpecProvider};
use reth_db::{open_db_read_only, mdbx::DatabaseArguments, ClientVersion, DatabaseEnv};
use reth_provider::{ProviderFactory, BlockNumReader, HeaderProvider, StateProvider, providers::StaticFileProvider};
use reth_node_types::NodeTypesWithDBAdapter;
use reth_node_ethereum::{EthereumNode, EthEvmConfig};

/// Transaction Simulator with direct database access
#[derive(Clone)]
pub struct TxSimulator {
    pub(crate) provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    pub(crate) evm_config: EthEvmConfig,
}

impl TxSimulator {
    /// Create new simulator with direct database access
    /// 
    /// # Arguments
    /// * `reth_datadir` - Path to Reth data directory (e.g., `/home/user/.local/share/reth/mainnet`)
    /// 
    /// # Returns
    /// * `Result<Self>` - New simulator instance or error
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let datadir = Path::new(reth_datadir);
        let db_path = datadir.join("db");
        let static_files_path = datadir.join("static_files");
        
        
        // Open database read-only
        let db = Arc::new(open_db_read_only(
            db_path.as_path(),
            DatabaseArguments::new(ClientVersion::default()),
        )?);
        
        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());
        
        let provider_factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
            db.clone(),
            chain_spec.clone(),
            StaticFileProvider::read_only(static_files_path, false)?,
        );
        
        let evm_config = EthEvmConfig::new(chain_spec.clone());
        
        
        Ok(Self {
            provider_factory,
            evm_config,
        })
    }
    
    /// Create new simulator with an existing provider factory
    /// 
    /// This is useful when you want to share a database connection across multiple components
    pub fn with_provider_factory(
        provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>
    ) -> Result<Self> {
        let chain_spec = provider_factory.chain_spec();
        let evm_config = EthEvmConfig::new(chain_spec);
        
        
        Ok(Self {
            provider_factory,
            evm_config,
        })
    }
    
    /// Get latest block number from local database
    pub fn get_latest_block(&self) -> Result<u64> {
        let provider = self.provider_factory.provider()?;
        let block_number = provider.best_block_number()?;
        Ok(block_number)
    }
    
    /// Get the provider factory for direct database access
    pub fn provider_factory(&self) -> &ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>> {
        &self.provider_factory
    }
    
    /// Get the base fee for a specific block
    /// 
    /// This is needed for EIP-1559 transactions to ensure gas prices are set correctly.
    /// Returns the base fee in wei.
    pub fn get_base_fee_at_block(&self, block_number: u64) -> Result<u128> {
        let provider = self.provider_factory.provider()?;
        let header = provider.header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block whilst getting base-fee {}", block_number))?;
        
        let base_fee = header.base_fee_per_gas
            .ok_or_else(|| eyre::eyre!("No base fee for block {} (pre-London?)", block_number))?;
        
        Ok(base_fee as u128)
    }
    
    /// Get chain state at a specific block
    /// Returns a StateProvider that gives access to all blockchain state at that block
    pub fn get_chain_state_at_block(&self, block_number: u64) -> Result<Box<dyn StateProvider>> {
        let state = self.provider_factory.history_by_block_number(block_number)?;
        Ok(state)
    }
    
    // === Block Simulation Methods (temporarily disabled) ===
    
    // Block simulation methods are temporarily disabled until the module is fixed
    // with proper revm imports and SystemCaller implementation.
    // These will be re-enabled once compilation issues are resolved.
    
    /// Get block metadata (timestamp, gas_limit, gas_used, base_fee)
    pub fn get_block_metadata(&self, block_number: u64) -> Result<(u64, u64, u64, Option<u128>)> {
        let provider = self.provider_factory.provider()?;
        let header = provider.header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block whilst getting block metadata {}", block_number))?;
        
        Ok((
            header.timestamp,
            header.gas_limit,
            header.gas_used,
            header.base_fee_per_gas.map(|v| v as u128),
        ))
    }
    
}

// Alias for compatibility
pub type RethTxSimulator = TxSimulator;
