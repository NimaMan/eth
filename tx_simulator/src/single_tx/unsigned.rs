/// Call simulation (unsigned transactions)
///
/// This module provides eth_call-equivalent execution along with tracing helpers that mirror
/// reth's `/debug/trace_call` and `/debug/trace_transaction` endpoints, all without going through
/// RPC. The "full trace" helpers enable step recording so `FullSimulationResult::struct_logs` is populated,
/// matching the high-fidelity output callers expect from `debug_traceTransaction`.
use crate::{
    block_context::{BlockContext, BlockStateProvider},
    simulation_revert_decoder::decode_revert_reason,
    simulator::TxSimulator,
    tx_chain::sequential::ForkedState,
    tx_fee_parameters::{GasInputs, TxFeeContext},
    types::{FullSimulationResult, RevertContext, SimulationResult, ViewFunctionResult},
};
use eyre::Result;
use tokio::task;

use alloy_consensus::transaction::Either;
use alloy_eips::{
    eip2930::{AccessList, AccessListItem},
    eip7702::{RecoveredAuthorization, SignedAuthorization},
};
// Reth imports
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_trace::geth::{CallConfig, CallFrame, GethDefaultTracingOptions, StructLog};
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::SealedHeader;
use reth_provider::StateProviderBox;
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use serde::{Deserialize, Serialize};

enum UnsignedTraceMode {
    None,
    Call { call_config: CallConfig },
    Full { call_config: CallConfig },
}

struct UnsignedExecutionResult {
    simulation: SimulationResult,
    call_trace: Option<CallFrame>,
    struct_logs: Option<Vec<StructLog>>,
    logs: Vec<alloy_primitives::Log>,
    output: Bytes,
}

impl UnsignedExecutionResult {
    fn into_simulation(self) -> SimulationResult {
        self.simulation
    }

    fn into_full(self) -> FullSimulationResult {
        let UnsignedExecutionResult {
            simulation,
            call_trace,
            struct_logs,
            logs,
            ..
        } = self;

        FullSimulationResult {
            success: simulation.success,
            gas_used: simulation.gas_used,
            revert_reason: simulation.revert_reason,
            revert_context: simulation.revert_context,
            call_trace: call_trace.unwrap_or_default(),
            struct_logs,
            logs,
        }
    }
}

/// Unsigned transaction for simulation (no signature required)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub value: Option<U256>,
    pub data: Option<Bytes>,
    pub nonce: Option<u64>,
    pub access_list: Vec<AccessListItem>,
    pub blob_versioned_hashes: Vec<B256>,
    pub max_fee_per_blob_gas: Option<u128>,
    pub signed_authorizations: Vec<SignedAuthorization>,
}

