use super::core::initialization::create_provider_factory;
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
use crate::block_processor::{BlockBatchOptions, BlockProcessor, ProcessedBlock};
use crate::processed_tx_provider::core::provider_factory::TxProcessorProviderFactory;
use crate::tx_processor::data_models::{
    ContractCreationEvent, ProcessedAccessListItem, ProcessedTransaction,
};
use crate::tx_processor::tx_loader::TransactionLoader;
use crate::tx_processor::{
    AddressBalanceChangeCalculator, LogDecoder, TransactionClassifier, TransactionTraceProcessor,
    TxProcessor,
};
use alloy_primitives::{keccak256, Address, B256};
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, GethTrace, TraceResult};
use eyre::{Result, WrapErr};
use reth_chain_query::ChainQuery;
use reth_primitives_traits::SealedHeader;
use reth_provider::TransactionsProvider;
use rlp::RlpStream;
use std::sync::Arc;
use tx_simulator::block_simulation::BlockTracer;
use tx_simulator::{TxSimulator, UnsignedTransaction};

/// Processed Transaction Provider with two entry points: from CallData or from TX Hash
pub struct ProcessedTxProvider {
    pub simulator: TxSimulator,
    pub decoder: LogDecoder,
    pub classifier: TransactionClassifier,
    pub transaction_loader: Option<TransactionLoader>,
    pub provider_factory: TxProcessorProviderFactory,
    pub chain_query: Arc<ChainQuery>,
    tx_processor: TxProcessor,
    block_processor: BlockProcessor,
}

fn derive_create_address(from: Address, nonce: u64) -> Address {
    let mut stream = RlpStream::new_list(2);
    stream.append(&from.as_slice());
    stream.append(&nonce);
    let hash = keccak256(stream.out());
    Address::from_slice(&hash[12..])
}

