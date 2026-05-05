/// Unsigned Transaction Chain Simulation
///
/// This module provides the UnsignedTxChainSimulation struct which maintains blockchain state
/// between individual unsigned transaction simulations. This is essential for workflows where
/// each transaction depends on the results of previous ones (e.g., buy → approve → sell).
///
/// The chain maintains a forked state that persists between `step()` calls, allowing
/// complex multi-transaction workflows to be simulated accurately with inspector fusing
/// for optimal performance.
use crate::{
    simulation_revert_decoder::decode_revert_reason,
    simulator::TxSimulator,
    single_tx::unsigned::UnsignedTransaction,
    tx_chain::sequential::ForkedState,
    types::{FullSimulationResult, RevertContext, SimulationResult, ViewFunctionResult},
};
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use reth_primitives_traits::SealedHeader;
use reth_revm::primitives::KECCAK_EMPTY;
use reth_revm::Database;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use std::collections::HashMap;
use std::sync::Arc;

/// Information about the current chain state
#[derive(Debug, Clone)]
pub struct ChainStateInfo {
    /// Current block number
    pub block_number: u64,
    /// Number of transactions executed
    pub transaction_count: usize,
    /// Total gas used across all transactions
    pub total_gas_used: u64,
    /// Current nonces for addresses that have sent transactions
    pub nonces: HashMap<Address, u64>,
}

/// A stateful simulation chain that maintains blockchain state between unsigned transactions
pub struct UnsignedTxChainSimulation {
    /// Reference to the underlying simulator
    simulator: Arc<TxSimulator>,
    /// The forked blockchain state
    forked_state: ForkedState,
    /// Results from all executed transactions
    results: Vec<SimulationResult>,
    /// Total gas used
    total_gas_used: u64,
    /// Fused inspector that persists across transactions
    inspector: Option<TracingInspector>,
}

impl UnsignedTxChainSimulation {
    /// Create a new simulation chain
    pub(crate) fn new(simulator: Arc<TxSimulator>, forked_state: ForkedState) -> Self {
        Self {
            simulator,
            forked_state,
            results: Vec::new(),
            total_gas_used: 0,
            inspector: None,
        }
    }

    /// Execute a single transaction and advance the chain state
    ///
    /// This simulates the transaction on the current state and commits the changes,
    /// making them visible to subsequent transactions. Uses inspector fusing for
    /// optimal performance across multiple transactions.
    ///
    /// # Example
    /// ```rust
    /// let mut chain = simulator.start_simulation_chain(None).await?;
    /// let result = chain.step(buy_unsigned_tx).await?;
    /// // State now includes the effects of buy_unsigned_tx
    /// ```
    pub async fn step(&mut self, unsigned_tx: UnsignedTransaction) -> Result<SimulationResult> {
        let mut unsigned_tx = unsigned_tx;
        self.populate_missing_nonce(&mut unsigned_tx)?;

        let result = self.execute_with_fused_inspector(unsigned_tx.clone())?;
        self.record_simulation_result(unsigned_tx.from, &result);
        Ok(result)
    }

    /// Get information about the current chain state
    pub fn current_state(&self) -> ChainStateInfo {
        ChainStateInfo {
            block_number: self.forked_state.block_number,
            transaction_count: self.results.len(),
            total_gas_used: self.total_gas_used,
            nonces: self.forked_state.nonces.clone(),
        }
    }