impl TxSimulator {
    /// Simulate an unsigned transaction against the latest block (eth_call equivalent).
    pub async fn simulate_unsigned_transaction(
        &self,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let block_number = self.latest_historical_context_block_number()?;
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block_number)
            .await
    }

    /// Simulate an unsigned transaction at a specific block (eth_call with block parameter).
    pub async fn simulate_unsigned_transaction_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        let context = self.prepare_block_context(block_number).await?;
        self.execute_with_block_context(unsigned_tx, context, UnsignedTraceMode::None)
            .await
            .map(UnsignedExecutionResult::into_simulation)
    }

    pub(crate) async fn simulate_unsigned_transaction_for_output_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<ViewFunctionResult> {
        let context = self.prepare_block_context(block_number).await?;
        let result = self
            .execute_with_block_context(unsigned_tx, context, UnsignedTraceMode::None)
            .await?;

        Ok(ViewFunctionResult {
            success: result.simulation.success,
            output: if result.simulation.success {
                result.output
            } else {
                Bytes::new()
            },
            gas_used: result.simulation.gas_used,
        })
    }

    /// Simulate an unsigned transaction using a pre-fetched header and state snapshot.
    pub async fn simulate_unsigned_transaction_on_state(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<SimulationResult> {
        self.execute_unsigned_transaction(unsigned_tx, block_header, state)
            .await
    }

    /// Simulate an unsigned transaction and capture a callTracer-style trace.
    ///
    /// Mirrors reth `/debug/trace_call` (callTracer). The returned `CallFrame` contains the full
    /// call hierarchy, emitted logs, and top-level return data while leaving
    /// [`FullSimulationResult::struct_logs`] empty for performance.
    /// empty for performance.
    pub async fn simulate_unsigned_transaction_with_trace(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
    ) -> Result<FullSimulationResult> {
        let block_number = block_number.unwrap_or(self.latest_historical_context_block_number()?);
        let context = self.prepare_block_context(block_number).await?;
        self.execute_with_block_context(
            unsigned_tx,
            context,
            UnsignedTraceMode::Call {
                call_config: CallConfig::default().with_log(),
            },
        )
        .await
        .map(UnsignedExecutionResult::into_full)
    }

    /// Simulate an unsigned transaction with a pre-fetched context and capture a callTracer-style trace.
    pub async fn simulate_unsigned_transaction_with_trace_on_state(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<FullSimulationResult> {
        self.execute_unsigned_transaction_with_trace(unsigned_tx, block_header, state)
            .await
    }

    /// Simulate an unsigned transaction with the highest-fidelity trace at a specific block.
    ///
    /// Mirrors reth `/debug/trace_transaction` (callTracer with step recording). Enables
    /// `TracingInspectorConfig::set_steps(true)` so the resulting [`FullSimulationResult`]
    /// carries populated `struct_logs` in addition to the call hierarchy and logs.
    pub async fn simulate_unsigned_transaction_with_full_trace_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<FullSimulationResult> {
        let context = self.prepare_block_context(block_number).await?;
        self.execute_with_block_context(
            unsigned_tx,
            context,
            UnsignedTraceMode::Full {
                call_config: CallConfig::default().with_log(),
            },
        )
        .await
        .map(UnsignedExecutionResult::into_full)
    }

    /// Simulate an unsigned transaction with a pre-fetched context and the highest-fidelity trace.
    ///
    /// Same fidelity as [`TxSimulator::simulate_unsigned_transaction_with_full_trace_at_block`]
    /// but executes against a caller-supplied header/state snapshot (reth
    /// `/debug/trace_transaction` with state overrides).
    pub async fn simulate_unsigned_transaction_with_full_trace_on_state(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<FullSimulationResult> {
        self.execute_unsigned_transaction_with_full_trace(unsigned_tx, block_header, state)
            .await
    }

    async fn execute_with_block_context(
        &self,
        unsigned_tx: UnsignedTransaction,
        context: BlockContext,
        trace_mode: UnsignedTraceMode,
    ) -> Result<UnsignedExecutionResult> {
        match context.state {
            BlockStateProvider::Historical(state) => {
                self.execute_on_state_provider(unsigned_tx, context.header, state, trace_mode)
                    .await
            }
            BlockStateProvider::LiveFork(fork_state) => {
                self.execute_on_live_fork(unsigned_tx, fork_state, trace_mode)
                    .await
            }
        }
    }

    async fn execute_on_state_provider(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
        trace_mode: UnsignedTraceMode,
    ) -> Result<UnsignedExecutionResult> {
        match trace_mode {
            UnsignedTraceMode::None => {
                self.execute_unsigned_transaction_without_trace(unsigned_tx, block_header, state)
                    .await
            }
            UnsignedTraceMode::Call { .. } => self
                .execute_unsigned_transaction_with_trace(unsigned_tx, block_header, state)
                .await
                .map(|full| Self::execution_from_full_result(full, false)),
            UnsignedTraceMode::Full { .. } => self
                .execute_unsigned_transaction_with_full_trace(unsigned_tx, block_header, state)
                .await
                .map(|full| Self::execution_from_full_result(full, true)),
        }
    }

    async fn execute_on_live_fork(
        &self,
        unsigned_tx: UnsignedTransaction,
        mut forked_state: ForkedState,
        trace_mode: UnsignedTraceMode,
    ) -> Result<UnsignedExecutionResult> {
        let simulator = self.clone();
        task::spawn_blocking(move || match trace_mode {
            UnsignedTraceMode::None => {
                simulator.simulate_on_fork_plain_execution(&mut forked_state, unsigned_tx)
            }
            UnsignedTraceMode::Call { .. } => {
                let block_number = forked_state.block_number;
                let mut full = simulator.simulate_on_fork_with_trace(
                    &mut forked_state,
                    unsigned_tx,
                    block_number,
                )?;
                full.struct_logs = None;
                Ok(Self::execution_from_full_result(full, false))
            }
            UnsignedTraceMode::Full { .. } => {
                let block_number = forked_state.block_number;
                let full = simulator.simulate_on_fork_with_trace(
                    &mut forked_state,
                    unsigned_tx,
                    block_number,
                )?;
                Ok(Self::execution_from_full_result(full, true))
            }
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    async fn prepare_block_context(&self, block_number: u64) -> Result<BlockContext> {
        self.block_context_loader()
            .load_block_context(block_number, None)
            .await
    }

    async fn execute_unsigned_transaction(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<SimulationResult> {
        self.execute_unsigned_transaction_without_trace(unsigned_tx, block_header, state)
            .await
            .map(UnsignedExecutionResult::into_simulation)
    }

    async fn execute_unsigned_transaction_without_trace(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<UnsignedExecutionResult> {
        let simulator = self.clone();
        task::spawn_blocking(move || {
            Self::run_unsigned_execution_without_trace(simulator, unsigned_tx, block_header, state)
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    async fn execute_unsigned_transaction_with_trace(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<FullSimulationResult> {
        let simulator = self.clone();
        task::spawn_blocking(move || {
            Self::run_unsigned_transaction_with_trace(
                simulator,
                unsigned_tx,
                block_header,
                state,
                TracingInspectorConfig::default_geth()
                    .set_steps(false)
                    .set_state_diffs(false)
                    .disable_stack_snapshots()
                    .set_record_logs(true),
                CallConfig::default().with_log(),
            )
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    async fn execute_unsigned_transaction_with_full_trace(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<FullSimulationResult> {
        let simulator = self.clone();
        task::spawn_blocking(move || {
            Self::run_unsigned_execution(
                simulator,
                unsigned_tx,
                block_header,
                state,
                TracingInspectorConfig::default_geth()
                    .set_record_logs(true)
                    .set_steps(true),
                UnsignedTraceMode::Full {
                    call_config: CallConfig::default().with_log(),
                },
            )
            .map(UnsignedExecutionResult::into_full)
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    fn run_unsigned_transaction_with_trace(
        simulator: TxSimulator,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
        inspector_config: TracingInspectorConfig,
        call_config: CallConfig,
    ) -> Result<FullSimulationResult> {
        Self::run_unsigned_execution(
            simulator,
            unsigned_tx,
            block_header,
            state,
            inspector_config,
            UnsignedTraceMode::Call { call_config },
        )
        .map(UnsignedExecutionResult::into_full)
    }

    fn run_unsigned_execution(
        simulator: TxSimulator,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
        inspector_config: TracingInspectorConfig,
        trace_mode: UnsignedTraceMode,
    ) -> Result<UnsignedExecutionResult> {
        let header = block_header.into_header();
        let mut db = CacheDB::new(StateProviderDatabase::new(state));
        let initial_context = Self::unsigned_initial_context(&mut db, &unsigned_tx)?;

        let mut inspector = TracingInspector::new(inspector_config);
        let evm_env = simulator
            .evm_config
            .evm_env(&header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);
        let tx_env = simulator.create_tx_env(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut db,
        )?;
        let gas_limit = tx_env.gas_limit;

        let mut evm =
            simulator
                .evm_config
                .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);
        let res = evm.transact(tx_env)?;
        db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = if success {
            None
        } else {
            decode_revert_reason(raw_output.as_ref(), initial_context.as_ref())
        };
        let revert_context = if success {
            None
        } else {
            initial_context.clone()
        };
        let simulation = SimulationResult {
            success,
            gas_used,
            revert_reason,
            revert_context,
        };

        let (call_trace, struct_logs) = match trace_mode {
            UnsignedTraceMode::None => (None, None),
            UnsignedTraceMode::Call { call_config } => {
                let builder = inspector
                    .with_transaction_gas_limit(gas_limit)
                    .into_geth_builder();
                let call_frame = builder.geth_call_traces(call_config, gas_used);
                (Some(call_frame), None)
            }
            UnsignedTraceMode::Full { call_config } => {
                let builder = inspector
                    .with_transaction_gas_limit(gas_limit)
                    .into_geth_builder();
                let call_frame = builder.geth_call_traces(call_config, gas_used);
                let return_value = raw_output.clone().unwrap_or_default();
                let struct_logs = builder
                    .geth_traces(gas_used, return_value, GethDefaultTracingOptions::default())
                    .struct_logs;
                (Some(call_frame), Some(struct_logs))
            }
        };

        Ok(UnsignedExecutionResult {
            simulation,
            call_trace,
            struct_logs,
            logs: emitted_logs,
            output: raw_output.unwrap_or_default(),
        })
    }

    fn run_unsigned_execution_without_trace(
        simulator: TxSimulator,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<UnsignedExecutionResult> {
        let header = block_header.into_header();
        let mut db = CacheDB::new(StateProviderDatabase::new(state));
        let initial_context = Self::unsigned_initial_context(&mut db, &unsigned_tx)?;

        let evm_env = simulator
            .evm_config
            .evm_env(&header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);
        let tx_env = simulator.create_tx_env(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut db,
        )?;

        let mut evm = simulator.evm_config.evm_with_env(&mut db, evm_env);
        let res = evm.transact(tx_env)?;
        db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = if success {
            None
        } else {
            decode_revert_reason(raw_output.as_ref(), initial_context.as_ref())
        };
        let revert_context = if success { None } else { initial_context };

        Ok(UnsignedExecutionResult {
            simulation: SimulationResult {
                success,
                gas_used,
                revert_reason,
                revert_context,
            },
            call_trace: None,
            struct_logs: None,
            logs: emitted_logs,
            output: raw_output.unwrap_or_default(),
        })
    }

    fn execution_from_full_result(
        mut full: FullSimulationResult,
        keep_struct_logs: bool,
    ) -> UnsignedExecutionResult {
        let struct_logs = if keep_struct_logs {
            full.struct_logs.take()
        } else {
            None
        };
        let simulation = SimulationResult {
            success: full.success,
            gas_used: full.gas_used,
            revert_reason: full.revert_reason,
            revert_context: full.revert_context,
        };

        UnsignedExecutionResult {
            simulation,
            call_trace: Some(full.call_trace),
            struct_logs,
            logs: full.logs,
            output: Bytes::new(),
        }
    }

    pub(crate) fn simulate_on_fork_without_trace(
        &self,
        forked_state: &mut ForkedState,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        self.simulate_on_fork_plain_execution(forked_state, unsigned_tx)
            .map(UnsignedExecutionResult::into_simulation)
    }

    pub(crate) fn simulate_view_on_fork_without_commit(
        &self,
        forked_state: &mut ForkedState,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<ViewFunctionResult> {
        let block_header = forked_state.block_header.clone();
        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let base_fee = block_header.header().base_fee_per_gas.map(|v| v as u128);

        let mut overlay_db = CacheDB::new(&mut forked_state.db);
        let tx_env = self.create_tx_env_from_unsigned_tx(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut overlay_db,
        )?;

        let mut evm = self.evm_config.evm_with_env(&mut overlay_db, evm_env);
        let res = evm.transact(tx_env)?;
        let success = res.result.is_success();
        let output = if success {
            res.result.output().cloned().unwrap_or_default()
        } else {
            Bytes::new()
        };

        Ok(ViewFunctionResult {
            success,
            output,
            gas_used: res.result.tx_gas_used(),
        })
    }

    fn simulate_on_fork_plain_execution(
        &self,
        forked_state: &mut ForkedState,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<UnsignedExecutionResult> {
        let block_header = forked_state.block_header.clone();
        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let base_fee = block_header.header().base_fee_per_gas.map(|v| v as u128);

        let initial_context = Self::unsigned_fork_context(forked_state, &unsigned_tx)?;
        let tx_env = self.create_tx_env_from_unsigned_tx(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut forked_state.db,
        )?;

        let mut evm = self.evm_config.evm_with_env(&mut forked_state.db, evm_env);
        let res = evm.transact(tx_env)?;
        forked_state.db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = if success {
            None
        } else {
            decode_revert_reason(raw_output.as_ref(), initial_context.as_ref())
        };
        let revert_context = if success { None } else { initial_context };

        Ok(UnsignedExecutionResult {
            simulation: SimulationResult {
                success,
                gas_used,
                revert_reason,
                revert_context,
            },
            call_trace: None,
            struct_logs: None,
            logs: emitted_logs,
            output: raw_output.unwrap_or_default(),
        })
    }

    fn unsigned_fork_context(
        forked_state: &mut ForkedState,
        tx: &UnsignedTransaction,
    ) -> Result<Option<RevertContext>> {
        if let Some(target) = tx.to {
            let info = forked_state.db.basic(target)?;
            let has_code = info
                .map(|acc| {
                    acc.code
                        .as_ref()
                        .map(|code| !code.is_empty())
                        .unwrap_or_else(|| acc.code_hash != reth_revm::primitives::KECCAK_EMPTY)
                })
                .unwrap_or(false);
            Ok(Some(RevertContext {
                target,
                has_code,
                calldata_len: tx.data.as_ref().map(|d| d.len()).unwrap_or(0),
            }))
        } else {
            Ok(None)
        }
    }

    fn unsigned_initial_context(
        db: &mut CacheDB<StateProviderDatabase<StateProviderBox>>,
        tx: &UnsignedTransaction,
    ) -> Result<Option<RevertContext>> {
        if let Some(target) = tx.to {
            let has_code = db.db.account_code(&target)?.is_some();
            Ok(Some(RevertContext {
                target,
                has_code,
                calldata_len: tx.data.as_ref().map(|d| d.len()).unwrap_or(0),
            }))
        } else {
            Ok(None)
        }
    }

    /// Helper to create transaction environment from UnsignedTransaction
    fn create_tx_env<DB: reth_revm::Database>(
        &self,
        request: &UnsignedTransaction,
        block_gas_limit: u128,
        base_fee: Option<u128>,
        db: &mut DB,
    ) -> Result<reth_revm::revm::context::TxEnv> {
        use alloy_primitives::TxKind;
        use reth_revm::revm::context::TxEnv;

        // Get caller address
        let caller = request.from.unwrap_or_default();

        // Get nonce from state if not provided
        let nonce = if let Some(nonce) = request.nonce {
            nonce
        } else {
            // Query the database for the account's nonce
            match db.basic(caller.into()) {
                Ok(Some(acc)) => acc.nonce,
                _ => 0,
            }
        };

        let fee_defaults = &self.defaults.fee;
        let simulation_gas = crate::tx_fee_parameters::prepare_tx_env_gas(
            None,
            &self.defaults.tx_gas,
            GasInputs {
                gas: request.gas,
                gas_price: request.gas_price,
                max_fee_per_gas: request.max_fee_per_gas,
                max_priority_fee_per_gas: request.max_priority_fee_per_gas,
                max_fee_per_blob_gas: request.max_fee_per_blob_gas,
                has_blob: !request.blob_versioned_hashes.is_empty(),
            },
            TxFeeContext {
                fee_defaults,
                block_gas_limit,
                base_fee,
            },
        )?;
        let gas_price = simulation_gas.gas_price;
        let gas_priority_fee = simulation_gas.max_priority_fee_per_gas;

        // Create TxEnv - no signature needed!
        let access_list = AccessList::from(request.access_list.clone());
        let blob_hashes = request.blob_versioned_hashes.clone();
        let authorization_list: Vec<Either<SignedAuthorization, RecoveredAuthorization>> = request
            .signed_authorizations
            .iter()
            .cloned()
            .map(Either::Left)
            .collect();
        let max_fee_per_blob_gas = simulation_gas.max_fee_per_blob_gas.unwrap_or(0);

        Ok(TxEnv {
            tx_type: simulation_gas.tx_type.as_reth_tx_type(),
            caller: caller.into(),
            gas_limit: simulation_gas.gas_limit,
            gas_price,
            gas_priority_fee,
            kind: if let Some(to) = request.to {
                TxKind::Call(to)
            } else {
                TxKind::Create
            },
            value: request.value.unwrap_or_default(),
            data: request.data.clone().unwrap_or_default(),
            nonce,
            chain_id: fee_defaults.chain_id,
            access_list,
            blob_hashes,
            max_fee_per_blob_gas,
            authorization_list,
        })
    }
}
