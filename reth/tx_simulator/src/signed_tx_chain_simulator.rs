/// Signed Transaction Chain Simulation
///
/// Maintains a forked state and executes signed transactions sequentially,
/// committing state changes between steps. Useful for buy → approve → sell
/// flows with real signatures.

use crate::{
    simulator::TxSimulator,
    types::{SimulationResult, FullSimulationResult, ViewFunctionResult},
    unsigned_tx_bundle_simulator::ForkedState,
};
use eyre::Result;
use std::sync::Arc;
use reth_primitives::{TransactionSigned, Recovered};
use alloy_consensus::transaction::SignerRecoverable;
use reth_provider::HeaderProvider;
use reth_evm::{ConfigureEvm, Evm};
use revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use alloy_primitives::{Address, Bytes, U256};
use crate::unsigned_tx_simulator::UnsignedTransaction;

/// Stateful signed-tx chain simulator
pub struct SignedTxChainSimulation {
    simulator: Arc<TxSimulator>,
    forked_state: ForkedState,
    block_number: u64,
    inspector: Option<TracingInspector>,
}

impl SignedTxChainSimulation {
    pub(crate) fn new(simulator: Arc<TxSimulator>, forked_state: ForkedState, block_number: u64) -> Self {
        Self { simulator, forked_state, block_number, inspector: None }
    }

    /// Execute a signed transaction and persist its state changes
    pub fn step(&mut self, tx: &TransactionSigned) -> Result<SimulationResult> {
        let provider = self.simulator.provider_factory.provider()?;
        let header = provider.header_by_number(self.block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", self.block_number))?;

        let evm_env = self.simulator.evm_config.evm_env(&header);

        // Recover sender and build tx env
        let recovered = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);
        let tx_env = self.simulator.evm_config.tx_env(&recovered);

        // Use fused inspector across steps
        let inspector = self.inspector.get_or_insert_with(|| TracingInspector::new(TracingInspectorConfig::default_geth()));
        let mut evm = self.simulator.evm_config.evm_with_env_and_inspector(&mut self.forked_state.db, evm_env, inspector);
        let res = evm.transact(tx_env)?;
        self.forked_state.db.commit(res.state);
        // Fuse inspector for subsequent steps
        self.inspector = self.inspector.take().map(|insp| insp.fused());

        Ok(SimulationResult {
            success: res.result.is_success(),
            gas_used: res.result.gas_used(),
            revert_reason: if res.result.is_success() { None } else { Some("Transaction reverted".to_string()) },
        })
    }

    /// Same as step() but returns full trace
    pub fn step_with_trace(&mut self, tx: &TransactionSigned) -> Result<FullSimulationResult> {
        let provider = self.simulator.provider_factory.provider()?;
        let header = provider.header_by_number(self.block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", self.block_number))?;

        let evm_env = self.simulator.evm_config.evm_env(&header);
        let recovered = Recovered::new_unchecked(tx.clone(), tx.recover_signer()?);
        let tx_env = self.simulator.evm_config.tx_env(&recovered);
        let gas_limit = tx_env.gas_limit;

        let mut inspector = TracingInspector::new(TracingInspectorConfig::default_geth().set_record_logs(true));
        let mut evm = self.simulator.evm_config.evm_with_env_and_inspector(&mut self.forked_state.db, evm_env, &mut inspector);
        let res = evm.transact(tx_env)?;
        self.forked_state.db.commit(res.state);

        let success = res.result.is_success();
        let gas_used = res.result.gas_used();
        let revert_reason = if success { None } else { Some("Transaction reverted".to_string()) };
        let call_frame = inspector
            .with_transaction_gas_limit(gas_limit)
            .into_geth_builder()
            .geth_call_traces(alloy_rpc_types_trace::geth::CallConfig::default().with_log(), gas_used);

        Ok(FullSimulationResult { success, gas_used, revert_reason, call_trace: call_frame })
    }

    /// Execute a read-only call against the current forked state and return raw output
    pub fn view_call_on_fork(&mut self, to: Address, data: Bytes) -> Result<ViewFunctionResult> {
        // Build an unsigned view tx
        let unsigned = UnsignedTransaction {
            from: Some(Address::ZERO),
            to: Some(to),
            gas: Some(3_000_000),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            value: Some(U256::ZERO),
            data: Some(data.clone()),
            nonce: None,
        };

        // Reuse internal forked-state simulator with trace to capture output
        let res = self.simulator.simulate_on_fork_with_trace(&mut self.forked_state, unsigned, self.block_number)?;
        let output = if res.success { res.call_trace.output.unwrap_or_default() } else { Bytes::new() };
        Ok(ViewFunctionResult { success: res.success, output, gas_used: res.gas_used })
    }

    /// Convenience: ERC-20 balanceOf on forked state
    pub fn erc20_balance_of_on_fork(&mut self, token: Address, owner: Address) -> Result<U256> {
        // balanceOf(address) selector: 0x70a08231
        let mut data = vec![0x70, 0xa0, 0x82, 0x31];
        let mut padded = [0u8; 32];
        padded[12..].copy_from_slice(owner.as_slice());
        data.extend_from_slice(&padded);
        let out = self.view_call_on_fork(token, Bytes::from(data))?;
        if !out.success { return Ok(U256::ZERO); }
        if out.output.len() >= 32 { Ok(U256::from_be_slice(&out.output[..32])) } else { Ok(U256::ZERO) }
    }

    /// Get native ETH balance for an address from the forked state
    pub fn eth_balance_of_on_fork(&mut self, owner: Address) -> Result<U256> {
        // Use the DB 'basic' to fetch account info including balance
        let acc = self.forked_state.db.basic(owner.into())?;
        Ok(acc.map(|a| U256::from(a.balance)).unwrap_or(U256::ZERO))
    }

    /// Get current nonce for an address from the forked state
    pub fn nonce_of(&mut self, address: Address) -> Result<u64> {
        self.simulator.get_nonce_from_state(&mut self.forked_state, address)
    }
}

impl TxSimulator {
    /// Start a signed-tx chain simulator at optional block
    pub fn start_signed_chain(&self, at_block: Option<u64>) -> Result<SignedTxChainSimulation> {
        let block = at_block.unwrap_or(self.get_latest_block()?);
        let fork = self.create_forked_state(block)?;
        Ok(SignedTxChainSimulation::new(Arc::new(self.clone()), fork, block))
    }
}
