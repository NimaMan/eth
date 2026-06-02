//! Investigate the custody-risk balance-drain calculation against the Session
//! case (token 0xc3A640…68cDff, vault drained at block 25202411).
//!
//! Pipeline (same token-builder pattern as `token_tracking_range`):
//!   BlockProcessor -> BlockTokenProcessor -> registry token
//!   -> HolderBalanceLedger (from Transfer events / "transfer data")
//!   -> reconcile vs on-chain balanceOf (state) -> custody drain victims
//!
//! It prints, per block in the range, the vault's ledger-expected balance vs its
//! actual on-chain balance, demonstrating the event-less drain: the ledger keeps
//! showing ~9.87M while state drops to 93 at the drain block.
//!
//! Run:
//!   cargo run -p eth_token --example custody_session_balance_drain
//!
//! For a fast holder-label verification:
//!   CUSTODY_TRACE_MODE=drain CUSTODY_END_BLOCK=25202411 cargo run -p eth_token --example custody_session_balance_drain
//!
//! For buyer-victim counts over the token window:
//!   CUSTODY_TRACE_MODE=drain CUSTODY_END_BLOCK=25202464 cargo run -p eth_token --example custody_session_balance_drain

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use eth_token::chain_metadata::RethChainMetadataProvider;
use eth_token::custody::{
    reconcile_custody_drain, CustodyDrainVictim, CustodyState, HolderBalanceLedger,
    HolderBalanceReader, ReconcileConfig,
};
use eth_token::tracking::BlockTokenProcessor;
use eyre::{bail, Result};
use reth_chain_query::common_addresses::is_burn_address_str;
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, PoolBuySellSimulator};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_TOKEN: &str = "0xc3A640bD249381F8097f44C1b61C46172068cDff";
const DEFAULT_HOLDER: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const DEFAULT_START_BLOCK: u64 = 25_202_408;
const DEFAULT_END_BLOCK: u64 = 25_202_464;
const DEFAULT_DRAIN_BLOCK: u64 = 25_202_411;
const HISTORY_LIMIT: usize = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TraceMode {
    All,
    Drain,
    None,
}

impl TraceMode {
    fn from_env() -> Result<Self> {
        match std::env::var("CUSTODY_TRACE_MODE")
            .unwrap_or_else(|_| "all".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "all" => Ok(Self::All),
            "drain" | "drain_block" => Ok(Self::Drain),
            "none" => Ok(Self::None),
            other => bail!("unsupported CUSTODY_TRACE_MODE={other}; expected all, drain, or none"),
        }
    }

    fn include_traces(self, block_number: u64, drain_block: u64) -> bool {
        match self {
            Self::All => true,
            Self::Drain => block_number == drain_block,
            Self::None => false,
        }
    }
}

/// Reads on-chain `balanceOf` and scales to token decimals so it matches the
/// ledger's scaled units.
struct ProviderBalanceReader {
    provider: Arc<RethQueryProvider>,
    token: alloy_primitives::Address,
    decimals: u8,
}

#[async_trait]
impl HolderBalanceReader for ProviderBalanceReader {
    async fn balance_of(&self, holder: &str, block: u64) -> Result<f64> {
        let holder = holder.trim().parse::<alloy_primitives::Address>()?;
        let raw = self
            .provider
            .get_token_balance(self.token, holder, Some(block))
            .await?;
        Ok(scale(raw, self.decimals))
    }
}

fn scale(raw: alloy_primitives::U256, decimals: u8) -> f64 {
    let divisor = 10f64.powi(decimals as i32);
    // U256 -> f64 via string is exact enough for display/reconciliation scale.
    raw.to_string().parse::<f64>().unwrap_or(0.0) / divisor
}

