use crate::single_tx::unsigned::UnsignedTransaction;
/// Signed Transaction Chain Simulation
///
/// Maintains a forked state and executes signed transactions sequentially,
/// committing state changes between steps. Useful for buy → approve → sell
/// flows with real signatures.
use crate::{
    simulation_revert_decoder::decode_revert_reason,
    simulator::TxSimulator,
    tx_chain::sequential::ForkedState,
    types::{FullSimulationResult, SimulationResult, ViewCallOverrides, ViewFunctionResult},
};
use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm::{ConfigureEvm, Evm, EvmEnvFor, TxEnvFor};
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives_traits::{Recovered, SealedHeader};
use reth_revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use std::sync::Arc;

struct PreparedSignedExecution {
    evm_env: EvmEnvFor<EthEvmConfig>,
    tx_env: TxEnvFor<EthEvmConfig>,
    gas_limit: u64,
}

/// Stateful signed-tx chain simulator
pub struct SignedTxChainSimulation {
    simulator: Arc<TxSimulator>,
    forked_state: ForkedState,
}

impl SignedTxChainSimulation {
    pub(crate) fn new(simulator: Arc<TxSimulator>, forked_state: ForkedState) -> Self {
        Self {
            simulator,
            forked_state,
        }
    }

    /// Execute a signed transaction and persist its state changes
    pub fn step(&mut self, tx: &TransactionSigned) -> Result<SimulationResult> {
        self.simulator
            .simulate_signed_on_fork_without_trace(&mut self.forked_state, tx)
    }

    /// Same as step() but returns full trace
    pub fn step_with_trace(&mut self, tx: &TransactionSigned) -> Result<FullSimulationResult> {
        self.simulator
            .simulate_signed_on_fork_with_trace(&mut self.forked_state, tx)
    }

    /// Execute a read-only call against the current forked state and return raw output
    pub fn view_call_on_fork_with_options(
        &mut self,
        to: Address,
        data: Bytes,
        overrides: Option<ViewCallOverrides>,
    ) -> Result<ViewFunctionResult> {
        // Build an unsigned view tx
        let resolved = overrides
            .unwrap_or_default()
            .resolve(&self.simulator.simulation_defaults().view_call);
        let unsigned = UnsignedTransaction {
            from: Some(resolved.from),
            to: Some(to),
            gas: Some(resolved.gas_limit),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            value: Some(U256::ZERO),
            data: Some(data.clone()),
            nonce: None,
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        };

        self.simulator
            .simulate_view_on_fork_without_commit(&mut self.forked_state, unsigned)
    }

    /// Backward compatibility helper using default overrides
    pub fn view_call_on_fork(&mut self, to: Address, data: Bytes) -> Result<ViewFunctionResult> {
        self.view_call_on_fork_with_options(to, data, None)
    }

    /// Convenience: ERC-20 balanceOf on forked state
    pub fn erc20_balance_of_on_fork(&mut self, token: Address, owner: Address) -> Result<U256> {
        // balanceOf(address) selector: 0x70a08231
        let mut data = vec![0x70, 0xa0, 0x82, 0x31];
        let mut padded = [0u8; 32];
        padded[12..].copy_from_slice(owner.as_slice());
        data.extend_from_slice(&padded);
        let out = self.view_call_on_fork(token, Bytes::from(data))?;
        if !out.success {
            return Ok(U256::ZERO);
        }
        if out.output.len() >= 32 {
            Ok(U256::from_be_slice(&out.output[..32]))
        } else {
            Ok(U256::ZERO)
        }
    }

    /// Get native ETH balance for an address from the forked state
    pub fn eth_balance_of_on_fork(&mut self, owner: Address) -> Result<U256> {
        // Use the DB 'basic' to fetch account info including balance
        let acc = self.forked_state.db.basic(owner.into())?;
        Ok(acc.map(|a| U256::from(a.balance)).unwrap_or(U256::ZERO))
    }

