use alloy_consensus::Transaction as _;
use alloy_primitives::{Address, B256};
use alloy_rpc_types_trace::geth::{
    CallConfig, GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
    GethTrace, PreStateConfig, TraceResult,
};
use eyre::Result;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::{Recovered, SealedHeader, SignerRecoverable};
use reth_provider::{BlockReader, StateProviderBox};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

use crate::block_trace::types::ReplayProfileConfig;
use crate::TxSimulator;

use super::engine::BlockTraceEngine;
use super::BlockTracer;

impl<'a> BlockTracer<'a> {
    pub(super) fn trace_block_sync_with_engine(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
        engine: BlockTraceEngine,
    ) -> Result<Vec<TraceResult>> {
        match engine {
            BlockTraceEngine::FreshInspector => {
                Self::trace_block_sync_fresh_inspector(simulator, block_hash, opts)
            }
            BlockTraceEngine::RethFusedCallTracer => {
                Self::trace_block_sync_reth_fused_call_tracer(simulator, block_hash, opts)
            }
            BlockTraceEngine::RethDebug => Self::trace_block_sync_profiled(
                simulator,
                block_hash,
                opts,
                engine,
                ReplayProfileConfig::default(),
            )
            .map(|profiled| profiled.traces),
        }
    }

    /// Synchronous block tracing implementation using the original fresh-inspector-per-tx path.
    fn trace_block_sync_fresh_inspector(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        let provider = simulator.provider_factory.provider()?;

        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;

        let transactions = block.body.transactions.clone();
        let parent_hash = block.header.parent_hash;
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());

        let state_at_parent = simulator
            .provider_factory
            .history_by_block_hash(parent_hash)?;
        let mut db = CacheDB::new(StateProviderDatabase::new(state_at_parent));
        let mut results = Vec::with_capacity(transactions.len());

        for (index, tx) in transactions.iter().enumerate() {
            let tx_hash = *tx.tx_hash();
            let sender = tx
                .recover_signer()
                .map_err(|e| eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e))?;

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

    /// Synchronous block tracing implementation modeled after Reth's debug tracer:
    /// keep one inspector alive, extract a result per transaction, then fuse it.
    fn trace_block_sync_reth_fused_call_tracer(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        let provider = simulator.provider_factory.provider()?;

        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;
        let transactions = block.body.transactions.clone();
        let parent_hash = block.header.parent_hash;
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());

        let state_at_parent = simulator
            .provider_factory
            .history_by_block_hash(parent_hash)?;
        let mut db = CacheDB::new(StateProviderDatabase::new(state_at_parent));
        let evm_env = simulator
            .evm_config
            .evm_env(&sealed_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let call_config = CallConfig::default();
        let mut inspector = create_inspector(&opts);
        let mut results = Vec::with_capacity(transactions.len());

        for (index, tx) in transactions.iter().enumerate() {
            let tx_hash = *tx.tx_hash();
            let sender = tx
                .recover_signer()
                .map_err(|e| eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e))?;
            let recovered = Recovered::new_unchecked(tx.clone(), sender);
            let tx_env = simulator.evm_config.tx_env(&recovered);

            let mut evm = simulator.evm_config.evm_with_env_and_inspector(
                &mut db,
                evm_env.clone(),
                &mut inspector,
            );
            let res = evm.transact(tx_env)?;

            inspector.set_transaction_gas_limit(tx.gas_limit());
            inspector.set_transaction_caller(sender);
            let call_frame = inspector
                .geth_builder()
                .geth_call_traces(call_config, res.result.tx_gas_used());

            results.push(TraceResult::Success {
                result: GethTrace::CallTracer(call_frame),
                tx_hash: Some(tx_hash),
            });

            db.commit(res.state);
            if index + 1 < transactions.len() {
                inspector.fuse();
            }
        }

        Ok(results)
    }

    /// Synchronously trace a single transaction by replaying the block up to it.
    pub(super) fn trace_transaction_in_block_sync(
        simulator: &TxSimulator,
        block_hash: B256,
        target_tx_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<TraceResult> {
        let provider = simulator.provider_factory.provider()?;

        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;
        let transactions = block.body.transactions.clone();

        let parent_hash = block.header.parent_hash;
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());

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

    /// Trace a single transaction within a block context.
    fn trace_single_transaction(
        simulator: &TxSimulator,
        tx: &TransactionSigned,
        sender: Address,
        db: &mut CacheDB<StateProviderDatabase<StateProviderBox>>,
        block_header: &SealedHeader,
        opts: &GethDebugTracingOptions,
        tx_hash: Option<B256>,
        tx_index: usize,
    ) -> Result<TraceResult> {
        let recovered = Recovered::new_unchecked(tx.clone(), sender);
        let evm_env = simulator
            .evm_config
            .evm_env(block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let tx_env = simulator.evm_config.tx_env(&recovered);
        let mut inspector = create_inspector(opts);

        let mut evm =
            simulator
                .evm_config
                .evm_with_env_and_inspector(&mut *db, evm_env, &mut inspector);
        let res = evm.transact(tx_env).map_err(|err| {
            eyre::eyre!(
                "failed to trace transaction tx_index={} tx_hash={:?}: {}",
                tx_index,
                tx_hash,
                err
            )
        })?;

        db.commit(res.state);

        inspector.set_transaction_gas_limit(tx.gas_limit());
        inspector.set_transaction_caller(sender);
        let call_frame = inspector
            .into_geth_builder()
            .geth_call_traces(CallConfig::default(), res.result.tx_gas_used());

        let trace = GethTrace::CallTracer(call_frame);
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
            other => TraceResult::Success {
                result: other,
                tx_hash,
            },
        };

        Ok(trace_result)
    }

    /// Execute a transaction solely to advance state, without trace collection.
    fn apply_transaction_without_trace(
        simulator: &TxSimulator,
        tx: &TransactionSigned,
        sender: Address,
        db: &mut CacheDB<StateProviderDatabase<StateProviderBox>>,
        block_header: &SealedHeader,
    ) -> Result<()> {
        let recovered = Recovered::new_unchecked(tx.clone(), sender);
        let evm_env = simulator
            .evm_config
            .evm_env(block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let tx_env = simulator.evm_config.tx_env(&recovered);

        let mut evm = simulator.evm_config.evm_with_env(&mut *db, evm_env);
        let res = evm.transact(tx_env)?;
        db.commit(res.state);

        Ok(())
    }
}

/// Create an inspector based on the tracing options.
pub(super) fn create_inspector(opts: &GethDebugTracingOptions) -> TracingInspector {
    match opts.tracer.as_ref() {
        Some(GethDebugTracerType::BuiltInTracer(tracer)) => match tracer {
            GethDebugBuiltInTracerType::CallTracer => {
                let config = TracingInspectorConfig::from_geth_call_config(&CallConfig::default());
                TracingInspector::new(config)
            }
            GethDebugBuiltInTracerType::PreStateTracer => {
                let config =
                    TracingInspectorConfig::from_geth_prestate_config(&PreStateConfig::default());
                TracingInspector::new(config)
            }
            _ => TracingInspector::new(TracingInspectorConfig::default_geth()),
        },
        _ => TracingInspector::new(TracingInspectorConfig::default_geth()),
    }
}

pub(super) fn call_tracer_options() -> GethDebugTracingOptions {
    GethDebugTracingOptions {
        tracer: Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer,
        )),
        ..Default::default()
    }
}
