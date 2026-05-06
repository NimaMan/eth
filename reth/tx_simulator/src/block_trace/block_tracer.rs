use alloy_consensus::Transaction as _;
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_trace::geth::{
    CallConfig, GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
    GethTrace, PreStateConfig, TraceResult,
};
/// Block tracer implementation - equivalent to debug_traceBlockByNumber
///
/// This module implements block-level tracing that matches Reth's debug_traceBlockByNumber
/// RPC method, but with direct database access for massive performance improvements.
use eyre::Result;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::{Recovered, SealedHeader, SignerRecoverable};
use reth_provider::{BlockHashReader, BlockReader, StateProviderBox, TransactionsProvider};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm::{bytecode::Bytecode, state::AccountInfo, DatabaseRef};
use revm_inspectors::tracing::{
    DebugInspector, TracingInspector, TracingInspectorConfig, TransactionContext,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::block_trace::types::{
    BlockReplayProfile, ProfiledBlockTrace, StateReadProfile, TransactionReplayProfile,
};
use crate::TxSimulator;

/// Local block trace implementation to use when replaying a block.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BlockTraceEngine {
    /// Historical implementation: build a fresh inspector for every transaction.
    #[default]
    FreshInspector,
    /// Reth-style implementation: reuse one call tracer inspector and fuse it between txs.
    RethFusedCallTracer,
    /// Reth debug RPC style: one DebugInspector, get_result per tx, fuse between txs.
    RethDebug,
}

impl BlockTraceEngine {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FreshInspector => "baseline-fresh",
            Self::RethFusedCallTracer => "tracing-fused",
            Self::RethDebug => "reth-debug",
        }
    }
}

