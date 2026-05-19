use std::{collections::HashMap, sync::Arc};

use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types_trace::geth::CallConfig;
use eyre::Result;
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives_traits::Recovered;
use reth_revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

use crate::{
    revert::decode_revert_reason,
    simulator::TxSimulator,
    single_tx::{signed::SignedTransaction, unsigned::UnsignedTransaction},
    tx_chain::{sequential::ForkedState, unsigned::UnsignedTxChainSimulation},
    tx_fee_parameters::effective_paid_gas_price_from_tx_env,
    types::{FullSimulationResult, SimulationResult, ViewCallOverrides, ViewFunctionResult},
};

/// Fork selection for a stateful simulation session.
#[derive(Debug, Clone, Default)]
pub struct SimulationSessionOptions {
    /// State block to fork. `None` means latest local historical context.
    pub at_block: Option<u64>,
    /// Optional block whose gas/base-fee fields should be used for tx environment construction.
    pub gas_block_number: Option<u64>,
}

/// Information about the current forked session state.
#[derive(Debug, Clone)]
pub struct SimulationSessionState {
    pub block_number: u64,
    pub transaction_count: usize,
    pub total_gas_used: u64,
    pub nonces: HashMap<Address, u64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SessionTransactionKind {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone)]
pub enum SessionTransaction {
    Unsigned(UnsignedTransaction),
    Signed(SignedTransaction),
}

#[derive(Debug, Clone)]
pub struct SessionStepSummary {
    pub transaction_index: usize,
    pub transaction_kind: SessionTransactionKind,
    pub success: bool,
    pub gas_used: u64,
    pub cumulative_gas_used: u64,
    pub revert_reason: Option<String>,
}

/// A warm, forked state session for mixed unsigned and signed transaction sequences.
pub struct SimulationSession {
    simulator: Arc<TxSimulator>,
    forked_state: ForkedState,
    results: Vec<SessionStepSummary>,
    total_gas_used: u64,
}

impl SimulationSession {
    pub(crate) fn new(simulator: Arc<TxSimulator>, forked_state: ForkedState) -> Self {
        Self {
            simulator,
            forked_state,
            results: Vec::new(),
            total_gas_used: 0,
        }
    }

    /// Current fork/session state counters.
    pub fn current_state(&self) -> SimulationSessionState {
        SimulationSessionState {
            block_number: self.forked_state.block_number,
            transaction_count: self.results.len(),
            total_gas_used: self.total_gas_used,
            nonces: self.forked_state.nonces.clone(),
        }
    }

    /// Per-step summaries accumulated by this session.
    pub fn results(&self) -> &[SessionStepSummary] {
        &self.results
    }

    /// Execute either an unsigned or signed transaction and commit its state changes.
    pub fn step(&mut self, tx: SessionTransaction) -> Result<SimulationResult> {
        match tx {
            SessionTransaction::Unsigned(tx) => self.step_unsigned(tx),
            SessionTransaction::Signed(tx) => self.step_signed(&tx),
        }
    }

    /// Execute an unsigned transaction on the current fork and commit state changes.
    pub fn step_unsigned(&mut self, unsigned_tx: UnsignedTransaction) -> Result<SimulationResult> {
        let mut unsigned_tx = unsigned_tx;
        self.populate_missing_unsigned_nonce(&mut unsigned_tx)?;

        let result = self
            .simulator
            .simulate_on_fork_without_trace(&mut self.forked_state, unsigned_tx.clone())?;
        self.record_unsigned_result(&unsigned_tx, &result);
        Ok(result)
    }

    /// Execute an unsigned transaction with call and struct-log tracing.
    pub fn step_unsigned_with_trace(
        &mut self,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<FullSimulationResult> {
        let mut unsigned_tx = unsigned_tx;
        self.populate_missing_unsigned_nonce(&mut unsigned_tx)?;

        let block_number = self.forked_state.block_number;
        let result = self.simulator.simulate_on_fork_with_trace(
            &mut self.forked_state,
            unsigned_tx.clone(),
            block_number,
        )?;
        let summary = SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            effective_gas_price: result.effective_gas_price,
            tx_type: result.tx_type,
            revert_reason: result.revert_reason.clone(),
            revert_context: result.revert_context.clone(),
        };
        self.record_unsigned_result(&unsigned_tx, &summary);
        Ok(result)
    }

    /// Execute a signed transaction on the current fork and commit state changes.
    pub fn step_signed(&mut self, tx: &SignedTransaction) -> Result<SimulationResult> {
        let prepared = self.prepare_signed_execution(tx)?;
        let mut evm = self
            .simulator
            .evm_config
            .evm_with_env(&mut self.forked_state.db, prepared.evm_env);
        let res = evm.transact(prepared.tx_env)?;
        self.forked_state.db.commit(res.state);

        let result = Self::signed_result(
            res.result.is_success(),
            res.result.tx_gas_used(),
            res.result.output(),
            prepared.effective_gas_price,
            prepared.tx_type,
        );
        self.record_signed_result(prepared.sender, prepared.nonce, &result);
        Ok(result)
    }

