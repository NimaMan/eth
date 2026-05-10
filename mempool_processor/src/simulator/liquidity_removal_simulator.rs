/// Liquidity Removal Simulator
/// A wrapper over `tx_processor::process_unsigned_tx` that returns a
/// `LiquidityRemovalResult` by deriving pool-drain metrics from the
/// `ProcessedTransaction.address_balance_changes`.
use crate::token_tracking::{PoolType as CachePoolType, TokenTrackingCache};
use alloy_primitives::{Address, U256};
use eyre::{eyre, Result};
use once_cell::sync::{Lazy, OnceCell};
use reth_chain_query::to_checksum_address;
use reth_chain_query::RethQueryProvider;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use tokio::time::{sleep, Duration};
use tracing::{error, warn};
use tx_processor::processed_tx_provider::ProcessedTxProvider;
use tx_processor::tx_processor::data_models::AddressBalanceChange;
use tx_processor::ProcessedTransaction;
use tx_simulator::{LiveTxSimulator, TxSimulator, UnsignedTransaction};

struct SimRequest {
    simulator: Arc<TxSimulator>,
    unsigned_tx: UnsignedTransaction,
    resolved_block: u64,
    responder: oneshot::Sender<Result<ProcessedTransaction>>,
}

struct SimWorker {
    sender: UnboundedSender<SimRequest>,
}

static SIM_PROVIDER: Lazy<OnceCell<Arc<ProcessedTxProvider>>> = Lazy::new(OnceCell::new);
static SIM_WORKER: Lazy<SimWorker> = Lazy::new(|| {
    // Dedicated runtime for tx_processor simulations (multi-thread to allow block_in_place)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build simulation runtime");

    let handle = runtime.handle().clone();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SimRequest>();

    // Move runtime into a dedicated thread to keep it alive
    std::thread::spawn(move || {
        // Keep runtime running until channel closes
        runtime.block_on(async move {
            while let Some(req) = rx.recv().await {
                let SimRequest {
                    simulator,
                    unsigned_tx,
                    resolved_block,
                    responder,
                } = req;

                // Lazily initialize and reuse a single provider to avoid per-call runtime drops.
                let provider = match SIM_PROVIDER.get_or_try_init(|| {
                    ProcessedTxProvider::with_simulator(simulator.clone()).map(Arc::new)
                }) {
                    Ok(p) => p.clone(),
                    Err(err) => {
                        let _ = responder.send(Err(err));
                        continue;
                    }
                };

                let fut_provider = provider.clone();
                let fut = async move {
                    fut_provider
                        .process_transaction_from_unsigned_tx(unsigned_tx, Some(resolved_block))
                        .await
                };
                let res = handle
                    .spawn(fut)
                    .await
                    .unwrap_or_else(|e| Err(eyre::eyre!("{}", e)));
                let _ = responder.send(res);
            }
        });
    });

    SimWorker { sender: tx }
});

#[derive(Debug, Clone)]
pub struct LiquidityRemovalResult {
    pub success: bool,
    pub revert_reason: Option<String>,
    pub address_balance_changes: HashMap<Address, AddressBalanceChange>,
    pub pool_address: Option<Address>,
    /// Canonical pool identifier for non-address pools too. V2/V3 use the pool
    /// address; V4 uses the `pool_manager#pool_id` display key used by eth_token.
    pub pool_identifier: Option<String>,
    pub pool_type: Option<String>,
    pub token_address: Option<Address>,
    pub function_name: Option<String>,
    pub metrics_known: bool,
    pub eth_removed: f64,
    pub drain_percentage: f64,
    pub remaining_eth: f64,
    pub is_scam: bool,
    pub debug_info: Option<String>,
}

#[derive(Debug)]
struct PoolDrainInfo {
    pool_address: Address,
    pool_identifier: String,
    pool_type: String,
    token_address: Option<Address>,
    eth_removed: f64,
    remaining_eth: f64,
    percentage: f64,
}

