/// Processed Transaction Provider
/// 
/// OBJECTIVE: Main entry point for obtaining ProcessedTransaction with two key methods:
/// 1. process_transaction_from_call_data() - Direct CallData simulation
/// 2. process_transaction_by_hash() - Load from DB then simulate
/// 
/// This integrates:
/// - tx_simulator for simulation
/// - tx_processor for event decoding and balance calculation
/// - CallDataBuilder for building CallRequest from DB data
/// - Direct Reth database access via TransactionLoader

use crate::tx_processor::{LogDecoder, TransactionClassifier, TxProcessor};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tx_loader::TransactionLoader;
use crate::simulator::UnsignedTxBuilder;
use reth_chain_query::ChainQuery;
use tx_simulator::{TxSimulator, UnsignedTransaction};
use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, B256, U256};
use super::core::initialization::create_provider_factory;

/// Processed Transaction Provider with two entry points: from CallData or from TX Hash
pub struct ProcessedTxProvider {
    pub simulator: TxSimulator,
    pub decoder: LogDecoder,
    pub classifier: TransactionClassifier,
    pub transaction_loader: Option<TransactionLoader>,
    pub provider_factory: reth_provider::ProviderFactory<reth_node_types::NodeTypesWithDBAdapter<reth_node_ethereum::EthereumNode, std::sync::Arc<reth_db::DatabaseEnv>>>,
    pub chain_query: Arc<ChainQuery>,
    unsigned_tx_builder: Option<UnsignedTxBuilder>,
    tx_processor: TxProcessor,
}

impl ProcessedTxProvider {
    /// Initialize the ProcessedTxProvider
    pub fn new(reth_datadir: &str) -> Result<Self> {
        // Create shared provider factory
        let provider_factory = create_provider_factory(reth_datadir)?;
        
        // Initialize NEW tx_simulator WITH SHARED PROVIDER FACTORY
        let simulator = TxSimulator::with_provider_factory(provider_factory.clone())?;
        let decoder = LogDecoder::new();
        let classifier = TransactionClassifier::new();
        let transaction_loader = TransactionLoader::with_provider_factory(provider_factory.clone()).ok();
        
        // Create unsigned tx builder if we have a transaction loader
        let unsigned_tx_builder = if let Some(loader) = transaction_loader.clone() {
            Some(UnsignedTxBuilder::new(loader))
        } else {
            None
        };
        
        // Initialize tx processor
        let tx_processor = TxProcessor::new();
        
        // Share the simulator to avoid duplicate DB connections
        let chain_query = Arc::new(ChainQuery::from_simulator(Arc::new(simulator.clone()))?);
        
        Ok(Self { 
            simulator,
            decoder, 
            classifier, 
            transaction_loader,
            provider_factory,
            chain_query,
            unsigned_tx_builder,
            tx_processor,
        })
    }
    
    /// MAIN ENTRY POINT 1: Process transaction from UnsignedTransaction
    /// 
    /// Flow: UnsignedTransaction → Simulate → Decode logs → ProcessedTransaction
    /// Use this when you already have the UnsignedTransaction ready for simulation
    pub async fn process_transaction_from_unsigned_tx(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
    ) -> Result<ProcessedTransaction> {
        
        // Use latest block if not specified
        let block_number = match block_number {
            Some(block) => block,
            None => self.simulator.get_latest_block()?,
        };
        
        // Simulate the transaction with full trace to get logs and internal txs
        let simulation_result = self.simulator.simulate_unsigned_transaction_with_full_trace_at_block(
            unsigned_tx.clone(), 
            block_number
        ).await?;
        
        // Convert simulation result to ProcessedTransaction (like Python does)
        let processed_tx = self.build_processed_transaction_from_simulation(
            &unsigned_tx,
            &simulation_result,
            block_number,
            0, // tx_index (0 for simulated transactions)
        ).await?;
        
        Ok(processed_tx)
    }
    