    /// Execute a signed transaction with call tracing.
    pub fn step_signed_with_trace(
        &mut self,
        tx: &SignedTransaction,
    ) -> Result<FullSimulationResult> {
        let prepared = self.prepare_signed_execution(tx)?;
        let mut inspector =
            TracingInspector::new(TracingInspectorConfig::default_geth().set_record_logs(true));
        let mut evm = self.simulator.evm_config.evm_with_env_and_inspector(
            &mut self.forked_state.db,
            prepared.evm_env,
            &mut inspector,
        );
        let res = evm.transact(prepared.tx_env)?;
        self.forked_state.db.commit(res.state);
        let emitted_logs = res.result.logs().to_vec();

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let revert_reason = Self::signed_revert_reason(success, res.result.output());
        let call_trace = inspector
            .with_transaction_gas_limit(prepared.gas_limit)
            .into_geth_builder()
            .geth_call_traces(CallConfig::default().with_log(), gas_used);

        let summary = SimulationResult {
            success,
            gas_used,
            effective_gas_price: prepared.effective_gas_price,
            tx_type: Some(prepared.tx_type),
            revert_reason: revert_reason.clone(),
            revert_context: None,
        };
        self.record_signed_result(prepared.sender, prepared.nonce, &summary);

        Ok(FullSimulationResult {
            success,
            gas_used,
            effective_gas_price: prepared.effective_gas_price,
            tx_type: Some(prepared.tx_type),
            revert_reason,
            revert_context: None,
            call_trace,
            struct_logs: None,
            logs: emitted_logs,
        })
    }

    /// Execute a read-only call against the current fork. State changes are discarded.
    pub fn view_call(
        &mut self,
        to: Address,
        data: Bytes,
        overrides: Option<ViewCallOverrides>,
    ) -> Result<ViewFunctionResult> {
        let resolved = overrides
            .unwrap_or_default()
            .resolve(&self.simulator.simulation_defaults().view_call);
        let base_fee = self
            .forked_state
            .block_header
            .header()
            .base_fee_per_gas
            .map(|v| v as u128)
            .unwrap_or(1);
        let unsigned = UnsignedTransaction {
            from: Some(resolved.from),
            to: Some(to),
            gas: Some(resolved.gas_limit),
            gas_price: None,
            max_fee_per_gas: Some(base_fee),
            max_priority_fee_per_gas: Some(0),
            value: Some(U256::ZERO),
            data: Some(data),
            nonce: None,
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        };
        self.simulator
            .simulate_view_on_fork_without_commit(&mut self.forked_state, unsigned)
    }

    pub fn account_nonce(&mut self, address: Address) -> Result<u64> {
        if let Some(&nonce) = self.forked_state.nonces.get(&address) {
            return Ok(nonce);
        }

        let nonce = self
            .simulator
            .get_nonce_from_state(&mut self.forked_state, address)?;
        self.forked_state.nonces.insert(address, nonce);
        Ok(nonce)
    }

    pub fn override_account_nonce(&mut self, address: Address, next_nonce: u64) {
        self.forked_state.nonces.insert(address, next_nonce);
    }

    pub fn set_account_nonce_for_replay(
        &mut self,
        address: Address,
        replay_nonce: u64,
    ) -> Result<u64> {
        let mut account = self.forked_state.db.basic(address)?.unwrap_or_default();
        let previous_nonce = account.nonce;
        account.nonce = replay_nonce;
        self.forked_state.db.insert_account_info(address, account);
        self.forked_state.nonces.insert(address, replay_nonce);
        Ok(previous_nonce)
    }

    pub fn account_has_code(&mut self, address: Address) -> Result<bool> {
        let info = self.forked_state.db.basic(address)?;
        Ok(info
            .map(|acc| {
                acc.code
                    .as_ref()
                    .map(|code| !code.is_empty())
                    .unwrap_or_else(|| acc.code_hash != reth_revm::primitives::KECCAK_EMPTY)
            })
            .unwrap_or(false))
    }

    pub fn eth_balance(&mut self, owner: Address) -> Result<U256> {
        let account = self.forked_state.db.basic(owner)?;
        Ok(account
            .map(|acc| U256::from(acc.balance))
            .unwrap_or(U256::ZERO))
    }

    pub fn set_eth_balance(&mut self, owner: Address, balance: U256) -> Result<U256> {
        let mut account = self.forked_state.db.basic(owner)?.unwrap_or_default();
        let previous = U256::from(account.balance);
        account.balance = balance;
        self.forked_state.db.insert_account_info(owner, account);
        Ok(previous)
    }

