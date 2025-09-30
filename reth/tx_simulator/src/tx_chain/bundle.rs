/// Batch Sequence Transaction Simulation
///
/// This module simulates a complete sequence of transactions as a single batch,
/// where each transaction builds on the state changes from previous ones.
/// All transactions must be provided upfront and are executed in order.
///
/// Use this for:
/// - MEV bundle simulation (known transaction sequences)
/// - Protocol testing with predetermined steps
/// - Batch validation of transaction sequences
///
/// For interactive, step-by-step simulation where you need to inspect results
/// between transactions, use SimulationChain instead.
use crate::{
    simulation_revert_decoder::decode_revert_data,
    simulator::TxSimulator,
    single_tx::unsigned::UnsignedTransaction,
    types::{SequentialSimulationOptions, SequentialSimulationResult, SequentialTransactionResult},
};
use eyre::Result;
use std::collections::HashMap;
use tokio::task;

// Reth imports
use alloy_primitives::Address;
use alloy_rpc_types_trace::geth::CallConfig;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives::SealedHeader;
use reth_provider::{HeaderProvider, StateProvider};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
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
    /// Simulate a sequence of transactions where each builds on previous state changes
    ///
    /// This is crucial for:
    /// - MEV bundle simulation
    /// - Protocol testing (e.g., enable trading -> swap)
    /// - Complex DeFi interactions
    /// - Transaction dependency analysis
    ///
    /// Uses inspector fusing for optimal performance across the bundle.
    pub async fn simulate_transaction_sequence(
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
        let block_number = options.at_block.unwrap_or(self.get_latest_block()?);

        task::spawn_blocking(move || {
            // Create forked state
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
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    /// Create a forked state at a specific block for sequential simulation
    pub(crate) fn create_forked_state(&self, block_number: u64) -> Result<ForkedState> {
        let provider = self.provider_factory.provider()?;
        // Important: fetching the canonical header can fail briefly if the MDBX mapping
        // has not advanced yet even though the block is visible via RPC.
        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;
        let sealed_header = SealedHeader::new(header.clone(), header.hash_slow());

        self.create_forked_state_with_header(block_number, sealed_header)
    }

    pub(crate) fn create_forked_state_with_header(
        &self,
        block_number: u64,
        block_header: SealedHeader,
    ) -> Result<ForkedState> {
        let state = self
            .provider_factory
            .history_by_block_number(block_number)?;
        let db = CacheDB::new(StateProviderDatabase::new(state));

        Ok(ForkedState {
            db,
            block_number,
            block_header,
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
        let header = forked_state.block_header.clone();

        // Create tracer with full config
        let unsigned_tx_config = TracingInspectorConfig::default_geth().set_record_logs(true);
        let mut inspector = TracingInspector::new(unsigned_tx_config);

        // Setup EVM environment
        let evm_env = self.evm_config.evm_env(&header);

        // Get base fee for gas price adjustment
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);

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
        let revert_reason = if !success {
            res.result
                .output()
                .map(|bytes| decode_revert_data(&bytes))
                .or_else(|| Some("Transaction reverted without data".to_string()))
        } else {
            None
        };

        // Extract unsigned_tx trace
        let call_frame = inspector
            .with_transaction_gas_limit(gas_limit)
            .into_geth_builder()
            .geth_call_traces(CallConfig::default().with_log(), gas_used);

        // Note: Nonce updating is handled by the unsigned_txer (SimulationChain)

        Ok(crate::types::FullSimulationResult {
            success,
            gas_used,
            revert_reason,
            call_trace: call_frame,
        })
    }

    /// Simulate a transaction on a forked state with fused inspector
    pub(crate) fn simulate_on_fork_with_inspector(
        &self,
        forked_state: &mut ForkedState,
        transaction: UnsignedTransaction,
        inspector: &mut Option<TracingInspector>,
    ) -> Result<SequentialTransactionResult> {
        let header = forked_state.block_header.clone();

        // Get or create inspector with fusing
        let insp = inspector.get_or_insert_with(|| {
            let config = TracingInspectorConfig::default_geth().set_record_logs(true);
            TracingInspector::new(config)
        });

        // Setup EVM environment
        let evm_env = self.evm_config.evm_env(&header);

        // Get base fee for gas price adjustment
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);

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
        let revert_reason = if !success {
            res.result
                .output()
                .map(|bytes| decode_revert_data(&bytes))
                .or_else(|| Some("Transaction reverted without data".to_string()))
        } else {
            None
        };

        // Update nonces
        let updated_nonces = forked_state.nonces.clone();

        Ok(SequentialTransactionResult {
            transaction_index: 0, // Will be set by caller
            success,
            gas_used,
            revert_reason,
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
        let header = forked_state.block_header.clone();

        // Create tracer
        let unsigned_tx_config = TracingInspectorConfig::default_geth().set_record_logs(true);
        let mut inspector = TracingInspector::new(unsigned_tx_config);

        // Setup EVM environment
        let evm_env = self.evm_config.evm_env(&header);

        // Get base fee for gas price adjustment
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);

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
        let revert_reason = if !success {
            res.result
                .output()
                .map(|bytes| decode_revert_data(&bytes))
                .or_else(|| Some("Transaction reverted without data".to_string()))
        } else {
            None
        };

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

        // Determine transaction type
        let tx_type = if request.max_fee_per_gas.is_some() {
            2 // EIP-1559
        } else {
            0 // Legacy
        };

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

        // Calculate fees with base fee awareness
        let (gas_price, gas_priority_fee) = if tx_type == 2 {
            let priority_fee = request
                .max_priority_fee_per_gas
                .unwrap_or(fee_defaults.bundle_default_priority_fee);

            // If max_fee_per_gas is provided, use it; otherwise calculate from base fee
            let max_fee = if let Some(max_fee) = request.max_fee_per_gas {
                max_fee
            } else {
                let base = base_fee.unwrap_or(fee_defaults.pre_london_base_fee);
                base.saturating_mul(fee_defaults.bundle_max_fee_multiplier)
                    .saturating_add(priority_fee)
            };

            (max_fee, Some(priority_fee))
        } else {
            // Legacy
            let price = if let Some(price) = request.gas_price {
                price
            } else {
                let base = base_fee.unwrap_or(fee_defaults.legacy_pre_london_base_fee);
                base.saturating_mul(fee_defaults.legacy_gas_price_multiplier)
            };
            (price, None)
        };

        // Create TxEnv - no signature needed!
        Ok(TxEnv {
            tx_type,
            caller: caller.into(),
            gas_limit: request.gas.unwrap_or(block_gas_limit as u64),
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
            access_list: Default::default(),
            blob_hashes: Default::default(),
            max_fee_per_blob_gas: fee_defaults.max_fee_per_blob_gas,
            authorization_list: Default::default(),
        })
    }
}