#[derive(Debug)]
struct ProtocolRemovalInfo {
    pool_address: Option<Address>,
    pool_identifier: String,
    pool_type: String,
    token_address: Address,
    function_name: &'static str,
}

pub struct LiquidityRemovalSimulator {
    simulator: Arc<TxSimulator>,
    live_tx_simulator: LiveTxSimulator,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl LiquidityRemovalSimulator {
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        let live_tx_simulator = LiveTxSimulator::from_simulator(simulator.clone());
        Self {
            simulator,
            live_tx_simulator,
            token_cache: None,
        }
    }

    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }

    /// Build a ProcessedTransaction for the unsigned tx and compute drain metrics.
    /// Only logs errors; otherwise stays quiet.
    pub async fn simulate_removal(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
        tx_hash: Option<&str>,
    ) -> Result<LiquidityRemovalResult> {
        self.simulate_removal_internal(unsigned_tx, block_number, false, tx_hash)
            .await
    }

    /// Run liquidity removal simulation with optional retry handling.
    /// When `retry_on_missing_header` is true, we will retry a few times if
    /// the provider has not yet materialized the header for the requested block.
    pub async fn simulate_removal_with_retry(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
        retry_on_missing_header: bool,
        tx_hash: Option<&str>,
    ) -> Result<LiquidityRemovalResult> {
        self.simulate_removal_internal(unsigned_tx, block_number, retry_on_missing_header, tx_hash)
            .await
    }

    async fn simulate_removal_internal(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
        retry_on_missing_header: bool,
        tx_hash: Option<&str>,
    ) -> Result<LiquidityRemovalResult> {
        // 1) Process tx with full trace + deltas
        let processed = match self
            .process_with_optional_retry(unsigned_tx.clone(), block_number, retry_on_missing_header)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                if let Some(expected_nonce) = Self::parse_nonce_mismatch(&e) {
                    return Ok(LiquidityRemovalResult {
                        success: false,
                        revert_reason: Some(format!(
                            "Nonce mismatch: expected nonce {} before liquidity removal",
                            expected_nonce
                        )),
                        address_balance_changes: HashMap::new(),
                        pool_address: None,
                        pool_identifier: None,
                        pool_type: None,
                        token_address: None,
                        function_name: Some("nonce_mismatch".to_string()),
                        metrics_known: false,
                        eth_removed: 0.0,
                        drain_percentage: 0.0,
                        remaining_eth: 0.0,
                        is_scam: true,
                        debug_info: None,
                    });
                }

                // If this looks like an insufficient funds error, log sender balance at the block
                let mut debug_info: Option<String> = None;
                let msg = e.to_string();
                let lower = msg.to_lowercase();
                if lower.contains("insufficient") || lower.contains("lack of funds") {
                    let from = unsigned_tx.from.unwrap_or_default();
                    let sim_block = match self.resolve_simulation_block_number(block_number).await {
                        Ok(number) => number,
                        Err(err) => {
                            warn!(
                                "Failed to resolve live simulation block for diagnostics: {}",
                                err
                            );
                            0
                        }
                    };
                    if sim_block > 0 {
                        if let Ok(rqp) = RethQueryProvider::with_simulator(self.simulator.clone()) {
                            // Gather diagnostics
                            let mut lines: Vec<String> = Vec::new();
                            let from_cs = to_checksum_address(&from);

                            if let Ok(balance) = rqp.get_eth_balance(from, Some(sim_block)).await {
                                let bal_eth = (balance.to::<u128>() as f64) / 1e18;
                                lines.push(format!(
                                    "sender {} balance at block {} = {:.6} ETH",
                                    from_cs, sim_block, bal_eth
                                ));
                            }

                            // Block gas metadata and base fees at sim_block and next block
                            if let Ok((gas_limit_hdr, _gas_used_hdr, base_fee_opt)) =
                                rqp.get_block_gas_metadata(sim_block)
                            {
                                if let Some(base_fee) = base_fee_opt {
                                    let base_gwei = (base_fee as f64) / 1e9;
                                    lines.push(format!(
                                        "base fee @{} = {:.9} gwei, header_gas_limit = {}",
                                        sim_block, base_gwei, gas_limit_hdr
                                    ));
                                    if let Ok(prev_bf) =
                                        rqp.get_base_fee_at_block(sim_block.saturating_sub(1))
                                    {
                                        let prev_gwei = (prev_bf as f64) / 1e9;
                                        lines.push(format!(
                                            "base fee @{} = {:.9} gwei",
                                            sim_block.saturating_sub(1),
                                            prev_gwei
                                        ));
                                    }
                                }

                                // Compute required upfront cost using the params we will pass
                                let gas_for_check: U256 = if let Some(g) = unsigned_tx.gas {
                                    U256::from(g)
                                } else {
                                    U256::from(gas_limit_hdr)
                                };
                                let value: U256 = unsigned_tx.value.unwrap_or(U256::ZERO);

                                // Determine fee model
                                if let Some(max_fee) = unsigned_tx.max_fee_per_gas {
                                    let required = value.saturating_add(
                                        gas_for_check.saturating_mul(U256::from(max_fee)),
                                    );
                                    let req_eth = (required.to::<u128>() as f64) / 1e18;
                                    let tip = unsigned_tx.max_priority_fee_per_gas.unwrap_or(0);
                                    lines.push(format!("EIP-1559 caps: max_fee={} gwei, tip={} gwei, gas={} (fallback used if None)", (max_fee as f64)/1e9, (tip as f64)/1e9, gas_for_check.to_string()));
                                    lines.push(format!(
                                        "upfront requirement ~= value + gas*max_fee = {:.9} ETH",
                                        req_eth
                                    ));
                                } else if let Some(gp) = unsigned_tx.gas_price {
                                    let required = value.saturating_add(
                                        gas_for_check.saturating_mul(U256::from(gp)),
                                    );
                                    let req_eth = (required.to::<u128>() as f64) / 1e18;
                                    lines.push(format!(
                                        "legacy gas_price={} gwei, gas={} (fallback used if None)",
                                        (gp as f64) / 1e9,
                                        gas_for_check.to_string()
                                    ));
                                    lines.push(format!(
                                        "upfront requirement ~= value + gas*gas_price = {:.9} ETH",
                                        req_eth
                                    ));
                                } else {
                                    lines.push("no fee fields on unsigned tx; defaults may inflate upfront requirement".to_string());
                                }
                            }

                            let line = lines.join(" | ");
                            tracing::warn!(target: "sim", "insufficient funds diagnostics: {}", line);
                            debug_info = Some(line);
                        }
                    }
                }
                if let Some(hash) = tx_hash {
                    error!(
                        "Liquidity removal processing failed for tx {}: {}",
                        hash, msg
                    );
                } else {
                    error!("Liquidity removal processing failed: {}", msg);
                }
                return Ok(LiquidityRemovalResult {
                    success: false,
                    revert_reason: Some(msg),
                    address_balance_changes: HashMap::new(),
                    pool_address: None,
                    pool_identifier: None,
                    pool_type: None,
                    token_address: None,
                    function_name: None,
                    metrics_known: false,
                    eth_removed: 0.0,
                    drain_percentage: 0.0,
                    remaining_eth: 0.0,
                    is_scam: false,
                    debug_info,
                });
            }
        };

        // 2) Convert balance deltas to canonical map
        let address_balance_changes = Self::convert_processed_changes(&processed);

        // 3) Compute drain from deltas (deterministic, cache-backed pools only)
        let drain = self.compute_pool_drain(&address_balance_changes).await;
        let protocol_removal = self.protocol_removal_from_processed(&processed).await;

        let (
            pool_address,
            pool_identifier,
            pool_type,
            token_address,
            function_name,
            eth_removed,
            drain_percentage,
            remaining_eth,
            metrics_known,
        ) = if let Some(d) = drain {
            let pool_identifier = Some(d.pool_identifier);
            let pool_type = Some(d.pool_type);
            (
                Some(d.pool_address),
                pool_identifier,
                pool_type,
                d.token_address,
                Some("remove_liquidity".to_string()),
                d.eth_removed,
                d.percentage,
                d.remaining_eth,
                true,
            )
        } else if let Some(candidate) = protocol_removal {
            (
                candidate.pool_address,
                Some(candidate.pool_identifier),
                Some(candidate.pool_type),
                Some(candidate.token_address),
                Some(candidate.function_name.to_string()),
                0.0,
                0.0,
                0.0,
                false,
            )
        } else {
            (None, None, None, None, None, 0.0, 0.0, 0.0, false)
        };

        let is_scam = metrics_known
            && pool_address.is_some()
            && (drain_percentage > 60.0 || remaining_eth < 0.3);
        let success = processed.status;
        let revert_reason = if success {
            None
        } else {
            Some("Reverted in simulation".to_string())
        };

        Ok(LiquidityRemovalResult {
            success,
            revert_reason,
            address_balance_changes,
            pool_address,
            pool_identifier,
            pool_type,
            token_address,
            function_name,
            metrics_known,
            eth_removed,
            drain_percentage,
            remaining_eth,
            is_scam,
            debug_info: None,
        })
    }

    fn convert_processed_changes(
        processed: &ProcessedTransaction,
    ) -> HashMap<Address, AddressBalanceChange> {
        processed.address_balance_changes.clone()
    }

    async fn compute_pool_drain(
        &self,
        address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    ) -> Option<PoolDrainInfo> {
        // Track the best (most negative) ETH delta among known pools
        let mut best: Option<PoolDrainInfo> = None;

        for (address, change) in address_balance_changes {
            // ETH delta in ETH units (negative for drains)
            let eth_change = change
                .currency_net
                .get("ETH")
                .copied()
                .map(|s| s.to_string().parse::<f64>().unwrap_or(0.0) / 1e18)
                .unwrap_or(0.0);
            if eth_change >= -0.01 {
                continue;
            } // ignore tiny or positive

            // Only consider addresses that are tracked pools in the token cache
            if let Some(ref cache) = self.token_cache {
                let addr_str = to_checksum_address(address);
                if let Some(pool_state) = cache.get_pool(&addr_str).await {
                    let initial = pool_state.eth_reserve;
                    if initial > 0.0 {
                        let removed = eth_change.abs();
                        let remaining = (initial + eth_change).max(0.0);
                        let pct = ((initial - remaining) / initial * 100.0).min(100.0);
                        let candidate = PoolDrainInfo {
                            pool_address: *address,
                            pool_identifier: pool_state.address.clone(),
                            pool_type: pool_type_label(&pool_state.pool_type).to_string(),
                            token_address: parse_address(&pool_state.token_address),
                            eth_removed: removed,
                            remaining_eth: remaining,
                            percentage: pct,
                        };
                        // Keep the one with largest removal
                        if let Some(cur) = &best {
                            if candidate.eth_removed > cur.eth_removed {
                                best = Some(candidate);
                            }
                        } else {
                            best = Some(candidate);
                        }
                    }
                }
            }
        }
        best
    }

    async fn protocol_removal_from_processed(
        &self,
        processed: &ProcessedTransaction,
    ) -> Option<ProtocolRemovalInfo> {
        let cache = self.token_cache.as_ref()?;

        for burn in &processed.uniswap_v3_burns {
            if burn.amount.is_zero() {
                continue;
            }
            let pool_identifier = to_checksum_address(&burn.pool_address);
            let Some(pool_state) = cache.get_pool(&pool_identifier).await else {
                continue;
            };
            if !matches!(pool_state.pool_type, CachePoolType::UniswapV3) {
                continue;
            }
            let Some(token_address) = parse_address(&pool_state.token_address) else {
                continue;
            };
            return Some(ProtocolRemovalInfo {
                pool_address: Some(burn.pool_address),
                pool_identifier: pool_state.address.clone(),
                pool_type: pool_type_label(&pool_state.pool_type).to_string(),
                token_address,
                function_name: "decreaseLiquidity",
            });
        }

        for decrease in &processed.uniswap_v3_decreases {
            if decrease.liquidity.is_zero() {
                continue;
            }
            let pool_identifier = to_checksum_address(&decrease.pool_address);
            let Some(pool_state) = cache.get_pool(&pool_identifier).await else {
                continue;
            };
            if !matches!(pool_state.pool_type, CachePoolType::UniswapV3) {
                continue;
            }
            let Some(token_address) = parse_address(&pool_state.token_address) else {
                continue;
            };
            return Some(ProtocolRemovalInfo {
                pool_address: Some(decrease.pool_address),
                pool_identifier: pool_state.address.clone(),
                pool_type: pool_type_label(&pool_state.pool_type).to_string(),
                token_address,
                function_name: "decreaseLiquidity",
            });
        }

        for modify in &processed.uniswap_v4_modifies {
            if modify.liquidity_delta >= 0 {
                continue;
            }
            let pool_identifier =
                v4_event_display_key(modify.pool_manager_address, modify.event_id);
            let Some(pool_state) = cache.get_pool(&pool_identifier).await else {
                continue;
            };
            if !matches!(pool_state.pool_type, CachePoolType::UniswapV4) {
                continue;
            }
            let Some(token_address) = parse_address(&pool_state.token_address) else {
                continue;
            };
            return Some(ProtocolRemovalInfo {
                pool_address: None,
                pool_identifier: pool_state.address.clone(),
                pool_type: pool_type_label(&pool_state.pool_type).to_string(),
                token_address,
                function_name: "modifyLiquidity",
            });
        }

        None
    }

    pub(crate) async fn process_with_optional_retry(
        &self,
        mut unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
        retry_on_missing_header: bool,
    ) -> Result<ProcessedTransaction> {
        const MAX_RETRIES: usize = 5;
        const RETRY_DELAY_MS: u64 = 150;

        let mut attempt = 0usize;
        let resolved_block = self.resolve_simulation_block_number(block_number).await?;
        let mut header_base_fee: Option<u128> = None;
        let mut adjusted_for_base_fee = false;

        loop {
            let result = Self::process_unsigned_tx_blocking(
                self.simulator.clone(),
                unsigned_tx.clone(),
                resolved_block,
            )
            .await;

            match result {
                Ok(processed) => return Ok(processed),
                Err(err) => {
                    if !adjusted_for_base_fee && Self::is_base_fee_error(&err) {
                        if header_base_fee.is_none() {
                            match self.resolve_base_fee(resolved_block).await {
                                Ok(fee) => header_base_fee = fee,
                                Err(load_err) => {
                                    warn!(
                                        "Failed to resolve base fee for block {} while repricing: {}",
                                        resolved_block,
                                        load_err
                                    );
                                }
                            }
                        }

                        if let Some(base_fee) = header_base_fee {
                            if Self::adjust_for_base_fee(&mut unsigned_tx, base_fee) {
                                adjusted_for_base_fee = true;
                                let base_fee_gwei = (base_fee as f64) / 1_000_000_000f64;
                                warn!(
                                    "Base fee {:.3} gwei exceeded tx caps; repricing for simulation",
                                    base_fee_gwei
                                );
                                continue;
                            }
                        } else {
                            warn!(
                                "Base fee validation error but no resolved base fee available; unable to reprice"
                            );
                        }
                    }

                    let is_missing_header = Self::is_missing_header_error(&err);
                    if retry_on_missing_header && is_missing_header && attempt < MAX_RETRIES {
                        attempt += 1;
                        warn!(
                            "Header not yet available for block during liquidity removal simulation (attempt {}/{})",
                            attempt,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }

    async fn resolve_base_fee(&self, block_number: u64) -> Result<Option<u128>> {
        let header = self
            .simulator
            .block_context_loader()
            .load_block_header(block_number, None)
            .await?;
        Ok(header.header().base_fee_per_gas.map(|fee| fee as u128))
    }

    async fn resolve_simulation_block_number(&self, block_number: Option<u64>) -> Result<u64> {
        match block_number {
            Some(number) => Ok(number),
            None => self.live_tx_simulator.latest_state_block_number().await,
        }
    }

    fn is_missing_header_error(err: &eyre::Report) -> bool {
        let message = err.to_string();
        message.contains("Provider did not return header for block")
            || message.contains("No header for block")
            || message.contains("missing live block header")
    }

    fn is_base_fee_error(err: &eyre::Report) -> bool {
        let message = err.to_string();
        if message.contains("GasPriceLessThanBasefee") {
            return true;
        }
        message.to_lowercase().contains("base fee")
    }

    fn adjust_for_base_fee(unsigned_tx: &mut UnsignedTransaction, base_fee: u128) -> bool {
        const MIN_PRIORITY_FEE: u128 = 1_000_000_000; // 1 gwei
        const DEFAULT_PRIORITY_FEE: u128 = 2_000_000_000; // 2 gwei

        if base_fee == 0 {
            return false;
        }

        if let Some(mut max_fee) = unsigned_tx.max_fee_per_gas {
            let mut priority = unsigned_tx
                .max_priority_fee_per_gas
                .unwrap_or(DEFAULT_PRIORITY_FEE)
                .max(MIN_PRIORITY_FEE);

            let required_max_fee = base_fee.saturating_add(priority);
            let mut changed = false;
            if max_fee < required_max_fee {
                max_fee = required_max_fee;
                changed = true;
            }

            let max_priority_allowed = max_fee.saturating_sub(base_fee).max(MIN_PRIORITY_FEE);
            let adjusted_priority = priority.min(max_priority_allowed);
            if unsigned_tx
                .max_priority_fee_per_gas
                .map(|existing| existing != adjusted_priority)
                .unwrap_or(true)
            {
                priority = adjusted_priority;
                changed = true;
            }

            if changed {
                unsigned_tx.max_fee_per_gas = Some(max_fee);
                unsigned_tx.max_priority_fee_per_gas = Some(priority);
            }

            return changed;
        } else {
            let required_price = base_fee.saturating_add(DEFAULT_PRIORITY_FEE);
            let current_price = unsigned_tx.gas_price.unwrap_or(0);
            if current_price >= required_price {
                return false;
            }
            unsigned_tx.gas_price = Some(required_price);
            return true;
        }
    }

    async fn process_unsigned_tx_blocking(
        simulator: Arc<TxSimulator>,
        unsigned_tx: UnsignedTransaction,
        resolved_block: u64,
    ) -> Result<ProcessedTransaction> {
        let (tx, rx) = oneshot::channel();
        let req = SimRequest {
            simulator,
            unsigned_tx,
            resolved_block,
            responder: tx,
        };

        // Send to dedicated simulation runtime
        if let Err(e) = SIM_WORKER.sender.send(req) {
            return Err(eyre!("simulation worker channel closed: {}", e));
        }

        rx.await
            .map_err(|e| eyre!("simulation worker dropped response: {}", e))?
    }

    fn parse_nonce_mismatch(err: &eyre::Report) -> Option<u64> {
        let message = err.to_string();
        if !message.contains("nonce") || !message.contains("expected") {
            return None;
        }

        // Extract the substring after "expected " and parse consecutive digits
        let expected_part = message.split("expected ").nth(1)?;
        let expected_str = expected_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if expected_str.is_empty() {
            return None;
        }

        expected_str.parse::<u64>().ok()
    }
}

fn parse_address(value: &str) -> Option<Address> {
    value.trim_start_matches("0x").parse::<Address>().ok()
}

fn pool_type_label(pool_type: &CachePoolType) -> &'static str {
    match pool_type {
        CachePoolType::UniswapV2 => "UNISWAP-V2",
        CachePoolType::UniswapV3 => "UNISWAP-V3",
        CachePoolType::UniswapV4 => "UNISWAP-V4",
        CachePoolType::Unknown => "UNKNOWN",
    }
}

fn v4_event_display_key(pool_manager: Address, pool_id: alloy_primitives::B256) -> String {
    format!(
        "{}#{:#x}",
        to_checksum_address(&pool_manager).to_ascii_lowercase(),
        pool_id
    )
}
