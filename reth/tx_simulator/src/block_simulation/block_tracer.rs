/// Block tracer implementation - equivalent to debug_traceBlockByNumber
/// 
/// This module implements block-level tracing that matches Reth's debug_traceBlockByNumber
/// RPC method, but with direct database access for massive performance improvements.

use eyre::Result;
use alloy_primitives::{B256, Address};
use alloy_rpc_types_trace::geth::{
    GethDebugTracingOptions, TraceResult, GethTrace, CallConfig, PreStateConfig,
    GethDebugBuiltInTracerType, GethDebugTracerType,
};
use reth_provider::{HeaderProvider, BlockHashReader, BlockReader};
use reth_primitives_traits::SignerRecoverable;
use reth_evm::{ConfigureEvm, Evm};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_primitives::TransactionSigned;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use revm::DatabaseCommit;

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
        tokio::task::spawn_blocking(move || {
            Self::trace_block_sync(&simulator, block_hash, opts)
        })
        .await?
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
        let header = &block.header;
        let parent_hash = header.parent_hash;
        let block_number = header.number;
        
        // Create state at parent block (we need the state before this block was executed)
        // We use history_by_block_hash to get the state at the parent block
        let state_at_parent = simulator.provider_factory.history_by_block_hash(parent_hash)?;
        let mut db = CacheDB::new(StateProviderDatabase::new(state_at_parent));
        
        // Process each transaction sequentially
        let mut results = Vec::with_capacity(transactions.len());
        
        for (index, tx) in transactions.iter().enumerate() {
            let tx_hash = *tx.tx_hash();
            
            // Recover sender
            let sender = tx.recover_signer()
                .map_err(|e| eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e))?;
            
            // Trace the transaction at the current block state
            let trace_result = Self::trace_single_transaction(
                simulator,
                tx,
                sender,
                &mut db,
                block_number,
                &opts,
                Some(tx_hash),
                index,
            )?;
            
            results.push(trace_result);
        }
        
        Ok(results)
    }

    /// Trace a single transaction within a block context
    fn trace_single_transaction(
        simulator: &TxSimulator,
        tx: &TransactionSigned,
        sender: Address,
        db: &mut CacheDB<StateProviderDatabase<Box<dyn reth_provider::StateProvider>>>,
        block_number: u64,
        opts: &GethDebugTracingOptions,
        tx_hash: Option<B256>,
        _tx_index: usize,
    ) -> Result<TraceResult> {
        use reth_primitives::Recovered;
        
        // Create recovered transaction
        let recovered = Recovered::new_unchecked(tx.clone(), sender);
        
        // Get block environment
        let provider = simulator.provider_factory.provider()?;
        let header = provider.header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("Header not found for block {}", block_number))?;
        
        // Setup EVM environment
        let evm_env = simulator.evm_config.evm_env(&header);
        
        // Create transaction environment from recovered transaction
        let tx_env = simulator.evm_config.tx_env(&recovered);
        
        // Create inspector based on options
        let mut inspector = Self::create_inspector(opts);
        
        // Create and execute EVM
        let mut evm = simulator.evm_config.evm_with_env_and_inspector(&mut *db, evm_env, &mut inspector);
        let res = evm.transact(tx_env)?;
        
        // Commit state changes to database for next transaction
        db.commit(res.state);
        
        // Build the trace from the inspector
        let call_frame = inspector.into_geth_builder().geth_call_traces(
            CallConfig::default(),
            res.result.gas_used(),
        );
        
        // Wrap in GethTrace
        let trace = GethTrace::CallTracer(call_frame);
        
        // Create the trace result
        let trace_result = match trace {
            GethTrace::Default(frame) => {
                TraceResult::Success {
                    result: GethTrace::Default(frame),
                    tx_hash,
                }
            }
            GethTrace::CallTracer(frame) => {
                TraceResult::Success {
                    result: GethTrace::CallTracer(frame),
                    tx_hash,
                }
            }
            GethTrace::PreStateTracer(diff) => {
                TraceResult::Success {
                    result: GethTrace::PreStateTracer(diff),
                    tx_hash,
                }
            }
            GethTrace::FourByteTracer(fourbyte) => {
                TraceResult::Success {
                    result: GethTrace::FourByteTracer(fourbyte),
                    tx_hash,
                }
            }
            GethTrace::NoopTracer(noop) => {
                TraceResult::Success {
                    result: GethTrace::NoopTracer(noop),
                    tx_hash,
                }
            }
            GethTrace::MuxTracer(mux) => {
                TraceResult::Success {
                    result: GethTrace::MuxTracer(mux),
                    tx_hash,
                }
            }
            GethTrace::FlatCallTracer(flat) => {
                TraceResult::Success {
                    result: GethTrace::FlatCallTracer(flat),
                    tx_hash,
                }
            }
            GethTrace::JS(js) => {
                TraceResult::Success {
                    result: GethTrace::JS(js),
                    tx_hash,
                }
            }
        };
        
        Ok(trace_result)
    }

    /// Create an inspector based on the tracing options
    fn create_inspector(opts: &GethDebugTracingOptions) -> TracingInspector {
        match opts.tracer.as_ref() {
            Some(GethDebugTracerType::BuiltInTracer(tracer)) => {
                match tracer {
                    GethDebugBuiltInTracerType::CallTracer => {
                        let config = TracingInspectorConfig::from_geth_call_config(&CallConfig::default());
                        TracingInspector::new(config)
                    }
                    GethDebugBuiltInTracerType::PreStateTracer => {
                        let config = TracingInspectorConfig::from_geth_prestate_config(&PreStateConfig::default());
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