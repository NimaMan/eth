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
    block_context::BlockContext,
    revert::decode_revert_reason,
    simulator::EthereumProviderFactory,
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
use std::sync::{Arc, Mutex};
use tokio::task;

// Reth imports
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_trace::geth::{CallConfig, GethDefaultTracingOptions};
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::SealedHeader;
use reth_primitives_traits::{Account, Bytecode};
use reth_provider::StateProviderBox;
use reth_revm::database::{EvmStateProvider, StateProviderDatabase};
use reth_revm::db::CacheDB;
use reth_revm::primitives::KECCAK_EMPTY;
use reth_revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

#[derive(Clone)]
pub(crate) enum SharedStateProvider {
    Static(Arc<Mutex<StateProviderBox>>),
    RefreshingHistory {
        provider_factory: EthereumProviderFactory,
        block_number: u64,
    },
}

impl SharedStateProvider {
    pub(crate) fn new(provider: StateProviderBox) -> Self {
        Self::Static(Arc::new(Mutex::new(provider)))
    }

    pub(crate) fn refreshing_history(
        provider_factory: EthereumProviderFactory,
        block_number: u64,
    ) -> Self {
        Self::RefreshingHistory {
            provider_factory,
            block_number,
        }
    }
}

impl EvmStateProvider for SharedStateProvider {
    fn basic_account(&self, address: &Address) -> reth_provider::ProviderResult<Option<Account>> {
        match self {
            Self::Static(provider) => provider
                .lock()
                .expect("state provider lock poisoned")
                .basic_account(address),
            Self::RefreshingHistory {
                provider_factory,
                block_number,
            } => provider_factory
                .history_by_block_number(*block_number)?
                .basic_account(address),
        }
    }

    fn block_hash(&self, number: u64) -> reth_provider::ProviderResult<Option<B256>> {
        match self {
            Self::Static(provider) => provider
                .lock()
                .expect("state provider lock poisoned")
                .block_hash(number),
            Self::RefreshingHistory {
                provider_factory,
                block_number,
            } => provider_factory
                .history_by_block_number(*block_number)?
                .block_hash(number),
        }
    }

    fn bytecode_by_hash(
        &self,
        code_hash: &B256,
    ) -> reth_provider::ProviderResult<Option<Bytecode>> {
        match self {
            Self::Static(provider) => provider
                .lock()
                .expect("state provider lock poisoned")
                .bytecode_by_hash(code_hash),
            Self::RefreshingHistory {
                provider_factory,
                block_number,
            } => provider_factory
                .history_by_block_number(*block_number)?
                .bytecode_by_hash(code_hash),
        }
    }

    fn storage(
        &self,
        account: Address,
        storage_key: B256,
    ) -> reth_provider::ProviderResult<Option<U256>> {
        match self {
            Self::Static(provider) => provider
                .lock()
                .expect("state provider lock poisoned")
                .storage(account, storage_key),
            Self::RefreshingHistory {
                provider_factory,
                block_number,
            } => provider_factory
                .history_by_block_number(*block_number)?
                .storage(account, storage_key),
        }
    }
}

pub(crate) type SharedStateProviderDatabase = StateProviderDatabase<SharedStateProvider>;

/// Forked state for sequential transaction simulation
#[derive(Clone)]
pub(crate) struct ForkedState {
    pub db: CacheDB<SharedStateProviderDatabase>,
    pub block_number: u64,
    pub block_header: SealedHeader,
    pub nonces: HashMap<Address, u64>,
}

impl TxSimulator {
    /// Simulate a sequence of unsigned transactions where each builds on the previous state.
    ///
    /// This variant runs the bundle in one shot and returns aggregate statistics. It uses the
    /// plain no-trace EVM path so bundle simulation does not pay tracing overhead unless callers
    /// explicitly request traces through the chain APIs.
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
            // Determine the forked context (local historical context or live replay via cache).
            let latest = simulator.latest_historical_context_block_number()?;
            let block_number = options.at_block.unwrap_or(latest);

            let mut forked_state = simulator.create_forked_state(block_number)?;
            let total_transactions = transactions.len();

            let mut results = Vec::new();
            let mut cumulative_gas_used = 0u64;
            let mut successful_transactions = 0usize;
            let mut failed_transactions = 0usize;

            for (index, mut transaction) in transactions.into_iter().enumerate() {
                // Auto-detect nonce if not provided
                if transaction.nonce.is_none() {
                    if let Some(from) = transaction.from {
                        let nonce = simulator.get_nonce_from_state(&mut forked_state, from)?;
                        transaction.nonce = Some(nonce);
                    }
                }

                // Apply gas limit if specified
                if let Some(gas_limit) = options.gas_limit_per_tx {
                    transaction.gas = Some(gas_limit);
                }

                // Simulate the transaction on the forked state. The no-trace path avoids
                // inspector allocation entirely and commits executed failures just like the EVM.
                let mut result = match simulator
                    .simulate_on_fork_without_trace(&mut forked_state, transaction.clone())
                {
                    Ok(result) => {
                        if options.auto_increment_nonces {
                            advance_tracked_nonce(&mut forked_state, &transaction);
                        }
                        SequentialTransactionResult {
                            transaction_index: index,
                            success: result.success,
                            gas_used: result.gas_used,
                            revert_reason: result.revert_reason,
                            revert_context: result.revert_context,
                            cumulative_gas_used,
                            updated_nonces: forked_state.nonces.clone(),
                        }
                    }
                    Err(err) => SequentialTransactionResult {
                        transaction_index: index,
                        success: false,
                        gas_used: 0,
                        revert_reason: Some(err.to_string()),
                        revert_context: None,
                        cumulative_gas_used,
                        updated_nonces: forked_state.nonces.clone(),
                    },
                };
                result.transaction_index = index;

                cumulative_gas_used += result.gas_used;
                result.cumulative_gas_used = cumulative_gas_used;
                result.updated_nonces = forked_state.nonces.clone();

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
                total_transactions,
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
        let db = CacheDB::new(StateProviderDatabase::new(SharedStateProvider::new(
            context.state,
        )));
        Ok(ForkedState {
            db,
            block_number,
            block_header: context.header,
            nonces: HashMap::new(),
        })
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
            .map_err(|err| eyre!("failed to build EVM env: {}", err))?;

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
        let (tx_env, effective_gas_price, tx_type) = self.create_tx_env_from_unsigned_tx(
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
        let gas_used = res.result.tx_gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = if success {
            None
        } else {
            decode_revert_reason(raw_output.as_ref(), initial_context.as_ref())
        };
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
            effective_gas_price: Some(effective_gas_price),
            tx_type: Some(tx_type),
            revert_reason,
            revert_context,
            call_trace: call_frame,
            struct_logs,
            logs: emitted_logs,
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
    ) -> Result<(reth_revm::revm::context::TxEnv, u128, u8)> {
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
        let tx_type = simulation_gas.tx_type.as_reth_tx_type();
        let tx_env = TxEnv {
            tx_type,
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
        };

        Ok((tx_env, simulation_gas.effective_gas_price, tx_type))
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

fn advance_tracked_nonce(forked_state: &mut ForkedState, tx: &UnsignedTransaction) {
    let Some(from) = tx.from else {
        return;
    };

    if let Some(nonce) = tx.nonce {
        forked_state.nonces.insert(from, nonce.saturating_add(1));
    } else {
        let current = forked_state.nonces.entry(from).or_insert(0);
        *current = current.saturating_add(1);
    }
}