    pub fn override_account_nonce(&mut self, address: Address, next_nonce: u64) {
        self.forked_state.nonces.insert(address, next_nonce);
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

    /// Set the forked account nonce for approximate selected-transaction replay.
    ///
    /// This is intended for live/pool-manager replay inputs that include selected setup
    /// transactions but omit earlier same-sender nonces from the block prefix.
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

    fn populate_missing_nonce(&mut self, tx: &mut UnsignedTransaction) -> Result<()> {
        if let Some(from) = tx.from {
            if tx.nonce.is_none() {
                let nonce = if let Some(&tracked_nonce) = self.forked_state.nonces.get(&from) {
                    tracked_nonce
                } else {
                    let db_nonce = self
                        .simulator
                        .get_nonce_from_state(&mut self.forked_state, from)?;
                    self.forked_state.nonces.insert(from, db_nonce);
                    db_nonce
                };
                tx.nonce = Some(nonce);
            }
        }

        Ok(())
    }

    fn record_simulation_result(&mut self, from: Option<Address>, result: &SimulationResult) {
        if result.success {
            self.total_gas_used += result.gas_used;

            if let Some(from) = from {
                let current = self.forked_state.nonces.entry(from).or_insert(0);
                *current += 1;
            }
        }

        self.results.push(result.clone());
    }

    pub(crate) fn into_forked_state(self) -> ForkedState {
        self.forked_state
    }

    /// Check whether an address currently has bytecode in the forked state.
    pub fn account_has_code(&mut self, address: Address) -> eyre::Result<bool> {
        let info = self.forked_state.db.basic(address)?;
        Ok(info
            .map(|acc| {
                acc.code
                    .as_ref()
                    .map(|code| !code.is_empty())
                    .unwrap_or_else(|| acc.code_hash != KECCAK_EMPTY)
            })
            .unwrap_or(false))
    }

    /// Execute a read-only contract call against the current forked state.
    pub fn simulate_view_call(
        &mut self,
        contract: Address,
        data: Bytes,
    ) -> Result<ViewFunctionResult> {
        let view_defaults = &self.simulator.defaults.view_call;
        let block_number = self.forked_state.block_number;
        let base_fee = self
            .forked_state
            .block_header
            .header()
            .base_fee_per_gas
            .map(|v| v as u128)
            .unwrap_or(1);

        let mut unsigned_tx = UnsignedTransaction::default();
        unsigned_tx.from = Some(view_defaults.from);
        unsigned_tx.to = Some(contract);
        unsigned_tx.value = Some(U256::ZERO);
        unsigned_tx.data = Some(data);
        unsigned_tx.gas = Some(view_defaults.gas_limit);
        unsigned_tx.max_fee_per_gas = Some(base_fee);
        unsigned_tx.max_priority_fee_per_gas = Some(0);

        let result = self.simulator.simulate_on_fork_with_trace(
            &mut self.forked_state,
            unsigned_tx,
            block_number,
        )?;

        Ok(ViewFunctionResult {
            success: result.success,
            output: result.call_trace.output.clone().unwrap_or_default(),
            gas_used: result.gas_used,
        })
    }

    /// Internal method to execute transaction with fused inspector
    fn execute_with_fused_inspector(
        &mut self,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        use reth_evm::{ConfigureEvm, Evm};
        use reth_revm::DatabaseCommit;

        let block_header = self.forked_state.block_header.clone();

        // Setup EVM environment
        let evm_env = self
            .simulator
            .evm_config
            .evm_env(&block_header)
            .map_err(|err| eyre::eyre!("failed to build EVM env: {}", err))?;
        let base_fee = block_header.header().base_fee_per_gas.map(|v| v as u128);

        let initial_context = if let Some(target) = unsigned_tx.to {
            let has_code = self.account_has_code(target)?;
            Some(RevertContext {
                target,
                has_code,
                calldata_len: unsigned_tx.data.as_ref().map(|d| d.len()).unwrap_or(0),
            })
        } else {
            None
        };

        // Create transaction environment
        let tx_env = self.simulator.create_tx_env_from_unsigned_tx(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut self.forked_state.db,
        )?;

        // Execute transaction with inspector
        let res = {
            let inspector = self.inspector.get_or_insert_with(|| {
                let config = TracingInspectorConfig::default_geth();
                TracingInspector::new(config)
            });

            let mut evm = self.simulator.evm_config.evm_with_env_and_inspector(
                &mut self.forked_state.db,
                evm_env,
                inspector,
            );
            evm.transact(tx_env)?
        };

        // Commit state changes
        self.forked_state.db.commit(res.state);

        // Fuse the inspector for next transaction (clear tx-specific data, keep internal buffers)
        self.inspector = self.inspector.take().map(|insp| insp.fused());

        let success = res.result.is_success();
        let gas_used = res.result.tx_gas_used();
        let revert_data = res.result.output().cloned();
        let mut revert_reason = if success {
            None
        } else {
            decode_revert_reason(revert_data.as_ref(), initial_context.as_ref())
        };
        if !success && revert_reason.is_none() {
            revert_reason = Some("Transaction reverted without data".to_string());
        }
        let revert_context = if success { None } else { initial_context };

        Ok(SimulationResult {
            success,
            gas_used,
            revert_reason,
            revert_context,
        })
    }

    /// Execute a single transaction with full trace information
    ///
    /// Similar to `step()` but returns comprehensive trace data including
    /// internal transactions, event logs, and unsigned_tx traces.
    pub async fn step_with_trace(
        &mut self,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<FullSimulationResult> {
        // Prepare the unsigned_tx with automatic nonce management
        let mut unsigned_tx = unsigned_tx;
        self.populate_missing_nonce(&mut unsigned_tx)?;

        // Use simulate_on_fork_with_trace to get full details
        let block_number = self.forked_state.block_number;
        let result = self.simulator.simulate_on_fork_with_trace(
            &mut self.forked_state,
            unsigned_tx.clone(),
            block_number,
        )?;

        let summary = SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason.clone(),
            revert_context: result.revert_context.clone(),
        };
        self.record_simulation_result(unsigned_tx.from, &summary);

        Ok(result)
    }
}

impl TxSimulator {
    /// Start a new simulation chain at the specified block or using a supplied header.
    ///
    /// Creates a stateful simulation environment where each transaction
    /// builds on the state changes from previous ones.
    ///
    /// # Example
    /// ```rust
    /// let simulator = TxSimulator::new("/path/to/db")?;
    /// let mut chain = simulator.start_simulation_chain(None).await?;
    ///
    /// // Execute transactions sequentially with state preservation
    /// let buy_result = chain.step(buy_tx).await?;
    /// let approve_result = chain.step(approve_tx).await?;
    /// let sell_result = chain.step(sell_tx).await?;
    /// ```
    pub async fn start_simulation_chain(
        &self,
        at_block: Option<u64>,
    ) -> Result<UnsignedTxChainSimulation> {
        self.start_simulation_chain_with_gas_block(at_block, None)
            .await
    }

