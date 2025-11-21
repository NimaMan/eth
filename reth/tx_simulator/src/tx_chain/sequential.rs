/// Batch Sequence Transaction Simulation
///
/// This module simulates a complete sequence of transactions as a single batch,
/// where each transaction builds on the state changes from previous ones.
/// All transactions must be provided upfront and are executed in order.
///
/// Use this for:
/// - MEV tx sequence simulation (known transaction sequences)
/// - Protocol testing with predetermined steps
/// - Batch validation of transaction sequences
///
/// For interactive, step-by-step simulation where you need to inspect results
/// between transactions, use SimulationChain instead.
use crate::{
    block_context::{BlockContext, BlockStateProvider},
    simulation_revert_decoder::decode_revert_reason,
    simulator::TxSimulator,
    single_tx::unsigned::UnsignedTransaction,
    tx_fee_parameters::{GasInputs, TxFeeContext},
    types::{
        RevertContext, SequentialSimulationOptions, SequentialSimulationResult,
        SequentialTransactionResult,
    },
};
use alloy_consensus::transaction::Either;
use alloy_eips::eip2930::AccessList;
use alloy_eips::eip7702::{RecoveredAuthorization, SignedAuthorization};
use eyre::{eyre, Result};
use std::collections::HashMap;
use tokio::task;

// Reth imports
use alloy_primitives::Address;
use alloy_rpc_types_trace::geth::{CallConfig, GethDefaultTracingOptions};
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives::SealedHeader;
use reth_provider::StateProvider;
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::primitives::KECCAK_EMPTY;
use reth_revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

/// Forked state for sequential transaction simulation
pub(crate) struct ForkedState {
    pub db: CacheDB<StateProviderDatabase<Box<dyn StateProvider>>>,
    pub block_number: u64,
    pub block_header: SealedHeader,
    pub nonces: HashMap<Address, u64>,
}

impl TxSimulator {
    /// Simulate a sequence of unsigned transactions where each builds on the previous state.
    ///
    /// This variant runs the bundle in one shot and returns aggregate statistics. It reuses a
    /// fused inspector under the hood so the cost stays close to a single `debug_traceBlock`
    /// despite running multiple transactions.
    pub async fn simulate_unsigned_tx_sequence(
        &self,
        transactions: Vec<UnsignedTransaction>,
        options: SequentialSimulationOptions,
    ) -> Result<SequentialSimulationResult> {
        if transactions.is_empty() {
            return Ok(SequentialSimulationResult {
                total_transactions: 0,
                successful_transactions: 0,
                failed_transactions: 0,
                total_gas_used: 0,
                results: vec![],
                sequence_success: true,
            });
        }

        let simulator = self.clone();

        task::spawn_blocking(move || {
            // Determine the forked context (historical MDBX or live replay via cache).
            let latest = simulator.get_latest_block()?;
            let block_number = options.at_block.unwrap_or(latest);

            let mut forked_state = simulator.create_forked_state(block_number)?;

            let mut results = Vec::new();
            let mut cumulative_gas_used = 0u64;
            let mut successful_transactions = 0usize;
            let mut failed_transactions = 0usize;

            // Create fused inspector that persists across transactions
            let mut inspector: Option<TracingInspector> = None;

            for (_index, mut transaction) in transactions.into_iter().enumerate() {
                // Auto-detect nonce if not provided
                if transaction.nonce.is_none() {
                    if let Some(from) = transaction.from {
                        let nonce = simulator.get_nonce_from_state(&mut forked_state, from)?;
                        transaction.nonce = Some(nonce);

                        // Track nonce for auto-increment
                        if options.auto_increment_nonces {
                            *forked_state.nonces.entry(from).or_insert(nonce) = nonce + 1;
                        }
                    }
                }

                // Apply gas limit if specified
                if let Some(gas_limit) = options.gas_limit_per_tx {
                    transaction.gas = Some(gas_limit);
                }

                // Simulate the transaction on the forked state with fused inspector
                let result = simulator.simulate_on_fork_with_inspector(
                    &mut forked_state,
                    transaction,
                    &mut inspector,
                )?;

                cumulative_gas_used += result.gas_used;

                // Track success/failure
                if result.success {
                    successful_transactions += 1;
                } else {
                    failed_transactions += 1;

                    // Stop on failure if configured
                    if options.stop_on_failure {
                        results.push(result);
                        break;
                    }
                }

                results.push(result);
            }

            let sequence_success = failed_transactions == 0;

            Ok(SequentialSimulationResult {
                total_transactions: results.len(),
                successful_transactions,
                failed_transactions,
                total_gas_used: cumulative_gas_used,
                results,
                sequence_success,
            })
        })
        .await
        .map_err(|e| eyre!("Spawn blocking failed: {}", e))?
    }

