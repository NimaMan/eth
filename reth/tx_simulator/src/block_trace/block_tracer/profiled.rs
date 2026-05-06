use alloy_consensus::Transaction as _;
use alloy_primitives::B256;
use alloy_rpc_types_trace::geth::{CallConfig, GethDebugTracingOptions, GethTrace, TraceResult};
use eyre::Result;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::{Recovered, SealedHeader, SignerRecoverable};
use reth_provider::BlockReader;
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm_inspectors::tracing::{DebugInspector, TransactionContext};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::block_trace::types::{
    BlockReplayProfile, ProfiledBlockTrace, ReplayProfileConfig, StateAccessKeys, StateReadProfile,
    TransactionReplayProfile,
};
use crate::TxSimulator;

use super::engine::BlockTraceEngine;
use super::execution::create_inspector;
use super::instrumented_db::{
    prewarm_profiled_cache_db, InstrumentedStateProviderDatabase, ProfiledCacheDb,
};
use super::metrics::{
    alloy_call_frame_nodes, geth_trace_node_count, ms, trace_result_error_count,
    trace_result_node_count,
};
use super::BlockTracer;

impl<'a> BlockTracer<'a> {
    pub(super) fn trace_block_sync_profiled(
        simulator: &TxSimulator,
        block_hash: B256,
        opts: GethDebugTracingOptions,
        engine: BlockTraceEngine,
        config: ReplayProfileConfig,
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
        let state_keys = Arc::new(Mutex::new(StateAccessKeys::default()));
        let state_db = InstrumentedStateProviderDatabase::new(
            StateProviderDatabase::new(state_at_parent),
            state_reads.clone(),
            config.record_keys.then_some(state_keys.clone()),
        );
        let mut db = CacheDB::new(state_db);

        if let Some(prewarm_keys) = config.prewarm_keys.as_ref() {
            let preload_started = Instant::now();
            prewarm_profiled_cache_db(&mut db, prewarm_keys)?;
            profile.preload_ms = ms(preload_started.elapsed());
        }

        let recovered = Self::recover_block_transactions(&transactions, &mut profile)?;

        let replay_started = Instant::now();
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
        profile.exec_after_prewarm_ms = ms(replay_started.elapsed());

        profile.trace_node_count = trace_result_node_count(&traces);
        profile.errors = trace_result_error_count(&traces);
        profile.state_reads = state_reads
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        profile.state_keys = state_keys
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        profile.total_ms = ms(total_started.elapsed());

        Ok(ProfiledBlockTrace { traces, profile })
    }

    pub(super) fn execute_block_sync_profiled(
        simulator: &TxSimulator,
        block_hash: B256,
        config: ReplayProfileConfig,
    ) -> Result<ProfiledBlockTrace> {
        let total_started = Instant::now();
        let mut profile = BlockReplayProfile {
            engine: "execute-only",
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
        let state_keys = Arc::new(Mutex::new(StateAccessKeys::default()));
        let state_db = InstrumentedStateProviderDatabase::new(
            StateProviderDatabase::new(state_at_parent),
            state_reads.clone(),
            config.record_keys.then_some(state_keys.clone()),
        );
        let mut db = CacheDB::new(state_db);

        if let Some(prewarm_keys) = config.prewarm_keys.as_ref() {
            let preload_started = Instant::now();
            prewarm_profiled_cache_db(&mut db, prewarm_keys)?;
            profile.preload_ms = ms(preload_started.elapsed());
        }

        let recovered = Self::recover_block_transactions(&transactions, &mut profile)?;

        let env_started = Instant::now();
        let evm_env = simulator
            .evm_config
            .evm_env(&sealed_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        profile.evm_env_ms = ms(env_started.elapsed());

        let replay_started = Instant::now();
        for (index, tx) in recovered.iter().enumerate() {
            let reads_before = db.db.snapshot_reads();
            let mut tx_profile = TransactionReplayProfile {
                tx_index: index,
                tx_hash: *tx.tx_hash(),
                evm_env_ms: if index == 0 { profile.evm_env_ms } else { 0.0 },
                ..Default::default()
            };

            let tx_env_started = Instant::now();
            let tx_env = simulator.evm_config.tx_env(tx);
            tx_profile.tx_env_ms = ms(tx_env_started.elapsed());
            profile.tx_env_ms += tx_profile.tx_env_ms;

            let exec_started = Instant::now();
            let mut evm = simulator.evm_config.evm_with_env(&mut db, evm_env.clone());
            let res = evm.transact(tx_env)?;
            tx_profile.evm_exec_ms = ms(exec_started.elapsed());
            profile.evm_exec_ms += tx_profile.evm_exec_ms;
            tx_profile.gas_used = res.result.tx_gas_used();

            let commit_started = Instant::now();
            db.commit(res.state);
            tx_profile.db_commit_ms = ms(commit_started.elapsed());
            profile.db_commit_ms += tx_profile.db_commit_ms;

            tx_profile.state_reads = db.db.snapshot_reads().delta_since(&reads_before);
            profile.tx_profiles.push(tx_profile);
        }
        profile.exec_after_prewarm_ms = ms(replay_started.elapsed());
        profile.state_reads = state_reads
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        profile.state_keys = state_keys
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        profile.total_ms = ms(total_started.elapsed());

        Ok(ProfiledBlockTrace {
            traces: Vec::new(),
            profile,
        })
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
            let reads_before = db.db.snapshot_reads();
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
            let mut inspector = create_inspector(opts);
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
            tx_profile.state_reads = db.db.snapshot_reads().delta_since(&reads_before);
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
        let mut inspector = create_inspector(opts);
        profile.inspector_build_ms = ms(inspector_started.elapsed());

        let mut results = Vec::with_capacity(transactions.len());
        let call_config = CallConfig::default();

        for (index, tx) in transactions.iter().enumerate() {
            let reads_before = db.db.snapshot_reads();
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
            tx_profile.state_reads = db.db.snapshot_reads().delta_since(&reads_before);
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
            let reads_before = db.db.snapshot_reads();
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
            tx_profile.state_reads = db.db.snapshot_reads().delta_since(&reads_before);
            profile.tx_profiles.push(tx_profile);
        }

        Ok(results)
    }
}