#[tokio::main]
async fn main() -> Result<()> {
    let datadir =
        std::env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());
    let token_address =
        std::env::var("CUSTODY_TOKEN").unwrap_or_else(|_| DEFAULT_TOKEN.to_string());
    let holder = std::env::var("CUSTODY_HOLDER").unwrap_or_else(|_| DEFAULT_HOLDER.to_string());
    let start_block = env_u64("CUSTODY_START_BLOCK", DEFAULT_START_BLOCK);
    let end_block = env_u64("CUSTODY_END_BLOCK", DEFAULT_END_BLOCK);
    let drain_block = env_u64("CUSTODY_DRAIN_BLOCK", DEFAULT_DRAIN_BLOCK);
    let trace_mode = TraceMode::from_env()?;
    if start_block > end_block {
        bail!("CUSTODY_START_BLOCK must be <= CUSTODY_END_BLOCK");
    }
    if drain_block < start_block || drain_block > end_block {
        bail!("CUSTODY_DRAIN_BLOCK must be inside the replay range");
    }

    let provider = Arc::new(RethQueryProvider::new(&datadir)?);
    let tx_processor = BlockProcessor::new(provider.clone());
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let mut token_processor = BlockTokenProcessor::new(HISTORY_LIMIT);

    let token_addr = token_address.parse::<alloy_primitives::Address>()?;

    println!("Custody balance-drain investigation");
    println!("token: {token_address}");
    println!("holder: {holder}");
    println!("range: {start_block}..={end_block}  drain_block={drain_block}");
    println!("trace_mode: {trace_mode:?}");
    println!();

    // include_traces=true so each ProcessedTransaction carries
    // `internal_erc20_calls` — the token builder depends only on the
    // ProcessedTransaction, and the holder-balance-drain detector reads the
    // internal transferFrom from that field (the drain emits no Transfer event).
    for block_number in start_block..=end_block {
        let include_traces = trace_mode.include_traces(block_number, drain_block);
        let block = tx_processor
            .process_block_with_options(block_number, include_traces)
            .await?;
        token_processor
            .process_block_with_discovery_provider(&block, &discovery_provider, &pool_simulator)
            .await;
    }

    let token = token_processor
        .registry
        .tokens
        .values()
        .find(|token| token.contract_address.eq_ignore_ascii_case(&token_address))
        .cloned();
    let Some(mut token) = token else {
        bail!("token {token_address} not tracked in range {start_block}..={end_block}");
    };

    let decimals = token.decimals;
    println!("token decimals: {decimals}");
    println!(
        "recorded transfer buckets: {}",
        token.transfer_tracker.erc20_transfers.len()
    );
    println!();

    // --- What the token tracker currently captures (labels / PnL / amounts) ---
    println!("=== token-level labels ===");
    println!("  is_scam:            {}", token.is_scam());
    println!("  scam_mechanism:     {:?}", token.scam_mechanism());
    println!("  scam_label:         {:?}", token.scam_label());
    println!("  total_supply:       {:?}", token.total_supply_scaled());
    println!();
    println!("=== custody findings (who lost tokens) ===");
    if token.custody_findings().is_empty() {
        println!("  (none)");
    }
    for finding in token.custody_findings() {
        println!(
            "  capability={} state={} block={:?}",
            finding.capability.as_str(),
            finding.state.as_str(),
            finding.block_number,
        );
        println!(
            "    victim={} to={} amount_scaled={} caller={}",
            finding
                .evidence
                .get("victim")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            finding
                .evidence
                .get("to_address")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            finding
                .evidence
                .get("amount_scaled")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            finding
                .evidence
                .get("caller")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
        );
    }
    println!(
        "  holder custody_drained_amount: {}",
        token.custody_drained_amount(&holder)
    );
    println!(
        "  distinct victims: {}",
        token
            .custody_findings()
            .iter()
            .filter_map(|f| f.evidence.get("victim").and_then(|v| v.as_str()))
            .collect::<std::collections::HashSet<_>>()
            .len()
    );
    verify_holder_confiscation_label(&token, &holder)?;
    println!();
    println!("=== pool labels ===");
    for pool in token.all_pool_bases() {
        println!("  pool {}", pool.identity.pool_address);
        println!(
            "    can_buy={} can_sell={} has_liquidity_removal={} is_scam_reserve={}",
            pool.state.can_buy,
            pool.state.can_sell,
            pool.has_liquidity_removal(),
            pool.reserve_tracker.is_scam,
        );
        println!(
            "    scam_mechanism={:?} scam_label={:?} scam_block={:?}",
            pool.scam_mechanism, pool.scam_label, pool.scam_block,
        );
    }
    println!();
    println!("=== token-builder pool state flags ===");
    for flags in token.current_pool_state_flags().values() {
        println!("  pool {}", flags.pool_address);
        println!("    labels={:?}", flags.labels);
        println!(
            "    custody_realized={} terminal_position_risk={} holder_balance_backdoor_drain={} reserve_liquidity_removed={}",
            flags.custody.realized,
            flags.risk.terminal_position_risk,
            flags.risk.holder_balance_backdoor_drain,
            flags.liquidity.reserve_liquidity_removed,
        );
    }
    println!();

    // --- internal_erc20_calls now captured by tx_processor across the range ---
    // (the drain transferFrom is here even though it emitted no Transfer event)
    println!("=== internal ERC-20 calls observed in range (transfer/transferFrom/approve) ===");
    {
        use tx_processor::Erc20CallKind;
        let mut shown = 0usize;
        // Trace processing is slow; scan a tight window around the drain block.
        // The drain transferFrom is an internal call, only visible in the trace.
        for block_number in drain_block.saturating_sub(1)..=drain_block + 1 {
            let block = tx_processor
                .process_block_with_options(block_number, true)
                .await?;
            for btx in &block.transactions {
                for call in &btx.processed.internal_erc20_calls {
                    if call.token_address != token_addr {
                        continue;
                    }
                    let matched = btx.processed.erc20_transfers.iter().any(|t| {
                        t.token_address == call.token_address
                            && t.from_address == call.from_address
                            && t.to_address == call.to_address
                            && t.amount == call.amount
                    });
                    let flag = match call.kind {
                        Erc20CallKind::Approve => "approve",
                        _ if matched => "matched-transfer",
                        _ => "NO-TRANSFER-EVENT (event-less move)",
                    };
                    println!(
                        "  block {} {} caller={} from={} to={} amount={} [{}]",
                        block_number,
                        call.kind.as_str(),
                        call.caller,
                        call.from_address,
                        call.to_address,
                        call.amount,
                        flag,
                    );
                    shown += 1;
                }
            }
        }
        if shown == 0 {
            println!("  (none)");
        }
    }
    println!();

    let reader = ProviderBalanceReader {
        provider: provider.clone(),
        token: token_addr,
        decimals,
    };

    // Per-block: ledger-expected vault balance (transfer data) vs on-chain state.
    println!(
        "{:>10}  {:>22}  {:>22}",
        "block", "ledger_expected_vault", "onchain_actual_vault"
    );
    for block_number in (start_block..=end_block).step_by(1) {
        let ledger = HolderBalanceLedger::from_token_through_block(&token, block_number);
        let expected = ledger.expected_balance(&holder);
        if expected <= 0.0 && block_number < drain_block {
            continue;
        }
        let actual = reader.balance_of(&holder, block_number).await?;
        let mark = if block_number == drain_block {
            "  <- drain"
        } else {
            ""
        };
        println!("{block_number:>10}  {expected:>22.9}  {actual:>22.9}{mark}");
    }
    println!();

    // Reconcile at the drain block over all material holders.
    let ledger = HolderBalanceLedger::from_token_through_block(&token, drain_block);
    let config = ReconcileConfig {
        min_expected_balance: 1.0,
        min_drop_fraction: 0.9,
    };
    let victims = reconcile_custody_drain(&ledger, &reader, drain_block, &config).await?;

    println!(
        "custody drain victims at block {drain_block}: {}",
        victims.len()
    );
    for victim in &victims {
        println!(
            "  holder={} expected={:.9} actual={:.9} missing={:.9} drained={:.2}% supply_share={:.4}%",
            victim.holder,
            victim.expected_balance,
            victim.actual_balance,
            victim.missing_balance,
            victim.drained_fraction * 100.0,
            victim.expected_supply_share * 100.0,
        );
    }
    println!();
    audit_buyer_confiscations(&mut token, &reader, end_block, &config).await?;

    Ok(())
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn verify_holder_confiscation_label(
    token: &eth_token::erc20::ERC20Token,
    holder: &str,
) -> Result<()> {
    let holder_findings: Vec<_> = token
        .custody_findings()
        .iter()
        .filter(|finding| finding.state == CustodyState::Realized)
        .filter(|finding| {
            finding
                .evidence
                .get("victim")
                .and_then(|value| value.as_str())
                .map(|victim| same_address(victim, holder))
                .unwrap_or(false)
        })
        .collect();
    let drained_amount = token.custody_drained_amount(holder);
    let pool_flags: Vec<_> = token.current_pool_state_flags().into_values().collect();
    let has_terminal_pool_flag = pool_flags.iter().any(|flags| {
        flags.custody.realized
            && flags.risk.terminal_position_risk
            && flags.risk.holder_balance_backdoor_drain
            && !flags.liquidity.reserve_liquidity_removed
    });

    println!("=== token-builder holder-confiscation verdict ===");
    println!("  holder: {holder}");
    println!(
        "  realized custody findings for holder: {}",
        holder_findings.len()
    );
    println!("  custody_drained_amount(holder): {drained_amount}");
    println!("  terminal holder-drain pool flag: {has_terminal_pool_flag}");

    if holder_findings.is_empty() {
        bail!("token builder did not label holder {holder} as a realized custody victim");
    }
    if drained_amount <= 0.0 {
        bail!("token builder did not record a positive drained amount for holder {holder}");
    }
    if !has_terminal_pool_flag {
        bail!("token builder did not project the holder drain into terminal pool flags");
    }

    Ok(())
}