    /// Simulate a sequence of transactions where each builds on previous state changes.
    /// Create a forked state at a specific block for sequential simulation
    pub(crate) fn create_forked_state(&self, block_number: u64) -> Result<ForkedState> {
        let context = self.load_block_context_blocking(block_number, None)?;
        Self::forked_state_from_context(block_number, context)
    }

    pub(crate) fn create_forked_state_with_header(
        &self,
        block_number: u64,
        block_header: SealedHeader,
    ) -> Result<ForkedState> {
        let context = self.load_block_context_blocking(block_number, Some(block_header))?;
        Self::forked_state_from_context(block_number, context)
    }

    fn forked_state_from_context(block_number: u64, context: BlockContext) -> Result<ForkedState> {
        match context.state {
            BlockStateProvider::Historical(state) => {
                let db = CacheDB::new(StateProviderDatabase::new(state));
                Ok(ForkedState {
                    db,
                    block_number,
                    block_header: context.header,
                    nonces: HashMap::new(),
                })
            }
            BlockStateProvider::LiveFork(fork) => Ok(fork),
        }
    }

    /// Simulate a transaction on a forked state with full trace
    pub(crate) fn simulate_on_fork_with_trace(
        &self,
        forked_state: &mut ForkedState,
        transaction: UnsignedTransaction,
        _block_number: u64,
    ) -> Result<crate::types::FullSimulationResult> {
        let block_header = forked_state.block_header.clone();

        // Create tracer with full config (call hierarchy + per-opcode struct logs)
        let unsigned_tx_config = TracingInspectorConfig::default_geth()
            .set_record_logs(true)
            .set_steps(true);
        let mut inspector = TracingInspector::new(unsigned_tx_config);

        // Setup EVM environment
        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .expect("failed to build EVM env");

        // Get base fee for gas price adjustment
        let base_fee = block_header.base_fee_per_gas.map(|v| v as u128);

        let initial_context = if let Some(target) = transaction.to {
            let has_code = fork_state_has_code(forked_state, target)?;
            Some(RevertContext {
                target,
                has_code,
                calldata_len: transaction.data.as_ref().map(|d| d.len()).unwrap_or(0),
            })
        } else {
            None
        };

        // Create transaction environment
        let tx_env = self.create_tx_env_from_unsigned_tx(
            &transaction,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut forked_state.db,
        )?;
        let gas_limit = tx_env.gas_limit;

        // Execute transaction
        let mut evm = self.evm_config.evm_with_env_and_inspector(
            &mut forked_state.db,
            evm_env,
            &mut inspector,
        );
        let res = evm.transact(tx_env)?;

        // Commit state changes to forked state
        forked_state.db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = decode_revert_reason(raw_output.as_ref(), initial_context.as_ref());
        let revert_context = if success { None } else { initial_context };

        // Extract unsigned_tx trace and step logs
        let builder = inspector
            .with_transaction_gas_limit(gas_limit)
            .into_geth_builder();
        let call_frame = builder.geth_call_traces(CallConfig::default().with_log(), gas_used);
        let struct_logs = Some(
            builder
                .geth_traces(
                    gas_used,
                    raw_output.clone().unwrap_or_default(),
                    GethDefaultTracingOptions::default(),
                )
                .struct_logs,
        );

        // Note: Nonce updating is handled by the unsigned_txer (SimulationChain)

        Ok(crate::types::FullSimulationResult {
            success,
            gas_used,
            revert_reason,
            revert_context,
            call_trace: call_frame,
            struct_logs,
            logs: emitted_logs,
        })
    }

    /// Simulate a transaction on a forked state with fused inspector
    pub(crate) fn simulate_on_fork_with_inspector(
        &self,
        forked_state: &mut ForkedState,
        transaction: UnsignedTransaction,
        inspector: &mut Option<TracingInspector>,
    ) -> Result<SequentialTransactionResult> {
        let block_header = forked_state.block_header.clone();

        // Get or create inspector with fusing
        let insp = inspector.get_or_insert_with(|| {
            let config = TracingInspectorConfig::default_parity().set_record_logs(true);
            TracingInspector::new(config)
        });

        // Setup EVM environment
        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .expect("failed to build EVM env");

        // Get base fee for gas price adjustment
        let base_fee = block_header.base_fee_per_gas.map(|v| v as u128);

        let initial_context = if let Some(target) = transaction.to {
            let has_code = fork_state_has_code(forked_state, target)?;
            Some(RevertContext {
                target,
                has_code,
                calldata_len: transaction.data.as_ref().map(|d| d.len()).unwrap_or(0),
            })
        } else {
            None
        };

        // Create transaction environment
        let tx_env = self.create_tx_env_from_unsigned_tx(
            &transaction,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut forked_state.db,
        )?;

        // Execute transaction with inspector
        let mut evm =
            self.evm_config
                .evm_with_env_and_inspector(&mut forked_state.db, evm_env, insp);
        let res = evm.transact(tx_env)?;

        // Commit state changes to forked state
        forked_state.db.commit(res.state);

        // Fuse inspector for next tx to clear tx-scoped buffers while reusing allocations
        if let Some(current) = inspector.take() {
            *inspector = Some(current.fused());
        }

        let success = res.result.is_success();
        let gas_used = res.result.gas_used();
        let revert_data = res.result.output().cloned();
        let revert_reason = decode_revert_reason(revert_data.as_ref(), initial_context.as_ref());
        let revert_context = if success { None } else { initial_context };

        // Update nonces
        let updated_nonces = forked_state.nonces.clone();

        Ok(SequentialTransactionResult {
            transaction_index: 0, // Will be set by caller
            success,
            gas_used,
            revert_reason,
            revert_context,
            cumulative_gas_used: gas_used,
            updated_nonces,
        })
    }

