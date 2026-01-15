use alloy_primitives::{Address, B256};
use alloy_rpc_types_trace::geth::{
    CallConfig, GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
    GethTrace, PreStateConfig, TraceResult,
};
/// Block tracer implementation - equivalent to debug_traceBlockByNumber
///
/// This module implements block-level tracing that matches Reth's debug_traceBlockByNumber
/// RPC method, but with direct database access for massive performance improvements.
use eyre::Result;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives::{SealedHeader, TransactionSigned};
use reth_primitives_traits::SignerRecoverable;
use reth_provider::{BlockHashReader, BlockReader, TransactionsProvider};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

use crate::TxSimulator;

/// Block tracer for simulating all transactions in a block
pub struct BlockTracer<'a> {
    simulator: &'a TxSimulator,
}

impl<'a> BlockTracer<'a> {
    /// Create a new block tracer
    pub fn new(simulator: &'a TxSimulator) -> Self {
        Self { simulator }
    }

    /// Trace all transactions in a block by number - equivalent to debug_traceBlockByNumber
    pub async fn trace_block_by_number(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        let opts = opts.unwrap_or_default();

        // Get block hash
        let provider = self.simulator.provider_factory.provider()?;
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;

        self.trace_block_by_hash(block_hash, opts).await
    }

    /// Trace all transactions in a block by hash
    pub async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        // Clone necessary data for async block
        let simulator = self.simulator.clone();