fn same_address(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

#[derive(Clone, Debug)]
struct BuyerAuditRow {
    holder: String,
    bought_amount: f64,
    normal_out_to_pool: f64,
    normal_out_to_other: f64,
    normal_out_to_burn: f64,
    expected_balance: f64,
    actual_balance: f64,
    missing_balance: f64,
    drained_fraction: f64,
}

async fn audit_buyer_confiscations(
    token: &mut eth_token::erc20::ERC20Token,
    reader: &ProviderBalanceReader,
    audit_block: u64,
    config: &ReconcileConfig,
) -> Result<()> {
    let pool_addresses = token
        .all_pool_bases()
        .into_iter()
        .map(|pool| normalize(&pool.identity.pool_address))
        .collect::<BTreeSet<_>>();
    let token_address = normalize(&token.contract_address);
    let mut bought_by_holder = BTreeMap::<String, f64>::new();
    let mut normal_out_to_pool = BTreeMap::<String, f64>::new();
    let mut normal_out_to_other = BTreeMap::<String, f64>::new();
    let mut normal_out_to_burn = BTreeMap::<String, f64>::new();

    for records in token.transfer_tracker.erc20_transfers.values() {
        for record in records
            .iter()
            .filter(|record| record.block_number <= audit_block)
        {
            if normalize(&record.token_address) != token_address {
                continue;
            }
            let from = normalize(&record.from_address);
            let to = normalize(&record.to_address);
            if !pool_addresses.contains(&from)
                || pool_addresses.contains(&to)
                || is_burn_address_str(&to)
                || to == token_address
            {
                continue;
            }
            *bought_by_holder.entry(to).or_default() += record.amount;
        }
    }
    for records in token.transfer_tracker.erc20_transfers.values() {
        for record in records
            .iter()
            .filter(|record| record.block_number <= audit_block)
        {
            if normalize(&record.token_address) != token_address {
                continue;
            }
            let from = normalize(&record.from_address);
            if !bought_by_holder.contains_key(&from) {
                continue;
            }
            let to = normalize(&record.to_address);
            if pool_addresses.contains(&to) {
                *normal_out_to_pool.entry(from).or_default() += record.amount;
            } else if is_burn_address_str(&to) {
                *normal_out_to_burn.entry(from).or_default() += record.amount;
            } else {
                *normal_out_to_other.entry(from).or_default() += record.amount;
            }
        }
    }

    let ledger = HolderBalanceLedger::from_token_through_block(token, audit_block);
    let mut no_expected_position = Vec::<BuyerAuditRow>::new();
    let mut intact_or_partial = Vec::<BuyerAuditRow>::new();
    let mut confiscated = Vec::<BuyerAuditRow>::new();
    let mut reconciled_buyer_victims = Vec::<CustodyDrainVictim>::new();
    let mut reconciliation_contexts = HashMap::<String, serde_json::Value>::new();

    for (holder, bought_amount) in &bought_by_holder {
        let expected = ledger.expected_balance(holder);
        let row_base = BuyerAuditRow {
            holder: holder.clone(),
            bought_amount: *bought_amount,
            normal_out_to_pool: normal_out_to_pool.get(holder).copied().unwrap_or(0.0),
            normal_out_to_other: normal_out_to_other.get(holder).copied().unwrap_or(0.0),
            normal_out_to_burn: normal_out_to_burn.get(holder).copied().unwrap_or(0.0),
            expected_balance: expected,
            actual_balance: 0.0,
            missing_balance: 0.0,
            drained_fraction: 0.0,
        };
        if expected < config.min_expected_balance {
            no_expected_position.push(row_base);
            continue;
        }
        let actual = reader.balance_of(holder, audit_block).await?;
        let missing = (expected - actual).max(0.0);
        let drained_fraction = if expected > 0.0 {
            (missing / expected).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let row = BuyerAuditRow {
            expected_balance: expected,
            actual_balance: actual,
            missing_balance: missing,
            drained_fraction,
            ..row_base
        };
        if actual <= expected * (1.0 - config.min_drop_fraction) {
            reconciled_buyer_victims.push(CustodyDrainVictim {
                holder: holder.clone(),
                block_number: audit_block,
                expected_balance: expected,
                actual_balance: actual,
                missing_balance: missing,
                drained_fraction,
                expected_supply_share: ledger.share_of_supply(holder),
            });
            reconciliation_contexts.insert(
                holder.clone(),
                serde_json::json!({
                    "candidate_reason": "pool_out_buyer",
                    "bought_amount": row.bought_amount,
                    "normal_out_to_pool": row.normal_out_to_pool,
                    "normal_out_to_other": row.normal_out_to_other,
                    "normal_out_to_burn": row.normal_out_to_burn,
                    "audit_block": audit_block,
                }),
            );
            confiscated.push(row);
        } else {
            intact_or_partial.push(row);
        }
    }

    for row in &mut no_expected_position {
        row.actual_balance = reader.balance_of(&row.holder, audit_block).await?;
    }
    no_expected_position.sort_by(|left, right| {
        right
            .bought_amount
            .partial_cmp(&left.bought_amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    confiscated.sort_by(|left, right| {
        right
            .missing_balance
            .partial_cmp(&left.missing_balance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    intact_or_partial.sort_by(|left, right| {
        right
            .expected_balance
            .partial_cmp(&left.expected_balance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("=== buyer confiscation audit at block {audit_block} ===");
    println!("  unique pool-out buyers: {}", bought_by_holder.len());
    println!(
        "  buyers with expected balance >= {}: {}",
        config.min_expected_balance,
        bought_by_holder
            .len()
            .saturating_sub(no_expected_position.len())
    );
    println!(
        "  buyers with no expected position: {}",
        no_expected_position.len()
    );
    println!("  confiscated buyers: {}", confiscated.len());
    println!("  intact/partial buyers: {}", intact_or_partial.len());
    let added = token.record_reconciled_holder_confiscations(
        &reconciled_buyer_victims,
        Some(&reconciliation_contexts),
    );
    println!("  reconciled custody findings persisted: {added}");
    println!(
        "  total persisted custody findings after reconciliation: {}",
        token.custody_findings().len()
    );
    println!(
        "  scam mechanism after reconciliation: {:?}",
        token.scam_mechanism()
    );
    if !no_expected_position.is_empty() {
        println!("  no expected position rows:");
        for row in &no_expected_position {
            println!(
                "    holder={} bought={:.9} out_to_pool={:.9} out_to_other={:.9} out_to_burn={:.9} expected={:.9} actual={:.9}",
                row.holder,
                row.bought_amount,
                row.normal_out_to_pool,
                row.normal_out_to_other,
                row.normal_out_to_burn,
                row.expected_balance,
                row.actual_balance,
            );
        }
    }
    if !confiscated.is_empty() {
        println!("  confiscated buyer rows:");
        for row in confiscated.iter().take(50) {
            println!(
                "    holder={} bought={:.9} out_to_pool={:.9} out_to_other={:.9} out_to_burn={:.9} expected={:.9} actual={:.9} missing={:.9} drained={:.2}%",
                row.holder,
                row.bought_amount,
                row.normal_out_to_pool,
                row.normal_out_to_other,
                row.normal_out_to_burn,
                row.expected_balance,
                row.actual_balance,
                row.missing_balance,
                row.drained_fraction * 100.0,
            );
        }
    }
    if !intact_or_partial.is_empty() {
        println!("  intact/partial buyer rows:");
        for row in intact_or_partial.iter().take(50) {
            println!(
                "    holder={} bought={:.9} out_to_pool={:.9} out_to_other={:.9} out_to_burn={:.9} expected={:.9} actual={:.9} missing={:.9} drained={:.2}%",
                row.holder,
                row.bought_amount,
                row.normal_out_to_pool,
                row.normal_out_to_other,
                row.normal_out_to_burn,
                row.expected_balance,
                row.actual_balance,
                row.missing_balance,
                row.drained_fraction * 100.0,
            );
        }
    }

    Ok(())
}

fn normalize(address: &str) -> String {
    address.trim().to_ascii_lowercase()
}
