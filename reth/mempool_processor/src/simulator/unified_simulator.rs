/// Unified Simulator that manages both single and sequential transaction simulations
/// 
/// This module solves the database lock conflict (error code 11) by creating a single
/// provider factory and sharing it between both simulators.
///
/// The unified simulator provides a simple interface to:
/// - Simulate single transactions
/// - Simulate buy/sell sequences
/// - All using the same database connection

use eyre::Result;
use std::sync::Arc;
use std::path::Path;
use tracing::info;
use reth_provider::ProviderFactory;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_node_ethereum::EthereumNode;
use reth_db::DatabaseEnv;
use reth_chainspec::ChainSpecBuilder;
use reth_provider::providers::StaticFileProvider;
use alloy_primitives::Address;

use reth_tx_simulator::{RethTxSimulator, CallRequest};
use crate::mempool_fetcher::FullTransaction;
use super::sequential_tx_simulator::{
    SequentialBuySellSimulator, BuySellSimulatorConfig, SequenceSimulationResult
};
use super::single_tx_simulator::{SimulationResult, StateChangeResult};

/// Unified simulator that manages both single and sequential simulations
pub struct UnifiedSimulator {
    /// The underlying Reth simulator for single transactions
    single_simulator: RethTxSimulator,
    /// The sequential buy/sell simulator
    sequential_simulator: SequentialBuySellSimulator,
}

impl UnifiedSimulator {
    /// Create a new unified simulator
    pub fn new(datadir: &str) -> Result<Self> {
        info!("Initializing unified simulator...");
        
        // Open the database once
        let db_path = Path::new(datadir).join("db");
        let db = Arc::new(reth_db::open_db_read_only(
            &db_path,
            Default::default()
        )?);
        
        // Create static file provider
        let static_files_path = Path::new(datadir).join("static_files");
        let static_file_provider = StaticFileProvider::read_only(static_files_path, true)?;
        
        // Create chain spec
        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());
        
        // Create a single provider factory
        let provider_factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
            db.clone(),
            chain_spec.clone(),
            static_file_provider,
        );
        
        // Create both simulators using the same provider factory
        let single_simulator = RethTxSimulator::with_provider_factory(provider_factory.clone())?;
        let sequential_simulator = RethTxSimulator::with_provider_factory(provider_factory)?;
        
        // Wrap the sequential simulator
        let sequential_simulator = SequentialBuySellSimulator::with_existing_simulator(
            Arc::new(sequential_simulator),
            BuySellSimulatorConfig::default()
        );
        
        info!("✅ Unified simulator initialized");
        
        Ok(Self {
            single_simulator,
            sequential_simulator,
        })
    }
    
    /// Create with custom buy/sell configuration
    pub fn with_config(datadir: &str, buy_sell_config: BuySellSimulatorConfig) -> Result<Self> {
        info!("Initializing unified simulator with custom config...");
        
        // Open the database once
        let db_path = Path::new(datadir).join("db");
        let db = Arc::new(reth_db::open_db_read_only(
            &db_path,
            Default::default()
        )?);
        
        // Create static file provider
        let static_files_path = Path::new(datadir).join("static_files");
        let static_file_provider = StaticFileProvider::read_only(static_files_path, true)?;
        
        // Create chain spec
        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());
        
        // Create a single provider factory
        let provider_factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
            db.clone(),
            chain_spec.clone(),
            static_file_provider,
        );
        
        // Create both simulators using the same provider factory
        let single_simulator = RethTxSimulator::with_provider_factory(provider_factory.clone())?;
        let sequential_simulator = RethTxSimulator::with_provider_factory(provider_factory)?;
        
        // Wrap the sequential simulator with custom config
        let sequential_simulator = SequentialBuySellSimulator::with_existing_simulator(
            Arc::new(sequential_simulator),
            buy_sell_config
        );
        
        info!("✅ Unified simulator initialized with custom config");
        
        Ok(Self {
            single_simulator,
            sequential_simulator,
        })
    }
    
    /// Get the latest block number
    pub fn get_latest_block(&self) -> Result<u64> {
        self.single_simulator.get_latest_block()
    }
    
    // ========== Single Transaction Simulation Methods ==========
    
    /// Simulate a single transaction from mempool
    pub async fn simulate_single_tx(&self, tx: &FullTransaction) -> Result<SimulationResult> {
        use crate::common::convert::ipc_to_call_request;
        
        // Convert IPC transaction to CallRequest
        let call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block()?;
        
        // Simulate the transaction
        let result = self.single_simulator
            .simulate_unsigned_transaction_at_block(call_request, latest_block)
            .await?;
            
        Ok(SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
            simulation_time_us: 0, // Timer handled externally
        })
    }
    
    /// Simulate transaction and get state changes
    pub async fn simulate_with_state_changes(&self, tx: &FullTransaction) -> Result<StateChangeResult> {
        use crate::common::convert::ipc_to_call_request;
        
        let call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block()?;
        
        let state_changes = self.single_simulator
            .simulate_unsigned_transaction_with_call_trace_at_block(call_request, latest_block)
            .await?;
            
        Ok(StateChangeResult {
            success: true,
            gas_used: 0, // Not provided by call trace
            revert_reason: None,
            state_changes,
        })
    }
    
    // ========== Sequential Buy/Sell Simulation Methods ==========
    
    /// Simulate buy → approve → sell sequence without an initial transaction
    pub async fn simulate_buy_sell_sequence(
        &self,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        self.sequential_simulator
            .simulate_sequence(token_address, pool_address, block_number)
            .await
    }
    
    /// Simulate a sequence with an initial transaction followed by buy → approve → sell
    pub async fn simulate_sequence_with_tx(
        &self,
        given_tx: Option<CallRequest>,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        self.sequential_simulator
            .simulate_sequence_with_tx(given_tx, token_address, pool_address, block_number)
            .await
    }
    
    /// Simulate a sequence with an initial transaction and specific pool type
    pub async fn simulate_sequence_with_tx_and_pool_type(
        &self,
        given_tx: Option<CallRequest>,
        token_address: Address,
        pool_address: Address,
        pool_type: &str,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        self.sequential_simulator
            .simulate_sequence_with_tx_and_pool_type(given_tx, token_address, pool_address, pool_type, block_number)
            .await
    }
    
    /// Get the buyer address used for simulations
    pub fn get_buyer_address(&self) -> Address {
        self.sequential_simulator.get_buyer_address()
    }
    
    /// Get access to the single transaction simulator
    pub fn single_tx_simulator(&self) -> &RethTxSimulator {
        &self.single_simulator
    }
}