/// Call simulation (unsigned transactions)
///
/// This module provides eth_call-equivalent execution along with tracing helpers that mirror
/// reth's `/debug/trace_call` and `/debug/trace_transaction` endpoints, all without going through
/// RPC. The "full trace" helpers enable step recording so `FullSimulationResult::struct_logs` is populated,
/// matching the high-fidelity output callers expect from `debug_traceTransaction`.
use crate::{
    simulation_revert_decoder::decode_revert_data,
    simulator::TxSimulator,
    types::{FullSimulationResult, SimulationResult},
};
use eyre::Result;
use tokio::task;

// Reth imports
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_types_trace::geth::{CallConfig, GethDefaultTracingOptions};
use reth_evm::{ConfigureEvm, Evm};
use reth_primitives::SealedHeader;
use reth_provider::{HeaderProvider, StateProviderBox};
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_revm::DatabaseCommit;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use serde::{Deserialize, Serialize};

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
}

impl TxSimulator {
    /// Simulate an unsigned transaction against the latest block (eth_call equivalent).
    pub async fn simulate_unsigned_transaction(
        &self,
        unsigned_tx: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let block_number = self.get_latest_block()?;
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block_number)
            .await
    }

    /// Simulate an unsigned transaction at a specific block (eth_call with block parameter).
    pub async fn simulate_unsigned_transaction_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        let (block_header, state) = self.prepare_block_context(block_number, None).await?;
        self.execute_unsigned_transaction(unsigned_tx, block_header, state)
            .await
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
        block_header: Option<SealedHeader>,
    ) -> Result<FullSimulationResult> {
        let block_number = block_number.unwrap_or(self.get_latest_block()?);
        let (block_header, state) =
            self.prepare_block_context(block_number, block_header).await?;
        self.execute_unsigned_transaction_with_trace(unsigned_tx, block_header, state)
            .await
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
        let (block_header, state) = self.prepare_block_context(block_number, None).await?;
        self.execute_unsigned_transaction_with_full_trace(unsigned_tx, block_header, state)
            .await
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

    async fn prepare_block_context(
        &self,
        block_number: u64,
        block_header: Option<SealedHeader>,
    ) -> Result<(SealedHeader, StateProviderBox)> {
        let resolved_header = match block_header {
            Some(block_header) => block_header,
            None => self.fetch_block_header(block_number)?,
        };
        let state = self.load_state_for_block(block_number).await?;
        Ok((resolved_header, state))
    }

    fn fetch_block_header(&self, block_number: u64) -> Result<SealedHeader> {
        let provider = self.provider_factory.provider()?;
        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;
        Ok(SealedHeader::new_unhashed(header))
    }

    async fn execute_unsigned_transaction(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<SimulationResult> {
        let simulator = self.clone();
        task::spawn_blocking(move || {
            Self::run_unsigned_transaction(simulator, unsigned_tx, block_header, state)
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
            Self::run_unsigned_transaction_with_trace(
                simulator,
                unsigned_tx,
                block_header,
                state,
                TracingInspectorConfig::default_geth()
                    .set_record_logs(true)
                    .set_steps(true),
                CallConfig::default().with_log(),
            )
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }

    fn run_unsigned_transaction(
        simulator: TxSimulator,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
    ) -> Result<SimulationResult> {
        let block_header = block_header.into_header();
        let mut db = CacheDB::new(StateProviderDatabase::new(state));
        let mut inspector = TracingInspector::new(TracingInspectorConfig::default_parity());
        let evm_env = simulator
            .evm_config
            .evm_env(&block_header)
            .expect("failed to build EVM env");
        let base_fee = block_header.base_fee_per_gas.map(|v| v as u128);
        let tx_env = simulator.create_tx_env(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut db,
        )?;
        let mut evm = simulator
            .evm_config
            .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);

        let res = evm.transact(tx_env)?;
        db.commit(res.state);

        Ok(SimulationResult {
            success: res.result.is_success(),
            gas_used: res.result.gas_used(),
            revert_reason: if res.result.is_success() {
                None
            } else {
                res.result
                    .output()
                    .map(|bytes| decode_revert_data(&bytes))
                    .or_else(|| Some("Transaction reverted without data".to_string()))
            },
        })
    }

    fn run_unsigned_transaction_with_trace(
        simulator: TxSimulator,
        unsigned_tx: UnsignedTransaction,
        block_header: SealedHeader,
        state: StateProviderBox,
        inspector_config: TracingInspectorConfig,
        call_config: CallConfig,
    ) -> Result<FullSimulationResult> {
        let block_header = block_header.into_header();
        let mut db = CacheDB::new(StateProviderDatabase::new(state));

        let mut inspector = TracingInspector::new(inspector_config);
        let evm_env = simulator
            .evm_config
            .evm_env(&block_header)
            .expect("failed to build EVM env");
        let base_fee = block_header.base_fee_per_gas.map(|v| v as u128);
        let tx_env = simulator.create_tx_env(
            &unsigned_tx,
            evm_env.block_env.gas_limit as u128,
            base_fee,
            &mut db,
        )?;
        let gas_limit = tx_env.gas_limit;

        let mut evm = simulator
            .evm_config
            .evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);
        let res = evm.transact(tx_env)?;
        db.commit(res.state);

        let success = res.result.is_success();
        let gas_used = res.result.gas_used();
        let raw_output = res.result.output().cloned();
        let revert_reason = if success {
            None
        } else {
            raw_output
                .as_ref()
                .map(|bytes| decode_revert_data(bytes))
                .or_else(|| Some("Transaction reverted without data".to_string()))
        };

        let builder = inspector.with_transaction_gas_limit(gas_limit).into_geth_builder();
        let call_frame = builder.geth_call_traces(call_config, gas_used);
        let struct_logs = if inspector_config.record_steps {
            let return_value = raw_output.clone().unwrap_or_default();
            let trace_opts = GethDefaultTracingOptions::default();
            Some(builder.geth_traces(gas_used, return_value, trace_opts).struct_logs)
        } else {
            None
        };

        Ok(FullSimulationResult {
            success,
            gas_used,
            revert_reason,
            call_trace: call_frame,
            struct_logs,
        })
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
            // EIP-1559
            let tip_divisor = fee_defaults.derived_tip_divisor.max(1);
            let base = base_fee.unwrap_or(fee_defaults.pre_london_base_fee);
            let derived_tip = (base / tip_divisor).max(fee_defaults.min_priority_fee);
            let priority_fee = request.max_priority_fee_per_gas.unwrap_or(derived_tip);

            // If max_fee_per_gas is provided, use it; otherwise calculate from base fee + tip + cushion
            let max_fee = if let Some(max_fee) = request.max_fee_per_gas {
                max_fee
            } else {
                let cushion_divisor = fee_defaults.priority_fee_cushion_divisor.max(1);
                let cushion = (base / cushion_divisor).max(fee_defaults.priority_fee_min_cushion);
                base.saturating_add(priority_fee).saturating_add(cushion)
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
