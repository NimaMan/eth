use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use alloy_primitives::Address;
use eth_alpha_core::{ids::TokenPoolId, market::PoolSnapshot};
use eth_alpha_store::PostgresTradingStore;
use eyre::{Result, WrapErr};

use crate::wire::parse_address;
use crate::{EngineExecutionAdapter, LiveChainSimExecutionAdapter};

use super::backtest::ChainSimGasPolicyBacktestAdapter;
use super::cli::RealExecutionArgs;
use super::execution_lifecycle::ChainSimSettlement;
use super::gas_policy::LiveRealGasPolicy;
use super::real_execution::{build_kartal_real_adapter, KartalRealPreflight};
use super::receipt_reconciliation::{JsonRpcReceiptProvider, VaultReceiptReconciler};
use super::support::TraderExecutionMode;
use super::token_server::TokenServerClient;

pub(super) struct ExecutionStackInput<'a> {
    pub(super) execution_mode: TraderExecutionMode,
    pub(super) real_args: Option<&'a RealExecutionArgs>,
    pub(super) token_server_url: &'a str,
    pub(super) reth_datadir: &'a str,
    pub(super) reth_http_rpc: &'a str,
    pub(super) store: PostgresTradingStore,
    pub(super) run_id: String,
    pub(super) live_gas_policy: LiveRealGasPolicy,
    pub(super) live_real_gas_policy: Option<LiveRealGasPolicy>,
    pub(super) kartal_real_preflight: Option<KartalRealPreflight>,
}

pub(super) struct ExecutionStack {
    pub(super) adapter: Box<dyn EngineExecutionAdapter>,
    pub(super) adapter_current_block: Arc<AtomicU64>,
    pub(super) pool_updates: Arc<Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    pub(super) manual_close_live_simulator: Option<tx_simulator::LiveTxSimulator>,
    pub(super) state_status_adapter: LiveChainSimExecutionAdapter,
    pub(super) manual_close_vault_address: Option<Address>,
    pub(super) chain_sim_settlement: Option<ChainSimSettlement>,
    pub(super) receipt_reconciler: Option<VaultReceiptReconciler<JsonRpcReceiptProvider>>,
    pub(super) next_order_sequence: u64,
}

pub(super) async fn build_execution_stack(
    input: ExecutionStackInput<'_>,
) -> Result<ExecutionStack> {
    let tx_simulator = Arc::new(
        tx_simulator::TxSimulator::new(input.reth_datadir)
            .wrap_err("failed to initialize tx simulator")?,
    );
    let live_state_provider = tx_simulator::InMemoryLiveBlockStateProvider::new();
    let live_simulator =
        tx_simulator::LiveTxSimulator::new(tx_simulator.clone(), live_state_provider.clone());
    let local_live_state_enabled = input.execution_mode == TraderExecutionMode::ChainSim;
    if local_live_state_enabled {
        super::live_state::spawn_live_state_publisher(
            TokenServerClient::new(input.token_server_url.to_string()),
            live_state_provider,
            tx_simulator.clone(),
        );
    }
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let next_order_sequence = input
        .store
        .max_order_sequence_for_prefix(&input.run_id)
        .await
        .wrap_err("failed to restore alpha trader order sequence")?;
    let chain_sim_adapter = LiveChainSimExecutionAdapter::with_prefix_and_next_order_sequence(
        live_simulator.clone(),
        tx_processor,
        input.run_id.clone(),
        next_order_sequence,
    )
    .wrap_err("failed to initialize chain-sim execution adapter")?;
    let adapter_current_block = chain_sim_adapter.current_block();
    let pool_updates = chain_sim_adapter.pools();
    let exact_pre_submit_live_simulator =
        local_live_state_enabled.then_some(live_simulator.clone());
    let manual_close_live_simulator = exact_pre_submit_live_simulator.clone();
    let state_status_adapter = chain_sim_adapter.clone();
    let manual_close_vault_address = match (input.execution_mode, input.real_args) {
        (TraderExecutionMode::KartalReal, Some(real_args)) => {
            Some(parse_address(&real_args.live_real_vault_address)?)
        }
        _ => None,
    };
    let chain_sim_settlement = if input.execution_mode == TraderExecutionMode::ChainSim {
        Some(ChainSimSettlement::new(
            input.store.clone(),
            chain_sim_adapter.clone(),
            Some(ChainSimGasPolicyBacktestAdapter::new(
                chain_sim_adapter.clone(),
                input.token_server_url.to_string(),
                input.reth_http_rpc.to_string(),
                input.live_gas_policy.clone(),
            )),
        ))
    } else {
        None
    };
    let receipt_reconciler = match (
        input.execution_mode,
        input.real_args,
        input.kartal_real_preflight.as_ref(),
    ) {
        (TraderExecutionMode::KartalReal, Some(real_args), Some(preflight)) => {
            let vault = parse_address(&real_args.live_real_vault_address)?;
            Some(VaultReceiptReconciler::new(
                JsonRpcReceiptProvider::new(preflight.status.rpc_url.clone()),
                vault,
            ))
        }
        _ => None,
    };
    let adapter: Box<dyn EngineExecutionAdapter> = match input.execution_mode {
        TraderExecutionMode::ChainSim => Box::new(ChainSimGasPolicyBacktestAdapter::new(
            chain_sim_adapter,
            input.token_server_url.to_string(),
            input.reth_http_rpc.to_string(),
            input.live_gas_policy.clone(),
        )),
        TraderExecutionMode::KartalReal => {
            let real_args = input
                .real_args
                .expect("kartal-real execution requires real args");
            build_kartal_real_adapter(
                real_args,
                input
                    .kartal_real_preflight
                    .expect("kartal-real preflight must exist"),
                input.token_server_url.to_string(),
                input.store.clone(),
                input.run_id.clone(),
                chain_sim_adapter,
                exact_pre_submit_live_simulator,
                pool_updates.clone(),
                adapter_current_block.clone(),
                input
                    .live_real_gas_policy
                    .expect("kartal-real gas policy must exist"),
            )
            .await?
        }
    };

    Ok(ExecutionStack {
        adapter,
        adapter_current_block,
        pool_updates,
        manual_close_live_simulator,
        state_status_adapter,
        manual_close_vault_address,
        chain_sim_settlement,
        receipt_reconciler,
        next_order_sequence,
    })
}