    /// Simulate a transaction on a forked state (legacy method without inspector fusing)
    #[allow(dead_code)]
    pub(crate) fn simulate_on_fork(
        &self,
        forked_state: &mut ForkedState,
        transaction: UnsignedTransaction,
    ) -> Result<SequentialTransactionResult> {
        let block_header = forked_state.block_header.clone();

        // Create tracer
        let unsigned_tx_config = TracingInspectorConfig::default_geth().set_record_logs(true);
        let mut inspector = TracingInspector::new(unsigned_tx_config);

        // Setup EVM environment
        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .expect("failed to build EVM env");

        // Get base fee for gas price adjustment
        let base_fee = block_header.base_fee_per_gas.map(|v| v as u128);

        let initial_context = if let Some(target) = transaction.to {
            let has_code = fork_state_has_code(forked_state, target)?;
            Some(RevertContext {
                target,
                has_code,
                calldata_len: transaction.data.as_ref().map(|d| d.len()).unwrap_or(0),
            })
        } else {
            None
        };

        // Create transaction environment
        let tx_env = self.create_tx_env_from_unsigned_tx(
            &transaction,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut forked_state.db,
        )?;
        let gas_limit = tx_env.gas_limit;

        // Execute transaction
        let mut evm = self.evm_config.evm_with_env_and_inspector(
            &mut forked_state.db,
            evm_env,
            &mut inspector,
        );
        let res = evm.transact(tx_env)?;

        // Commit state changes to forked state
        forked_state.db.commit(res.state);

        let success = res.result.is_success();
        let gas_used = res.result.gas_used();
        let revert_data = res.result.output().cloned();
        let revert_reason = decode_revert_reason(revert_data.as_ref(), initial_context.as_ref());
        let revert_context = if success { None } else { initial_context };

        // Extract unsigned_tx trace (not used in this method)
        let _call_frame = inspector
            .with_transaction_gas_limit(gas_limit)
            .into_geth_builder()
            .geth_call_traces(CallConfig::default().with_log(), gas_used);

        Ok(SequentialTransactionResult {
            transaction_index: 0, // Will be set by unsigned_txer
            success,
            gas_used,
            revert_reason,
            revert_context,
            cumulative_gas_used: 0, // Will be set by unsigned_txer
            updated_nonces: forked_state.nonces.clone(),
        })
    }

    /// Get nonce from forked state
    pub(crate) fn get_nonce_from_state(
        &self,
        forked_state: &mut ForkedState,
        address: Address,
    ) -> Result<u64> {
        // Check if we already tracked this nonce
        if let Some(&nonce) = forked_state.nonces.get(&address) {
            return Ok(nonce);
        }

        // Otherwise get from database
        let account_info = forked_state.db.basic(address)?;
        Ok(account_info.map(|info| info.nonce).unwrap_or(0))
    }

    /// Helper to create transaction environment from UnsignedTransaction
    /// This should ideally be in call_simulator but we'll add it here for now
    pub(crate) fn create_tx_env_from_unsigned_tx<DB: Database>(
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
        let access_list = AccessList::from(request.access_list.clone());
        let blob_hashes = request.blob_versioned_hashes.clone();
        let authorization_list: Vec<Either<SignedAuthorization, RecoveredAuthorization>> = request
            .signed_authorizations
            .iter()
            .cloned()
            .map(Either::Left)
            .collect();
        let max_fee_per_blob_gas = simulation_gas.max_fee_per_blob_gas.unwrap_or(0);

        // Create TxEnv - no signature needed!
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

fn fork_state_has_code(forked_state: &mut ForkedState, address: Address) -> eyre::Result<bool> {
    let info = forked_state.db.basic(address)?;
    Ok(info
        .map(|acc| {
            acc.code
                .as_ref()
                .map(|code| !code.is_empty())
                .unwrap_or_else(|| acc.code_hash != KECCAK_EMPTY)
        })
        .unwrap_or(false))
}