        // Use spawn_blocking since this is CPU-intensive
        tokio::task::spawn_blocking(move || Self::trace_block_sync(&simulator, block_hash, opts))
            .await?
    }

    /// Trace a single transaction within a block (replaying all prior transactions)
    pub async fn trace_transaction_in_block_by_hash(
        &self,
        block_hash: B256,
        target_tx_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<TraceResult> {
        let simulator = self.simulator.clone();
        tokio::task::spawn_blocking(move || {
            Self::trace_transaction_in_block_sync(&simulator, block_hash, target_tx_hash, opts)
        })
        .await?
    }

    /// Trace a transaction by hash (automatically resolving its block)
    pub async fn trace_transaction_by_hash(
        &self,
        tx_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<TraceResult> {
        let opts = opts.unwrap_or_default();
        let provider = self.simulator.provider_factory.provider()?;
        let (_, meta) = provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction {:?} not found", tx_hash))?;
        let block_hash = meta.block_hash;
        self.trace_transaction_in_block_by_hash(block_hash, tx_hash, opts)
            .await
    }

    /// Synchronous block tracing implementation
    fn trace_block_sync(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        let provider = simulator.provider_factory.provider()?;

        // Get the block with transactions
        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;

        // Get block transactions from the block we just fetched
        let transactions = block.body.transactions.clone();

        // Get block header for environment setup
        let parent_hash = block.header.parent_hash;
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());

        // Create state at parent block (we need the state before this block was executed)
        // We use history_by_block_hash to get the state at the parent block
        let state_at_parent = simulator
            .provider_factory
            .history_by_block_hash(parent_hash)?;
        let mut db = CacheDB::new(StateProviderDatabase::new(state_at_parent));

        // Process each transaction sequentially
        let mut results = Vec::with_capacity(transactions.len());

        for (index, tx) in transactions.iter().enumerate() {
            let tx_hash = *tx.tx_hash();

            // Recover sender
            let sender = tx
                .recover_signer()
                .map_err(|e| eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e))?;

            // Trace the transaction at the current block state
            let trace_result = Self::trace_single_transaction(
                simulator,
                tx,
                sender,
                &mut db,
                &sealed_header,
                &opts,
                Some(tx_hash),
                index,
            )?;

            results.push(trace_result);
        }

        Ok(results)
    }

    /// Synchronously trace a single transaction by replaying the block up to it.
    fn trace_transaction_in_block_sync(
        simulator: &TxSimulator,
        block_hash: B256,
        target_tx_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<TraceResult> {
        let provider = simulator.provider_factory.provider()?;

        // Fetch the block and clone transactions
        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;
        let transactions = block.body.transactions.clone();

        let parent_hash = block.header.parent_hash;
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());

        // Seed state at the parent block
        let state_at_parent = simulator
            .provider_factory
            .history_by_block_hash(parent_hash)?;
        let mut db = CacheDB::new(StateProviderDatabase::new(state_at_parent));

        for (index, tx) in transactions.iter().enumerate() {
            let tx_hash = *tx.tx_hash();
            let sender = tx
                .recover_signer()
                .map_err(|e| eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e))?;

            if tx_hash == target_tx_hash {
                return Self::trace_single_transaction(
                    simulator,
                    tx,
                    sender,
                    &mut db,
                    &sealed_header,
                    &opts,
                    Some(tx_hash),
                    index,
                );
            } else {
                Self::apply_transaction_without_trace(
                    simulator,
                    tx,
                    sender,
                    &mut db,
                    &sealed_header,
                )?;
            }
        }

        Err(eyre::eyre!(
            "Transaction {:?} not found in block {:?}",
            target_tx_hash,
            block_hash
        ))
    }

    /// Trace a single transaction within a block context
    fn trace_single_transaction(
        simulator: &TxSimulator,
        tx: &TransactionSigned,
        sender: Address,
        db: &mut CacheDB<StateProviderDatabase<Box<dyn reth_provider::StateProvider>>>,
        block_header: &SealedHeader,
        opts: &GethDebugTracingOptions,
        tx_hash: Option<B256>,
        _tx_index: usize,
    ) -> Result<TraceResult> {
        use reth_primitives::Recovered;

        // Create recovered transaction
        let recovered = Recovered::new_unchecked(tx.clone(), sender);

        // Setup EVM environment
        let evm_env = simulator
            .evm_config
            .evm_env(block_header)
            .expect("failed to build EVM env");

        // Create transaction environment from recovered transaction
        let tx_env = simulator.evm_config.tx_env(&recovered);

        // Create inspector based on options
        let mut inspector = Self::create_inspector(opts);

        // Create and execute EVM
        let mut evm =
            simulator
                .evm_config
                .evm_with_env_and_inspector(&mut *db, evm_env, &mut inspector);
        let res = evm.transact(tx_env)?;

        // Commit state changes to database for next transaction
        db.commit(res.state);

        // Build the trace from the inspector
        let call_frame = inspector
            .into_geth_builder()
            .geth_call_traces(CallConfig::default(), res.result.gas_used());

        // Wrap in GethTrace
        let trace = GethTrace::CallTracer(call_frame);

        // Create the trace result
        let trace_result = match trace {
            GethTrace::Default(frame) => TraceResult::Success {
                result: GethTrace::Default(frame),
                tx_hash,
            },
            GethTrace::CallTracer(frame) => TraceResult::Success {
                result: GethTrace::CallTracer(frame),
                tx_hash,
            },
            GethTrace::PreStateTracer(diff) => TraceResult::Success {
                result: GethTrace::PreStateTracer(diff),
                tx_hash,
            },
            GethTrace::FourByteTracer(fourbyte) => TraceResult::Success {
                result: GethTrace::FourByteTracer(fourbyte),
                tx_hash,
            },
            GethTrace::NoopTracer(noop) => TraceResult::Success {
                result: GethTrace::NoopTracer(noop),
                tx_hash,
            },
            GethTrace::MuxTracer(mux) => TraceResult::Success {
                result: GethTrace::MuxTracer(mux),
                tx_hash,
            },
            GethTrace::FlatCallTracer(flat) => TraceResult::Success {
                result: GethTrace::FlatCallTracer(flat),
                tx_hash,
            },
            GethTrace::JS(js) => TraceResult::Success {
                result: GethTrace::JS(js),
                tx_hash,
            },
        };

        Ok(trace_result)
    }

    /// Execute a transaction solely to advance state (no trace collection)
    fn apply_transaction_without_trace(
        simulator: &TxSimulator,
        tx: &TransactionSigned,
        sender: Address,
        db: &mut CacheDB<StateProviderDatabase<Box<dyn reth_provider::StateProvider>>>,
        block_header: &SealedHeader,
    ) -> Result<()> {
        use reth_primitives::Recovered;

        let recovered = Recovered::new_unchecked(tx.clone(), sender);
        let evm_env = simulator
            .evm_config
            .evm_env(block_header)
            .expect("failed to build EVM env");
        let tx_env = simulator.evm_config.tx_env(&recovered);

        let mut evm = simulator.evm_config.evm_with_env(&mut *db, evm_env);
        let res = evm.transact(tx_env)?;
        db.commit(res.state);

        Ok(())
    }

    /// Create an inspector based on the tracing options
    fn create_inspector(opts: &GethDebugTracingOptions) -> TracingInspector {
        match opts.tracer.as_ref() {
            Some(GethDebugTracerType::BuiltInTracer(tracer)) => {
                match tracer {
                    GethDebugBuiltInTracerType::CallTracer => {
                        let config =
                            TracingInspectorConfig::from_geth_call_config(&CallConfig::default());
                        TracingInspector::new(config)
                    }
                    GethDebugBuiltInTracerType::PreStateTracer => {
                        let config = TracingInspectorConfig::from_geth_prestate_config(
                            &PreStateConfig::default(),
                        );
                        TracingInspector::new(config)
                    }
                    _ => {
                        // Default to call tracer
                        TracingInspector::new(TracingInspectorConfig::default_geth())
                    }
                }
            }
            _ => {
                // Default to call tracer
                TracingInspector::new(TracingInspectorConfig::default_geth())
            }
        }
    }
}