    fn populate_missing_unsigned_nonce(&mut self, tx: &mut UnsignedTransaction) -> Result<()> {
        if let Some(from) = tx.from {
            if tx.nonce.is_none() {
                let nonce = self.account_nonce(from)?;
                tx.nonce = Some(nonce);
            }
        }

        Ok(())
    }

    fn record_unsigned_result(&mut self, tx: &UnsignedTransaction, result: &SimulationResult) {
        if let Some(from) = tx.from {
            if let Some(nonce) = tx.nonce {
                self.forked_state
                    .nonces
                    .insert(from, nonce.saturating_add(1));
            } else {
                let current = self.forked_state.nonces.entry(from).or_insert(0);
                *current = current.saturating_add(1);
            }
        }
        self.record_result(SessionTransactionKind::Unsigned, result);
    }

    fn record_signed_result(&mut self, sender: Address, nonce: u64, result: &SimulationResult) {
        self.forked_state
            .nonces
            .insert(sender, nonce.saturating_add(1));
        self.record_result(SessionTransactionKind::Signed, result);
    }

    fn record_result(&mut self, kind: SessionTransactionKind, result: &SimulationResult) {
        self.total_gas_used = self.total_gas_used.saturating_add(result.gas_used);
        self.results.push(SessionStepSummary {
            transaction_index: self.results.len(),
            transaction_kind: kind,
            success: result.success,
            gas_used: result.gas_used,
            cumulative_gas_used: self.total_gas_used,
            revert_reason: result.revert_reason.clone(),
        });
    }

    fn prepare_signed_execution(
        &mut self,
        tx: &SignedTransaction,
    ) -> Result<PreparedSignedExecution> {
        let block_header = self.forked_state.block_header.clone();
        let evm_env = self
            .simulator
            .evm_config
            .evm_env(&block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let sender = tx.recover_signer()?;
        let recovered = Recovered::new_unchecked(tx.clone(), sender);
        let tx_env = self.simulator.evm_config.tx_env(&recovered);
        let gas_limit = tx_env.gas_limit;
        let nonce = tx_env.nonce;
        let tx_type = tx_env.tx_type;
        let effective_gas_price = effective_paid_gas_price_from_tx_env(
            tx_type,
            tx_env.gas_price,
            tx_env.gas_priority_fee,
            block_header.header().base_fee_per_gas.map(u128::from),
        );

        Ok(PreparedSignedExecution {
            evm_env,
            tx_env,
            gas_limit,
            sender,
            nonce,
            effective_gas_price,
            tx_type,
        })
    }

    fn signed_result(
        success: bool,
        gas_used: u64,
        output: Option<&Bytes>,
        effective_gas_price: Option<u128>,
        tx_type: u8,
    ) -> SimulationResult {
        SimulationResult {
            success,
            gas_used,
            effective_gas_price,
            tx_type: Some(tx_type),
            revert_reason: Self::signed_revert_reason(success, output),
            revert_context: None,
        }
    }

    fn signed_revert_reason(success: bool, revert_data: Option<&Bytes>) -> Option<String> {
        if success {
            return None;
        }

        decode_revert_reason(revert_data, None).or_else(|| Some("Transaction reverted".to_string()))
    }
}

struct PreparedSignedExecution {
    evm_env: reth_evm::EvmEnvFor<reth_evm_ethereum::EthEvmConfig>,
    tx_env: reth_evm::TxEnvFor<reth_evm_ethereum::EthEvmConfig>,
    gas_limit: u64,
    sender: Address,
    nonce: u64,
    effective_gas_price: Option<u128>,
    tx_type: u8,
}

impl TxSimulator {
    /// Start a stateful simulation session at the latest available block.
    pub async fn simulation_session(&self) -> Result<SimulationSession> {
        self.simulation_session_with_options(SimulationSessionOptions::default())
            .await
    }

    /// Start a stateful simulation session with explicit fork options.
    pub async fn simulation_session_with_options(
        &self,
        options: SimulationSessionOptions,
    ) -> Result<SimulationSession> {
        let chain: UnsignedTxChainSimulation = self
            .start_simulation_chain_with_gas_block(options.at_block, options.gas_block_number)
            .await?;
        Ok(SimulationSession::new(
            Arc::new(self.clone()),
            chain.into_forked_state(),
        ))
    }

    /// Convenience helper for starting a session at a specific state block.
    pub async fn simulation_session_at_block(
        &self,
        block_number: u64,
    ) -> Result<SimulationSession> {
        self.simulation_session_with_options(SimulationSessionOptions {
            at_block: Some(block_number),
            gas_block_number: None,
        })
        .await
    }
}
