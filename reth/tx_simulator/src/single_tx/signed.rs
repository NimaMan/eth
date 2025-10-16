/// Signed transaction simulation methods
///
/// This module contains methods for simulating fully signed transactions
/// with valid signatures (v, r, s).
use crate::{
    simulation_revert_decoder::decode_revert_reason,
    simulator::TxSimulator,
    types::{FullSimulationResult, SimulationResult},
};
use eyre::Result;
use tokio::task;

// Type alias for consistent naming style with UnsignedTransaction
pub type SignedTransaction = reth_primitives::TransactionSigned;

// Reth imports
use alloy_consensus::transaction::SignerRecoverable;
use alloy_rpc_types_trace::geth::CallConfig;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives::{Recovered, TransactionSigned};
use reth_provider::HeaderProvider;
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

impl TxSimulator {
    /// Simulate a SIGNED transaction and return basic result
    ///
    /// This method requires a fully signed transaction with valid signature (v, r, s).
    /// Use this when you have raw signed transactions from RPC or transaction pool.
    pub async fn simulate_signed_transaction(
        &self,
        tx: &TransactionSigned,
    ) -> Result<SimulationResult> {
        let latest_block = self.get_latest_block()?;
        self.simulate_signed_transaction_at_block(tx, latest_block)
            .await
    }

    /// Simulate a SIGNED transaction at specific block
    pub async fn simulate_signed_transaction_at_block(
        &self,
        tx: &TransactionSigned,
        block_number: u64,
    ) -> Result<SimulationResult> {
        let tx = tx.clone();
        let simulator = self.clone();

        task::spawn_blocking(move || {
            let provider = simulator.provider_factory.provider()?;
            let block_header = provider
                .header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;

            let state = simulator
                .provider_factory
                .history_by_block_number(block_number)?;

            let mut db = CacheDB::new(StateProviderDatabase::new(state));

            let mut inspector = TracingInspector::new(TracingInspectorConfig::default_parity());

            let evm_env = simulator
                .evm_config
                .evm_env(&block_header)
                .expect("failed to build EVM env");

            let recovered_tx = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);

            let tx_env = simulator.evm_config.tx_env(&recovered_tx);

            let mut evm =
                simulator
                    .evm_config
                    .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);

            let res = evm.transact(tx_env)?;

            db.commit(res.state);

            let success = res.result.is_success();
            let gas_used = res.result.gas_used();
            let revert_data = res.result.output().cloned();
            let mut revert_reason = if success {
                None
            } else {
                decode_revert_reason(revert_data.as_ref(), None)
            };
            if !success && revert_reason.is_none() {
                revert_reason = Some(
                    "Transaction reverted and the simulator could not decode a specific reason"
                        .to_string(),
                );
            }

            Ok(SimulationResult {
                success,
                gas_used,
                revert_reason,
                revert_context: None,
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    /// Simulate signed transaction with detailed call trace at specific block
    pub async fn simulate_signed_transaction_with_trace_at_block(
        &self,
        tx: &TransactionSigned,
        block_number: u64,
    ) -> Result<FullSimulationResult> {
        let tx = tx.clone();
        let simulator = self.clone();

        task::spawn_blocking(move || {
            let provider = simulator.provider_factory.provider()?;
            let block_header = provider
                .header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;

            let state = simulator
                .provider_factory
                .history_by_block_number(block_number)?;

            let mut db = CacheDB::new(StateProviderDatabase::new(state));

            // Create tracer with call config
            let call_config = TracingInspectorConfig::default_geth()
                .set_steps(false)
                .set_state_diffs(false)
                .disable_stack_snapshots()
                .set_record_logs(true);
            let mut inspector = TracingInspector::new(call_config);

            let evm_env = simulator
                .evm_config
                .evm_env(&block_header)
                .expect("failed to build EVM env");

            let recovered_tx = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);

            let tx_env = simulator.evm_config.tx_env(&recovered_tx);
            let gas_limit = tx_env.gas_limit;

            let mut evm =
                simulator
                    .evm_config
                    .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);

            let res = evm.transact(tx_env)?;

            db.commit(res.state);

            let success = res.result.is_success();
            let gas_used = res.result.gas_used();
            let revert_data = res.result.output().cloned();
            let mut revert_reason = if success {
                None
            } else {
                decode_revert_reason(revert_data.as_ref(), None)
            };
            if !success && revert_reason.is_none() {
                revert_reason = Some(
                    "Transaction reverted without returning data and no specific reason could be decoded".to_string(),
                );
            }

            // Extract call trace
            let call_frame = inspector
                .with_transaction_gas_limit(gas_limit)
                .into_geth_builder()
                .geth_call_traces(CallConfig::default().with_log(), gas_used);

            Ok(FullSimulationResult {
                success,
                gas_used,
                revert_reason,
                revert_context: None,
                call_trace: call_frame,
                struct_logs: None,
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    /// Simulate signed transaction with detailed call trace
    pub async fn simulate_signed_transaction_with_trace(
        &self,
        tx: &TransactionSigned,
        block_number: Option<u64>,
    ) -> Result<FullSimulationResult> {
        let block = block_number.unwrap_or(self.get_latest_block()?);
        let tx = tx.clone();
        let simulator = self.clone();

        task::spawn_blocking(move || {
            let provider = simulator.provider_factory.provider()?;
            let block_header = provider
                .header_by_number(block)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block))?;

            let state = simulator.provider_factory.history_by_block_number(block)?;

            let mut db = CacheDB::new(StateProviderDatabase::new(state));

            // Create tracer with call config
            let call_config = TracingInspectorConfig::default_geth()
                .set_steps(false)
                .set_state_diffs(false)
                .disable_stack_snapshots()
                .set_record_logs(true);
            let mut inspector = TracingInspector::new(call_config);

            let evm_env = simulator
                .evm_config
                .evm_env(&block_header)
                .expect("failed to build EVM env");

            let recovered_tx = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);

            let tx_env = simulator.evm_config.tx_env(&recovered_tx);
            let gas_limit = tx_env.gas_limit;

            let mut evm =
                simulator
                    .evm_config
                    .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);

            let res = evm.transact(tx_env)?;

            db.commit(res.state);

            let success = res.result.is_success();
            let gas_used = res.result.gas_used();
            let revert_data = res.result.output().cloned();
            let mut revert_reason = if success {
                None
            } else {
                decode_revert_reason(revert_data.as_ref(), None)
            };
            if !success && revert_reason.is_none() {
                revert_reason = Some("Transaction reverted".to_string());
            }

            // Extract call trace
            let call_frame = inspector
                .with_transaction_gas_limit(gas_limit)
                .into_geth_builder()
                .geth_call_traces(CallConfig::default().with_log(), gas_used);

            Ok(FullSimulationResult {
                success,
                gas_used,
                revert_reason,
                revert_context: None,
                call_trace: call_frame,
                struct_logs: None,
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }
}