    /// Set native ETH balance for an address on the forked state.
    ///
    /// This is useful for signed simulation examples and tests where the signer is
    /// arbitrary but the signature must still recover to that signer.
    pub fn set_eth_balance_on_fork(&mut self, owner: Address, balance: U256) -> Result<U256> {
        let mut account = self
            .forked_state
            .db
            .basic(owner.into())?
            .unwrap_or_default();
        let previous = U256::from(account.balance);
        account.balance = balance;
        self.forked_state.db.insert_account_info(owner, account);
        Ok(previous)
    }

    /// Get current nonce for an address from the forked state
    pub fn nonce_of(&mut self, address: Address) -> Result<u64> {
        self.simulator
            .get_nonce_from_state(&mut self.forked_state, address)
    }

    fn revert_reason_from(success: bool, revert_data: Option<&Bytes>) -> Option<String> {
        if success {
            return None;
        }

        let mut reason = decode_revert_reason(revert_data, None);
        if reason.is_none() {
            reason = Some("Transaction reverted".to_string());
        }
        reason
    }
}

impl TxSimulator {
    pub(crate) fn simulate_signed_on_fork_without_trace(
        &self,
        forked_state: &mut ForkedState,
        tx: &TransactionSigned,
    ) -> Result<SimulationResult> {
        let PreparedSignedExecution {
            evm_env, tx_env, ..
        } = self.prepare_signed_execution_on_fork(forked_state, tx)?;

        let mut evm = self.evm_config.evm_with_env(&mut forked_state.db, evm_env);
        let res = evm.transact(tx_env)?;
        forked_state.db.commit(res.state);

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let revert_reason =
            SignedTxChainSimulation::revert_reason_from(success, res.result.output());

        Ok(SimulationResult {
            success,
            gas_used,
            revert_reason,
            revert_context: None,
        })
    }

    pub(crate) fn simulate_signed_on_fork_with_trace(
        &self,
        forked_state: &mut ForkedState,
        tx: &TransactionSigned,
    ) -> Result<FullSimulationResult> {
        let PreparedSignedExecution {
            evm_env,
            tx_env,
            gas_limit,
        } = self.prepare_signed_execution_on_fork(forked_state, tx)?;

        let mut inspector =
            TracingInspector::new(TracingInspectorConfig::default_geth().set_record_logs(true));
        let mut evm = self.evm_config.evm_with_env_and_inspector(
            &mut forked_state.db,
            evm_env,
            &mut inspector,
        );
        let res = evm.transact(tx_env)?;
        forked_state.db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let revert_reason =
            SignedTxChainSimulation::revert_reason_from(success, res.result.output());
        let call_frame = inspector
            .with_transaction_gas_limit(gas_limit)
            .into_geth_builder()
            .geth_call_traces(
                alloy_rpc_types_trace::geth::CallConfig::default().with_log(),
                gas_used,
            );

        Ok(FullSimulationResult {
            success,
            gas_used,
            revert_reason,
            revert_context: None,
            call_trace: call_frame,
            struct_logs: None,
            logs: emitted_logs,
        })
    }

    fn prepare_signed_execution_on_fork(
        &self,
        forked_state: &mut ForkedState,
        tx: &TransactionSigned,
    ) -> Result<PreparedSignedExecution> {
        let block_header = forked_state.block_header.clone();

        let evm_env = self
            .evm_config
            .evm_env(&block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;

        let recovered = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);
        let tx_env = self.evm_config.tx_env(&recovered);
        let gas_limit = tx_env.gas_limit;

        Ok(PreparedSignedExecution {
            evm_env,
            tx_env,
            gas_limit,
        })
    }
}

impl TxSimulator {
    /// Start a signed-tx chain simulator at optional block
    pub fn start_signed_chain(&self, at_block: Option<u64>) -> Result<SignedTxChainSimulation> {
        let block = at_block.unwrap_or(self.latest_historical_context_block_number()?);
        let fork = self.create_forked_state(block)?;
        Ok(SignedTxChainSimulation::new(Arc::new(self.clone()), fork))
    }

    /// Start a signed-tx chain simulator using a provided block header snapshot.
    pub fn start_signed_chain_with_header(
        &self,
        block_header: SealedHeader,
    ) -> Result<SignedTxChainSimulation> {
        let block = block_header.number;
        let fork = self.create_forked_state_with_header(block, block_header)?;
        Ok(SignedTxChainSimulation::new(Arc::new(self.clone()), fork))
    }
}