    /// MAIN ENTRY POINT 2: Process transaction by hash  
    /// 
    /// Flow: TX Hash → Load from DB → Build UnsignedTransaction → Simulate → tx_processing → ProcessedTransaction
    /// Use this when you have a transaction hash and want to process the actual transaction
    pub async fn process_transaction_by_hash(&self, tx_hash: B256) -> Result<ProcessedTransaction> {
        
        // Ensure we have an unsigned tx builder
        let unsigned_tx_builder = self.unsigned_tx_builder.as_ref()
            .ok_or_else(|| eyre::eyre!("TransactionLoader not available for loading transaction by hash"))?;
        
        // Build UnsignedTransaction from transaction hash (loads from DB)
        let unsigned_tx = unsigned_tx_builder.build_unsigned_transaction_from_tx_hash(tx_hash).await?;
        
        // Load block number from transaction data
        let transaction_loader = self.transaction_loader.as_ref().unwrap();
        let (
            _tx_hash_loaded,
            block_number,
            _timestamp,
            tx_index,
            _from,
            _to,
            _value,
            _input,
            _gas_price,
            _gas_used,
            _status,
            _nonce,
            _logs,
            _gas_limit,
        ) = transaction_loader.load_transaction_data(tx_hash).await?;
        
        // Simulate at block_number - 1 (the block BEFORE the transaction was included)
        // This ensures we have the correct state before the transaction executed
        let simulation_block = block_number.saturating_sub(1);
        
        // Simulate the transaction with full trace
        let simulation_result = self.simulator.simulate_unsigned_transaction_with_full_trace_at_block(
            unsigned_tx.clone(),
            simulation_block
        ).await?;
        
        // Convert simulation result to ProcessedTransaction
        let processed_tx = self.build_processed_transaction_from_simulation(
            &unsigned_tx,
            &simulation_result,
            block_number,
            tx_index,
        ).await?;
        
        Ok(processed_tx)
    }
    
    /// Get the shared provider factory (for use by other components that need DB access)
    pub fn provider_factory(&self) -> Arc<reth_provider::ProviderFactory<reth_node_types::NodeTypesWithDBAdapter<reth_node_ethereum::EthereumNode, Arc<reth_db::DatabaseEnv>>>> {
        Arc::new(self.provider_factory.clone())
    }
    
    /// Get the latest block number using NEW simulator
    pub async fn get_latest_block(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }
    
    /// Get the base fee for the latest block using NEW simulator
    pub async fn get_latest_base_fee(&self) -> Result<u128> {
        let latest_block = self.simulator.get_latest_block()?;
        self.simulator.get_base_fee_at_block(latest_block)
    }
    
    /// Get the base fee for a specific block using NEW simulator
    pub async fn get_base_fee_at_block(&self, block_number: u64) -> Result<u128> {
        self.simulator.get_base_fee_at_block(block_number)
    }
    
    /// Build ProcessedTransaction from simulation result (like Python's process_transaction)
    /// 
    /// This is the core method that takes simulation results (logs, traces, etc.)
    /// and builds a complete ProcessedTransaction, just like Python does.
    async fn build_processed_transaction_from_simulation(
        &self,
        unsigned_tx: &UnsignedTransaction,
        simulation_result: &tx_simulator::FullSimulationResult,
        block_number: u64,
        tx_index: u64,
    ) -> Result<ProcessedTransaction> {
        
        // Generate synthetic transaction hash for simulation
        let tx_hash = B256::random();
        
        // Extract transaction parameters from UnsignedTransaction
        let from = unsigned_tx.from.unwrap_or(Address::ZERO);
        let to = unsigned_tx.to;
        let value = unsigned_tx.value.unwrap_or(U256::ZERO);
        let input = unsigned_tx.data.as_ref().map(|d| d.to_vec()).unwrap_or_default();
        let gas_price = U256::from(unsigned_tx.gas_price.unwrap_or(20_000_000_000));
        let gas_used = simulation_result.gas_used;
        let status = if simulation_result.success { "1".to_string() } else { "0".to_string() };
        let nonce = unsigned_tx.nonce.unwrap_or(0);
        let gas_limit = unsigned_tx.gas.unwrap_or(300_000);
        
        // Use current timestamp for simulation
        let block_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Use tx_processor to create complete ProcessedTransaction with balance changes
        let processed_tx = self.tx_processor.process_transaction_from_simulation_result(
            &unsigned_tx,
            &simulation_result,
            block_number,
            tx_index,
        ).await?;
        
        Ok(processed_tx)
    }
}