impl ProcessedTxProvider {
    /// Initialize the ProcessedTxProvider
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let provider_factory = create_provider_factory(reth_datadir)?;
        let simulator = TxSimulator::with_provider_factory(provider_factory.clone())?;
        Self::build(simulator, provider_factory)
    }

    /// Load and decode a transaction from DB only (no simulation)
    ///
    /// Flow: Load raw tx/receipt/logs from Reth DB → decode events → build ProcessedTransaction
    /// Balance changes and internal traces are NOT computed in this path.
    pub async fn load_transaction_from_hash_db_only(
        &self,
        tx_hash: B256,
    ) -> Result<ProcessedTransaction> {
        // Ensure we have a transaction loader
        let transaction_loader = self
            .transaction_loader
            .as_ref()
            .ok_or_else(|| eyre::eyre!("TransactionLoader not available for DB-only loading"))?;

        // Load raw transaction data and logs from DB
        let (
            _loaded_hash,
            block_number,
            block_timestamp,
            tx_index,
            from,
            to,
            value,
            input,
            gas_price,
            gas_used,
            status,
            nonce,
            logs,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
            raw_tx_type,
        ) = transaction_loader.load_transaction_data(tx_hash).await?;

        // Build ProcessedTransaction from raw DB data without simulation
        let access_list_items: Vec<ProcessedAccessListItem> = access_list
            .into_iter()
            .map(|item| ProcessedAccessListItem {
                address: item.address,
                storage_keys: item.storage_keys,
            })
            .collect();
        let processed_tx = self
            .tx_processor
            .process_transaction_from_raw_data(
                tx_hash,
                block_number,
                block_timestamp,
                tx_index,
                from,
                to,
                value,
                input,
                gas_price,
                gas_used,
                status,
                nonce,
                raw_tx_type,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                logs,
                gas_limit,
                access_list_items,
                blob_versioned_hashes,
                max_fee_per_blob_gas,
                None,
                signed_authorizations,
                None, // No balance changes without simulation
            )
            .await?;

        Ok(processed_tx)
    }

    /// Create ProcessedTxProvider with an existing TxSimulator (shared across consumers)
    pub fn with_simulator(simulator: Arc<TxSimulator>) -> Result<Self> {
        let provider_factory = simulator.provider_factory().clone();
        Self::build(simulator.as_ref().clone(), provider_factory)
    }

    /// Create ProcessedTxProvider with an existing provider factory
    /// This is useful when sharing a database connection across multiple components
    pub fn with_provider_factory(provider_factory: TxProcessorProviderFactory) -> Result<Self> {
        let simulator = TxSimulator::with_provider_factory(provider_factory.clone())?;
        Self::build(simulator, provider_factory)
    }

    fn build(simulator: TxSimulator, provider_factory: TxProcessorProviderFactory) -> Result<Self> {
        let decoder = LogDecoder::new();
        let classifier = TransactionClassifier::new();
        let transaction_loader =
            TransactionLoader::with_provider_factory(provider_factory.clone()).ok();

        // Initialize tx processor
        let tx_processor = TxProcessor::new();

        let block_processor = BlockProcessor::new(Arc::new(
            reth_chain_query::RethQueryProvider::with_provider_factory(Arc::new(
                provider_factory.clone(),
            ))?,
        ));

        // Share the simulator to avoid duplicate DB connections
        let chain_query = Arc::new(ChainQuery::from_simulator(Arc::new(simulator.clone()))?);

        Ok(Self {
            simulator,
            decoder,
            classifier,
            transaction_loader,
            provider_factory,
            chain_query,
            tx_processor,
            block_processor,
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
        let simulation_result = self
            .simulator
            .simulate_unsigned_transaction_with_full_trace_at_block(
                unsigned_tx.clone(),
                block_number,
            )
            .await?;

        // Convert simulation result to ProcessedTransaction (like Python does)
        let processed_tx = self
            .build_processed_transaction_from_simulation(
                &unsigned_tx,
                &simulation_result,
                block_number,
                0, // tx_index (0 for simulated transactions)
            )
            .await?;

        Ok(processed_tx)
    }

    /// Process transaction from unsigned tx using a provided block header snapshot
    pub async fn process_transaction_from_unsigned_tx_with_header(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
    ) -> Result<ProcessedTransaction> {
        let block_number = block_header.number;

        let state = self
            .simulator
            .provider_factory()
            .history_by_block_number(block_number)?;

        let simulation_result = self
            .simulator
            .simulate_unsigned_transaction_with_full_trace_on_state(
                unsigned_tx.clone(),
                block_header,
                state,
            )
            .await?;

        let processed_tx = self
            .build_processed_transaction_from_simulation(
                &unsigned_tx,
                &simulation_result,
                block_number,
                0,
            )
            .await?;

        Ok(processed_tx)
    }

    /// MAIN ENTRY POINT 2: Process transaction by hash.
    ///
    /// Flow: TX Hash → Load from DB → Partially replay block up to target → tx_processing → ProcessedTransaction
    /// Use this when you have a transaction hash and want to process the actual transaction from chain data.
    pub async fn process_transaction_by_hash(&self, tx_hash: B256) -> Result<ProcessedTransaction> {
        let provider = self.provider_factory.provider()?;
        let (_, meta) = provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction {:?} not found", tx_hash))?;
        let block_number = meta.block_number;
        let block_hash = meta.block_hash;
        drop(provider);

        self.process_transaction_by_hash_with_partial_block_replay(tx_hash, block_hash)
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to replay transaction {:?} in block {} ({:?})",
                    tx_hash, block_number, block_hash
                )
            })
    }

    /// Replay the block up to (and including) the target transaction instead of
    /// simulating the entire block. This pulls raw metadata from the DB and then
    /// uses the block tracer to stop once the target tx index is reached.
    async fn process_transaction_by_hash_with_partial_block_replay(
        &self,
        tx_hash: B256,
        block_hash: B256,
    ) -> Result<ProcessedTransaction> {
        let transaction_loader = self.transaction_loader.as_ref().ok_or_else(|| {
            eyre::eyre!("TransactionLoader not available for targeted processing")
        })?;

        let (
            _loaded_hash,
            block_number,
            block_timestamp,
            tx_index,
            from,
            to,
            value,
            input,
            gas_price,
            gas_used,
            status,
            nonce,
            logs,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
            raw_tx_type,
        ) = transaction_loader.load_transaction_data(tx_hash).await?;

        let access_list_items: Vec<ProcessedAccessListItem> = access_list
            .into_iter()
            .map(|item| ProcessedAccessListItem {
                address: item.address,
                storage_keys: item.storage_keys,
            })
            .collect();

        let mut processed_tx = self
            .tx_processor
            .process_transaction_from_raw_data(
                tx_hash,
                block_number,
                block_timestamp,
                tx_index,
                from,
                to,
                value,
                input,
                gas_price,
                gas_used,
                status,
                nonce,
                raw_tx_type,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                logs,
                gas_limit,
                access_list_items,
                blob_versioned_hashes,
                max_fee_per_blob_gas,
                None,
                signed_authorizations,
                None,
            )
            .await?;

        let tracer = BlockTracer::new(&self.simulator);
        let trace_result = tracer
            .trace_transaction_in_block_by_hash(
                block_hash,
                tx_hash,
                GethDebugTracingOptions::default(),
            )
            .await?;

        let call_frame = match trace_result {
            TraceResult::Success {
                result: GethTrace::CallTracer(frame),
                ..
            } => frame,
            TraceResult::Success { result, .. } => {
                return Err(eyre::eyre!(
                    "Unsupported tracer result for {:?}: {:?}",
                    tx_hash,
                    result
                ));
            }
            TraceResult::Error { error, .. } => {
                return Err(eyre::eyre!(
                    "Tracing transaction {:?} failed: {}",
                    tx_hash,
                    error
                ));
            }
        };

        let trace_processor = TransactionTraceProcessor::new();
        processed_tx.internal_transactions =
            trace_processor.extract_internal_transactions_from_call_trace(&call_frame);
        processed_tx.bribe_amount =
            TxProcessor::calculate_bribe_amount(&processed_tx.internal_transactions);

        let mut balance_calculator = AddressBalanceChangeCalculator::new();
        processed_tx.address_balance_changes = balance_calculator
            .calculate_balance_changes_from_processed_data(
                &processed_tx.erc20_transfers,
                &processed_tx.internal_transactions,
                block_number,
                tx_index,
            )?;

        if processed_tx.to_address.is_none() {
            let contract_address = derive_create_address(from, nonce);
            processed_tx.contract_address = Some(contract_address);

            if status {
                processed_tx
                    .contract_creation_events
                    .push(ContractCreationEvent { contract_address });
            }
        }

        Ok(processed_tx)
    }

    /// Get the shared provider factory (for use by other components that need DB access)
    pub fn provider_factory(&self) -> Arc<TxProcessorProviderFactory> {
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
        self.tx_processor
            .process_transaction_from_simulation_result(
                unsigned_tx,
                simulation_result,
                block_number,
                tx_index,
            )
            .await
    }

    /// Process all transactions within a block and return the structured block result.
    pub async fn process_block(&self, block_number: u64) -> Result<ProcessedBlock> {
        self.block_processor.process_block(block_number).await
    }

    /// Process multiple blocks in parallel using the underlying block processor.
    pub async fn process_block_batch<I>(
        &self,
        block_numbers: I,
        options: Option<BlockBatchOptions>,
    ) -> Result<Vec<ProcessedBlock>>
    where
        I: IntoIterator<Item = u64>,
    {
        let opts = options.unwrap_or_default();
        self.block_processor
            .process_block_batch(block_numbers, opts)
            .await
    }
}