type ProfiledCacheDb = CacheDB<InstrumentedStateProviderDatabase>;

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
        self.trace_block_by_number_with_engine(block_number, opts, BlockTraceEngine::default())
            .await
    }

    /// Trace all transactions in a block by number using an explicit local trace engine.
    pub async fn trace_block_by_number_with_engine(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
        engine: BlockTraceEngine,
    ) -> Result<Vec<TraceResult>> {
        let opts = opts.unwrap_or_default();

        // Get block hash
        let provider = self.simulator.provider_factory.provider()?;
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;

        self.trace_block_by_hash_with_engine(block_hash, opts, engine)
            .await
    }

    /// Trace all transactions in a block and return cold-replay profiling metrics.
    pub async fn trace_block_by_number_profiled(
        &self,
        block_number: u64,
        opts: Option<GethDebugTracingOptions>,
        engine: BlockTraceEngine,
    ) -> Result<ProfiledBlockTrace> {
        let opts = opts.unwrap_or_else(Self::call_tracer_options);
        let provider = self.simulator.provider_factory.provider()?;

        let hash_started = Instant::now();
        let block_hash = provider
            .block_hash(block_number)?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;
        let block_hash_lookup_ms = ms(hash_started.elapsed());

        let simulator = self.simulator.clone();
        tokio::task::spawn_blocking(move || {
            let mut profiled =
                Self::trace_block_sync_profiled(&simulator, block_hash, opts, engine)?;
            profiled.profile.block_hash_lookup_ms = block_hash_lookup_ms;
            Ok(profiled)
        })
        .await?
    }

    /// Trace all transactions in a block by hash
    pub async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> Result<Vec<TraceResult>> {
        self.trace_block_by_hash_with_engine(block_hash, opts, BlockTraceEngine::default())
            .await
    }

    /// Trace all transactions in a block by hash using an explicit local trace engine.
    pub async fn trace_block_by_hash_with_engine(
        &self,
        block_hash: B256,
        opts: GethDebugTracingOptions,
        engine: BlockTraceEngine,
    ) -> Result<Vec<TraceResult>> {
        // Clone necessary data for async block
        let simulator = self.simulator.clone();

        // Use spawn_blocking since this is CPU-intensive
        tokio::task::spawn_blocking(move || {
            Self::trace_block_sync_with_engine(&simulator, block_hash, opts, engine)
        })
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

    fn trace_block_sync_with_engine(
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
            BlockTraceEngine::RethDebug => {
                Self::trace_block_sync_profiled(simulator, block_hash, opts, engine)
                    .map(|profiled| profiled.traces)
            }
        }
    }

    /// Synchronous block tracing implementation using the original fresh-inspector-per-tx path.
    fn trace_block_sync_fresh_inspector(
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
        let mut inspector = Self::create_inspector(&opts);
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

    fn trace_block_sync_profiled(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
        engine: BlockTraceEngine,
    ) -> Result<ProfiledBlockTrace> {
        let total_started = Instant::now();
        let mut profile = BlockReplayProfile {
            engine: engine.as_str(),
            block_hash,
            ..Default::default()
        };

        let provider = simulator.provider_factory.provider()?;

        let block_started = Instant::now();
        let block = provider
            .block_by_hash(block_hash)?
            .ok_or_else(|| eyre::eyre!("Block {:?} not found", block_hash))?;
        profile.block_load_ms = ms(block_started.elapsed());
        profile.block_number = block.header.number;
        profile.gas_used = block.header.gas_used;

        let transactions = block.body.transactions.clone();
        profile.tx_count = transactions.len();
        let sealed_header = SealedHeader::new_unhashed(block.header.clone());
        let parent_hash = block.header.parent_hash;

        let state_started = Instant::now();
        let state_at_parent = simulator
            .provider_factory
            .history_by_block_hash(parent_hash)?;
        profile.state_open_ms = ms(state_started.elapsed());

        let state_reads = Arc::new(Mutex::new(StateReadProfile::default()));
        let state_db = InstrumentedStateProviderDatabase::new(
            StateProviderDatabase::new(state_at_parent),
            state_reads.clone(),
        );
        let mut db = CacheDB::new(state_db);

        let recovered = Self::recover_block_transactions(&transactions, &mut profile)?;

        let traces = match engine {
            BlockTraceEngine::FreshInspector => Self::trace_profiled_fresh_inspector(
                simulator,
                &recovered,
                &mut db,
                &sealed_header,
                &opts,
                &mut profile,
            )?,
            BlockTraceEngine::RethFusedCallTracer => Self::trace_profiled_tracing_fused(
                simulator,
                &recovered,
                &mut db,
                &sealed_header,
                &opts,
                &mut profile,
            )?,
            BlockTraceEngine::RethDebug => Self::trace_profiled_reth_debug(
                simulator,
                &recovered,
                &mut db,
                block_hash,
                &sealed_header,
                opts,
                &mut profile,
            )?,
        };

        profile.trace_node_count = trace_result_node_count(&traces);
        profile.errors = trace_result_error_count(&traces);
        profile.state_reads = state_reads
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        profile.total_ms = ms(total_started.elapsed());

        Ok(ProfiledBlockTrace { traces, profile })
    }

    fn recover_block_transactions(
        transactions: &[TransactionSigned],
        profile: &mut BlockReplayProfile,
    ) -> Result<Vec<Recovered<TransactionSigned>>> {
        let started = Instant::now();
        let recovered = transactions
            .iter()
            .map(|tx| {
                let tx_hash = *tx.tx_hash();
                let sender = tx.recover_signer().map_err(|e| {
                    eyre::eyre!("Failed to recover signer for tx {:?}: {}", tx_hash, e)
                })?;
                Ok(Recovered::new_unchecked(tx.clone(), sender))
            })
            .collect::<Result<Vec<_>>>()?;
        profile.sender_recovery_ms = ms(started.elapsed());
        Ok(recovered)
    }

    fn trace_profiled_fresh_inspector(
        simulator: &TxSimulator,
        transactions: &[Recovered<TransactionSigned>],
        db: &mut ProfiledCacheDb,
        sealed_header: &SealedHeader,
        opts: &GethDebugTracingOptions,
        profile: &mut BlockReplayProfile,
    ) -> Result<Vec<TraceResult>> {
        let mut results = Vec::with_capacity(transactions.len());

        for (index, tx) in transactions.iter().enumerate() {
            let mut tx_profile = TransactionReplayProfile {
                tx_index: index,
                tx_hash: *tx.tx_hash(),
                ..Default::default()
            };

            let env_started = Instant::now();
            let evm_env = simulator
                .evm_config
                .evm_env(sealed_header)
                .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
            tx_profile.evm_env_ms = ms(env_started.elapsed());
            profile.evm_env_ms += tx_profile.evm_env_ms;

            let tx_env_started = Instant::now();
            let tx_env = simulator.evm_config.tx_env(tx);
            tx_profile.tx_env_ms = ms(tx_env_started.elapsed());
            profile.tx_env_ms += tx_profile.tx_env_ms;

            let inspector_started = Instant::now();
            let mut inspector = Self::create_inspector(opts);
            tx_profile.inspector_build_ms = ms(inspector_started.elapsed());
            profile.inspector_build_ms += tx_profile.inspector_build_ms;

            let exec_started = Instant::now();
            let mut evm =
                simulator
                    .evm_config
                    .evm_with_env_and_inspector(&mut *db, evm_env, &mut inspector);
            let res = evm.transact(tx_env)?;
            tx_profile.evm_exec_ms = ms(exec_started.elapsed());
            profile.evm_exec_ms += tx_profile.evm_exec_ms;
            tx_profile.gas_used = res.result.tx_gas_used();

            let commit_started = Instant::now();
            db.commit(res.state);
            tx_profile.db_commit_ms = ms(commit_started.elapsed());
            profile.db_commit_ms += tx_profile.db_commit_ms;

            let trace_started = Instant::now();
            inspector.set_transaction_gas_limit(tx.gas_limit());
            inspector.set_transaction_caller(tx.signer());
            let call_frame = inspector
                .into_geth_builder()
                .geth_call_traces(CallConfig::default(), tx_profile.gas_used);
            tx_profile.trace_nodes = alloy_call_frame_nodes(&call_frame);
            tx_profile.trace_build_ms = ms(trace_started.elapsed());
            profile.trace_build_ms += tx_profile.trace_build_ms;

            results.push(TraceResult::Success {
                result: GethTrace::CallTracer(call_frame),
                tx_hash: Some(tx_profile.tx_hash),
            });
            profile.tx_profiles.push(tx_profile);
        }

        Ok(results)
    }

    fn trace_profiled_tracing_fused(
        simulator: &TxSimulator,
        transactions: &[Recovered<TransactionSigned>],
        db: &mut ProfiledCacheDb,
        sealed_header: &SealedHeader,
        opts: &GethDebugTracingOptions,
        profile: &mut BlockReplayProfile,
    ) -> Result<Vec<TraceResult>> {
        let env_started = Instant::now();
        let evm_env = simulator
            .evm_config
            .evm_env(sealed_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        profile.evm_env_ms = ms(env_started.elapsed());

        let inspector_started = Instant::now();
        let mut inspector = Self::create_inspector(opts);
        profile.inspector_build_ms = ms(inspector_started.elapsed());

        let mut results = Vec::with_capacity(transactions.len());
        let call_config = CallConfig::default();

        for (index, tx) in transactions.iter().enumerate() {
            let mut tx_profile = TransactionReplayProfile {
                tx_index: index,
                tx_hash: *tx.tx_hash(),
                evm_env_ms: if index == 0 { profile.evm_env_ms } else { 0.0 },
                inspector_build_ms: if index == 0 {
                    profile.inspector_build_ms
                } else {
                    0.0
                },
                ..Default::default()
            };

            let tx_env_started = Instant::now();
            let tx_env = simulator.evm_config.tx_env(tx);
            tx_profile.tx_env_ms = ms(tx_env_started.elapsed());
            profile.tx_env_ms += tx_profile.tx_env_ms;

            let exec_started = Instant::now();
            let mut evm = simulator.evm_config.evm_with_env_and_inspector(
                &mut *db,
                evm_env.clone(),
                &mut inspector,
            );
            let res = evm.transact(tx_env)?;
            tx_profile.evm_exec_ms = ms(exec_started.elapsed());
            profile.evm_exec_ms += tx_profile.evm_exec_ms;
            tx_profile.gas_used = res.result.tx_gas_used();

            let trace_started = Instant::now();
            inspector.set_transaction_gas_limit(tx.gas_limit());
            inspector.set_transaction_caller(tx.signer());
            let call_frame = inspector
                .geth_builder()
                .geth_call_traces(call_config, tx_profile.gas_used);
            tx_profile.trace_nodes = alloy_call_frame_nodes(&call_frame);
            tx_profile.trace_build_ms = ms(trace_started.elapsed());
            profile.trace_build_ms += tx_profile.trace_build_ms;

            results.push(TraceResult::Success {
                result: GethTrace::CallTracer(call_frame),
                tx_hash: Some(tx_profile.tx_hash),
            });

            let commit_started = Instant::now();
            db.commit(res.state);
            tx_profile.db_commit_ms = ms(commit_started.elapsed());
            profile.db_commit_ms += tx_profile.db_commit_ms;

            if index + 1 < transactions.len() {
                let fuse_started = Instant::now();
                inspector.fuse();
                let fuse_ms = ms(fuse_started.elapsed());
                tx_profile.inspector_build_ms += fuse_ms;
                profile.inspector_build_ms += fuse_ms;
            }
            profile.tx_profiles.push(tx_profile);
        }

        Ok(results)
    }

    fn trace_profiled_reth_debug(
        simulator: &TxSimulator,
        transactions: &[Recovered<TransactionSigned>],
        db: &mut ProfiledCacheDb,
        block_hash: B256,
        sealed_header: &SealedHeader,
        opts: GethDebugTracingOptions,
        profile: &mut BlockReplayProfile,
    ) -> Result<Vec<TraceResult>> {
        let env_started = Instant::now();
        let evm_env = simulator
            .evm_config
            .evm_env(sealed_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        profile.evm_env_ms = ms(env_started.elapsed());

        let inspector_started = Instant::now();
        let mut inspector =
            DebugInspector::new(opts).map_err(|err| eyre::eyre!("debug inspector: {}", err))?;
        profile.inspector_build_ms = ms(inspector_started.elapsed());

        let mut results = Vec::with_capacity(transactions.len());

        for (index, tx) in transactions.iter().enumerate() {
            let mut tx_profile = TransactionReplayProfile {
                tx_index: index,
                tx_hash: *tx.tx_hash(),
                evm_env_ms: if index == 0 { profile.evm_env_ms } else { 0.0 },
                inspector_build_ms: if index == 0 {
                    profile.inspector_build_ms
                } else {
                    0.0
                },
                ..Default::default()
            };

            let tx_env_started = Instant::now();
            let tx_env = simulator.evm_config.tx_env(tx);
            tx_profile.tx_env_ms = ms(tx_env_started.elapsed());
            profile.tx_env_ms += tx_profile.tx_env_ms;

            let exec_started = Instant::now();
            let mut evm = simulator.evm_config.evm_with_env_and_inspector(
                &mut *db,
                evm_env.clone(),
                &mut inspector,
            );
            let res = evm.transact(tx_env.clone())?;
            tx_profile.evm_exec_ms = ms(exec_started.elapsed());
            profile.evm_exec_ms += tx_profile.evm_exec_ms;
            tx_profile.gas_used = res.result.tx_gas_used();

            let trace_started = Instant::now();
            let result = inspector
                .get_result(
                    Some(TransactionContext {
                        block_hash: Some(block_hash),
                        tx_hash: Some(tx_profile.tx_hash),
                        tx_index: Some(index),
                    }),
                    &tx_env,
                    &evm_env.block_env,
                    &res,
                    db,
                )
                .map_err(|err| eyre::eyre!("debug trace result: {}", err))?;
            tx_profile.trace_nodes = geth_trace_node_count(&result);
            tx_profile.trace_build_ms = ms(trace_started.elapsed());
            profile.trace_build_ms += tx_profile.trace_build_ms;

            results.push(TraceResult::Success {
                result,
                tx_hash: Some(tx_profile.tx_hash),
            });

            let commit_started = Instant::now();
            db.commit(res.state);
            tx_profile.db_commit_ms = ms(commit_started.elapsed());
            profile.db_commit_ms += tx_profile.db_commit_ms;

            if index + 1 < transactions.len() {
                let fuse_started = Instant::now();
                inspector
                    .fuse()
                    .map_err(|err| eyre::eyre!("debug inspector fuse: {}", err))?;
                let fuse_ms = ms(fuse_started.elapsed());
                tx_profile.inspector_build_ms += fuse_ms;
                profile.inspector_build_ms += fuse_ms;
            }
            profile.tx_profiles.push(tx_profile);
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
        db: &mut CacheDB<StateProviderDatabase<StateProviderBox>>,
        block_header: &SealedHeader,
        opts: &GethDebugTracingOptions,
        tx_hash: Option<B256>,
        _tx_index: usize,
    ) -> Result<TraceResult> {
        // Create recovered transaction
        let recovered = Recovered::new_unchecked(tx.clone(), sender);

        // Setup EVM environment
        let evm_env = simulator
            .evm_config
            .evm_env(block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;

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
        inspector.set_transaction_gas_limit(tx.gas_limit());
        inspector.set_transaction_caller(sender);
        let call_frame = inspector
            .into_geth_builder()
            .geth_call_traces(CallConfig::default(), res.result.tx_gas_used());

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
            // Handle any new tracer types added in future alloy versions
            other => TraceResult::Success {
                result: other,
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

    fn call_tracer_options() -> GethDebugTracingOptions {
        GethDebugTracingOptions {
            tracer: Some(GethDebugTracerType::BuiltInTracer(
                GethDebugBuiltInTracerType::CallTracer,
            )),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
struct InstrumentedStateProviderDatabase {
    inner: StateProviderDatabase<StateProviderBox>,
    reads: Arc<Mutex<StateReadProfile>>,
}

impl InstrumentedStateProviderDatabase {
    fn new(
        inner: StateProviderDatabase<StateProviderBox>,
        reads: Arc<Mutex<StateReadProfile>>,
    ) -> Self {
        Self { inner, reads }
    }

    fn record(&self, kind: StateReadKind, elapsed: Duration) {
        if let Ok(mut reads) = self.reads.lock() {
            match kind {
                StateReadKind::Account => reads.account_reads += 1,
                StateReadKind::Storage => reads.storage_reads += 1,
                StateReadKind::Code => reads.code_reads += 1,
                StateReadKind::BlockHash => reads.block_hash_reads += 1,
            }
            reads.provider_read_ms += ms(elapsed);
        }
    }
}

enum StateReadKind {
    Account,
    Storage,
    Code,
    BlockHash,
}

impl DatabaseRef for InstrumentedStateProviderDatabase {
    type Error = <StateProviderDatabase<StateProviderBox> as DatabaseRef>::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let started = Instant::now();
        let result = self.inner.basic_ref(address);
        self.record(StateReadKind::Account, started.elapsed());
        result
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let started = Instant::now();
        let result = self.inner.code_by_hash_ref(code_hash);
        self.record(StateReadKind::Code, started.elapsed());
        result
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let started = Instant::now();
        let result = self.inner.storage_ref(address, index);
        self.record(StateReadKind::Storage, started.elapsed());
        result
    }

    fn storage_by_account_id_ref(
        &self,
        address: Address,
        account_id: usize,
        storage_key: U256,
    ) -> Result<U256, Self::Error> {
        let started = Instant::now();
        let result = self
            .inner
            .storage_by_account_id_ref(address, account_id, storage_key);
        self.record(StateReadKind::Storage, started.elapsed());
        result
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        let started = Instant::now();
        let result = self.inner.block_hash_ref(number);
        self.record(StateReadKind::BlockHash, started.elapsed());
        result
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn trace_result_node_count(traces: &[TraceResult]) -> usize {
    traces.iter().map(trace_nodes).sum()
}

fn trace_result_error_count(traces: &[TraceResult]) -> usize {
    traces
        .iter()
        .filter(|trace| matches!(trace, TraceResult::Error { .. }))
        .count()
}

fn trace_nodes(trace: &TraceResult) -> usize {
    match trace {
        TraceResult::Success { result, .. } => geth_trace_node_count(result),
        TraceResult::Error { .. } => 0,
    }
}

fn geth_trace_node_count(trace: &GethTrace) -> usize {
    match trace {
        GethTrace::CallTracer(frame) => alloy_call_frame_nodes(frame),
        GethTrace::FlatCallTracer(frames) => frames.len(),
        _ => 0,
    }
}

fn alloy_call_frame_nodes(frame: &alloy_rpc_types_trace::geth::CallFrame) -> usize {
    1 + frame
        .calls
        .iter()
        .map(alloy_call_frame_nodes)
        .sum::<usize>()
}