    /// Start a simulation chain with state from `at_block` but allow overriding gas/environment
    /// parameters with `gas_block_number`. When `gas_block_number` is `None`, it defaults to
    /// `at_block` (or the latest block if `at_block` is also `None`).
    pub async fn start_simulation_chain_with_gas_block(
        &self,
        at_block: Option<u64>,
        gas_block_number: Option<u64>,
    ) -> Result<UnsignedTxChainSimulation> {
        let latest = self.get_latest_block()?;
        let block_number = at_block.unwrap_or(latest);

        if block_number > latest {
            if let Some(mut forked_state) = self
                .block_context_loader()
                .replay_live_state(block_number, None)
                .await?
            {
                let gas_block = gas_block_number.unwrap_or(block_number);
                if gas_block != block_number {
                    self.override_forked_block_header_gas(&mut forked_state, gas_block)
                        .await?;
                }
                return Ok(UnsignedTxChainSimulation::new(
                    Arc::new(self.clone()),
                    forked_state,
                ));
            }
            return Err(eyre::eyre!(
                "state for block {} not yet available locally or via live cache",
                block_number
            ));
        }

        let mut forked_state = self.create_forked_state(block_number)?;
        let gas_block = gas_block_number.unwrap_or(block_number);
        if gas_block != block_number {
            self.override_forked_block_header_gas(&mut forked_state, gas_block)
                .await?;
        }

        Ok(UnsignedTxChainSimulation::new(
            Arc::new(self.clone()),
            forked_state,
        ))
    }
}

impl TxSimulator {
    async fn override_forked_block_header_gas(
        &self,
        forked_state: &mut ForkedState,
        gas_block_number: u64,
    ) -> Result<()> {
        let gas_header = self
            .block_context_loader()
            .load_block_header(gas_block_number, None)
            .await?;

        let mut combined_header = forked_state.block_header.header().clone();
        let src = gas_header.header();

        combined_header.gas_limit = src.gas_limit;
        combined_header.base_fee_per_gas = src.base_fee_per_gas;
        combined_header.blob_gas_used = src.blob_gas_used;
        combined_header.excess_blob_gas = src.excess_blob_gas;
        combined_header.parent_beacon_block_root = src.parent_beacon_block_root;
        combined_header.requests_hash = src.requests_hash;

        forked_state.block_header = SealedHeader::new_unhashed(combined_header);
        Ok(())
    }
}
