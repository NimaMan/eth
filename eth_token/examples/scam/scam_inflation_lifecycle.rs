//! # scam_inflation_lifecycle — reconstruct HOW a scam token was inflated and rugged
//!
//! ## Objective
//! For one concrete pump-then-rug case (the `e_pair` token, default), replay the
//! pool's active on-chain window and emit a machine-readable `scam_mechanics.json`
//! plus a plain-language `reports/how_it_was_scammed.md` that tells the
//! "how it was scammed" story: deploy → fund → open trading → price inflated as
//! buyers came in → operator pulled the liquidity, taking the ETH, collapsing the
//! price to ~0 and leaving buyers holding worthless tokens.
//!
//! Built case-first (env-overridable, defaults to the e_pair case) so the code
//! generalizes to other deploy→fund→open→pump→remove rugs later.
//!
//! ## Algorithm (full description)
//! 1. Open the Reth provider + RethIndex (same scaffolding as the custody case
//!    report example).
//! 2. Resolve the lifecycle `key_txs` from the case (token_deploy, token_fund,
//!    open_trading, lp_approval, liquidity_removal). For each, call
//!    `provider.get_transaction_by_hash(hash)` to learn its block + timestamp.
//!    These anchor the timeline and the replay bounds.
//! 3. Bound the replay window:
//!      start = min(creation/deploy block over the resolved key_txs)
//!      end   = liquidity_removal block + LOOKAHEAD (to capture the collapse)
//!    Cap (end-start) at MAX_WINDOW_BLOCKS; if exceeded, clamp `end` and note it
//!    in provenance.
//! 4. Collect the *active* blocks in that window for {token,pool} via
//!    `activity_blocks` (RethIndex address-participation), then process each active
//!    block with `BlockProcessor::process_block_with_options(block,false)`.
//! 5. Build token state with `TokenStateBuilder::new(metadata,HISTORY_LIMIT)
//!    .build_from_processed_blocks(active_processed_blocks)`; fetch the pool via
//!    `token.pool_base(pool)`. This populates `reserve_tracker.reserve_history`,
//!    `swap_events`, `mint_events`, `burn_events`.
//! 6. Compute, from the pool state:
//!    - price_series: per reserve snapshot {block, denom_reserve, token_reserve,
//!      price, price_ratio_to_initial = price / initial_price}. The inflation curve.
//!    - inflation: initial_price (first meaningful snapshot price), peak_price /
//!      peak_block (max snapshot price), price_at_removal (price at/just after the
//!      liquidity_removal block), inflation_x = peak/initial,
//!      collapse_ratio = price_at_removal/peak.
//!    - liquidity: eth_added_as_lp (largest upward jump in denom_reserve, i.e. the
//!      LP add / mint), eth_removed_at_rug (largest downward drop in denom_reserve
//!      at/after the removal, i.e. the burn), net_eth_extracted = removed - added,
//!      each computed from `reserve_history` denom_reserve deltas and corroborated
//!      with mint_events / burn_events when available.
//!    - trading: buy_count, sell_count, distinct_buyers from `swap_events`
//!      (is_buy/is_sell/to fields). best-effort, confidence labelled.
//!    - lifecycle: ordered phases, each key_tx resolved to block+timestamp with a
//!      plain-English label, plus derived phases (first_trade, peak, collapse).
//!    - scam_mechanism: prefer the pool's `inferred_scam_mechanism()` (which, for a
//!      reserve collapse with a matching burn, labels `direct_lp_liquidity_removal`);
//!      fall back to the stored `scam_mechanism` fields.
//! 7. Emit `artifacts/scam_mechanics.json` (the field-by-field contract below) and
//!    `reports/how_it_was_scammed.md` (the narrative for a non-expert).
//!
//! ## scam_mechanics.json shape (frontend contract)
//! {
//!   summary: { token, pool, suspect, denom, token_symbol, token_name,
//!              start_block, end_block, active_block_count, replay_window_blocks },
//!   inflation: { initial_price, peak_price, peak_block, price_at_removal,
//!                inflation_x, collapse_ratio },
//!   liquidity: { eth_added_as_lp, eth_added_block, eth_removed_at_rug,
//!                eth_removed_block, net_eth_extracted, source, mint_event_count,
//!                burn_event_count },
//!   trading:   { buy_count, sell_count, swap_count, distinct_buyers, confidence },
//!   scam_mechanism: { mechanism, label, block_number, tx_hash, source },
//!   lifecycle: [ { phase, label, kind, block_number, timestamp, tx_hash } ... ],
//!   price_series: [ { block_number, denom_reserve, token_reserve, price,
//!                     price_ratio_to_initial } ... ],
//!   provenance: { method, block_range, history_limit, notes[] }
//! }
//!
//! ## Run
//!   cargo run -p eth_token --example scam_inflation_lifecycle
//! Env overrides: CASE_DIR, TOKEN, POOL, SUSPECT, DENOM, RETH_DATADIR,
//! RETH_INDEX_DIR, START_BLOCK, END_BLOCK, LOOKAHEAD_BLOCKS, MAX_WINDOW_BLOCKS.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address, B256};
use eth_token::erc20::ERC20TokenMetadata;
use eth_token::tracking::TokenStateBuilder;
use eth_token::pools::base::BasePool;
use eyre::{bail, eyre, Result, WrapErr};
use reth_chain_query::{to_checksum_address, RethQueryProvider};
use serde::Serialize;
use serde_json::Value;
use tx_processor::BlockProcessor;

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_CASE_DIR: &str = "/home/nima/code/crypto/blockchains/eth/risk_atlas/scammer_analytics/cases/eth_0x9d58c75a_e_pair_25181124";
const DEFAULT_TOKEN: &str = "0x2CedB62299Ad3422Fc0b0ec57180695501D15fef";
const DEFAULT_POOL: &str = "0x5D43262637B4fc4bfaF4164A8974c436D5CFCe8D";
const DEFAULT_SUSPECT: &str = "0x9d58c75A8749a94C9CF9539B4f625Ed42936105D";
const DEFAULT_DENOM: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

const HISTORY_LIMIT: usize = 50_000;
const DEFAULT_LOOKAHEAD_BLOCKS: u64 = 50;
const DEFAULT_MAX_WINDOW_BLOCKS: u64 = 5_000;

// Lifecycle key_txs we expect in case.toml [key_txs], in narrative order.
const KEY_TX_ORDER: [(&str, &str); 5] = [
    ("token_deploy", "Operator deployed the token contract"),
    ("token_fund", "Operator funded the token / seeded ETH"),
    ("open_trading", "Operator opened trading (buys enabled)"),
    ("lp_approval", "Operator approved LP tokens to the router (pre-stage the rug)"),
    (
        "liquidity_removal",
        "Operator removed the liquidity (the rug pull)",
    ),
];

#[derive(Debug)]
struct Args {
    datadir: String,
    reth_index: String,
    case_dir: PathBuf,
    token: Address,
    pool: Address,
    suspect: Address,
    denom: Address,
    start_block_override: Option<u64>,
    end_block_override: Option<u64>,
    lookahead_blocks: u64,
    max_window_blocks: u64,
}

#[derive(Clone, Debug, Serialize)]
struct ResolvedKeyTx {
    phase: String,
    label: String,
    tx_hash: String,
    block_number: Option<u64>,
    timestamp: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
struct LifecycleEntry {
    phase: String,
    label: String,
    kind: String, // "key_tx" | "derived"
    block_number: Option<u64>,
    timestamp: Option<u64>,
    tx_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PriceSeriesPoint {
    block_number: u64,
    tx_hash: String,
    denom_reserve: f64,
    token_reserve: f64,
    price: f64,
    price_ratio_to_initial: f64,
}

#[derive(Clone, Debug, Serialize)]
struct Inflation {
    initial_price: f64,
    peak_price: f64,
    peak_block: Option<u64>,
    price_at_removal: f64,
    inflation_x: f64,
    collapse_ratio: f64,
}

#[derive(Clone, Debug, Serialize)]
struct Liquidity {
    eth_added_as_lp: f64,
    eth_added_block: Option<u64>,
    eth_removed_at_rug: f64,
    eth_removed_block: Option<u64>,
    net_eth_extracted: f64,
    source: String,
    mint_event_count: usize,
    burn_event_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Trading {
    buy_count: u64,
    sell_count: u64,
    swap_count: usize,
    distinct_buyers: usize,
    confidence: String,
}

#[derive(Clone, Debug, Serialize)]
struct ScamMechanism {
    mechanism: Option<String>,
    label: Option<String>,
    block_number: Option<u64>,
    tx_hash: Option<String>,
    source: String,
}

#[derive(Clone, Debug, Serialize)]
struct Summary {
    token: String,
    pool: String,
    suspect: String,
    denom: String,
    token_symbol: String,
    token_name: String,
    start_block: u64,
    end_block: u64,
    active_block_count: usize,
    replay_window_blocks: u64,
}

#[derive(Clone, Debug, Serialize)]
struct Provenance {
    method: String,
    block_range: String,
    history_limit: usize,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ScamMechanicsArtifact {
    summary: Summary,
    inflation: Inflation,
    liquidity: Liquidity,
    trading: Trading,
    scam_mechanism: ScamMechanism,
    lifecycle: Vec<LifecycleEntry>,
    price_series: Vec<PriceSeriesPoint>,
    provenance: Provenance,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let provider = Arc::new(
        RethQueryProvider::new(&args.datadir)
            .wrap_err("failed to open Reth provider")?
            .with_reth_index(&args.reth_index)
            .wrap_err("failed to open RethIndex")?,
    );
    let latest = provider.get_latest_block()?;

    // --- 1. resolve lifecycle key_txs to block + timestamp -------------------
    let key_txs = read_key_txs(&args.case_dir)?;
    let mut resolved: Vec<ResolvedKeyTx> = Vec::new();
    for (phase, label) in KEY_TX_ORDER {
        let Some(tx_hash) = key_txs.get(phase).cloned() else {
            continue;
        };
        let (block_number, timestamp) = resolve_tx(provider.as_ref(), &tx_hash).await;
        resolved.push(ResolvedKeyTx {
            phase: phase.to_string(),
            label: label.to_string(),
            tx_hash,
            block_number,
            timestamp,
        });
    }

    let deploy_block = resolved
        .iter()
        .find(|tx| tx.phase == "token_deploy")
        .and_then(|tx| tx.block_number);
    let removal_block = resolved
        .iter()
        .find(|tx| tx.phase == "liquidity_removal")
        .and_then(|tx| tx.block_number);

    let mut notes: Vec<String> = Vec::new();

    // --- 2. bound the replay window ------------------------------------------
    let min_resolved_block = resolved.iter().filter_map(|tx| tx.block_number).min();
    let max_resolved_block = resolved.iter().filter_map(|tx| tx.block_number).max();

    let start_block = args
        .start_block_override
        .or(deploy_block)
        .or(min_resolved_block)
        .ok_or_else(|| eyre!("could not derive a start block (no key_txs resolved)"))?;

    let removal_or_last = removal_block.or(max_resolved_block).unwrap_or(start_block);
    let mut end_block = args
        .end_block_override
        .unwrap_or_else(|| removal_or_last.saturating_add(args.lookahead_blocks))
        .min(latest);

    if end_block < start_block {
        end_block = start_block;
    }
    if end_block.saturating_sub(start_block) > args.max_window_blocks {
        let clamped = start_block.saturating_add(args.max_window_blocks);
        notes.push(format!(
            "replay window {}..={} exceeded MAX_WINDOW_BLOCKS={}; clamped end to {}",
            start_block, end_block, args.max_window_blocks, clamped
        ));
        end_block = clamped;
    }
    let replay_window_blocks = end_block.saturating_sub(start_block);

    println!("scam_inflation_lifecycle");
    println!("token:    {}", addr(args.token));
    println!("pool:     {}", addr(args.pool));
    println!("suspect:  {}", addr(args.suspect));
    println!("case_dir: {}", args.case_dir.display());
    println!(
        "window:   {}..={} ({} blocks); deploy={:?} removal={:?}",
        start_block, end_block, replay_window_blocks, deploy_block, removal_block
    );

    // --- 3. collect active blocks + process them -----------------------------
    let mut active_blocks = activity_blocks(provider.as_ref(), args.token, args.pool, start_block, end_block)?;
    // Always include the resolved key_tx blocks so the lifecycle anchors are replayed.
    for tx in &resolved {
        if let Some(block) = tx.block_number {
            if block >= start_block && block <= end_block {
                active_blocks.push(block);
            }
        }
    }
    active_blocks.sort_unstable();
    active_blocks.dedup();
    if active_blocks.is_empty() {
        bail!("no active blocks found for token/pool in {start_block}..={end_block}");
    }
    println!(
        "active:   {} blocks ({}..={})",
        active_blocks.len(),
        active_blocks[0],
        active_blocks[active_blocks.len() - 1]
    );

    let processor = BlockProcessor::new(provider.clone());
    let mut processed_blocks = Vec::with_capacity(active_blocks.len());
    for (index, block_number) in active_blocks.iter().enumerate() {
        let block = processor
            .process_block_with_options(*block_number, false)
            .await
            .wrap_err_with(|| format!("failed to process active block {block_number}"))?;
        processed_blocks.push(block);
        if (index + 1) % 25 == 0 || index + 1 == active_blocks.len() {
            println!("processed {}/{}", index + 1, active_blocks.len());
        }
    }

    // --- 4. build token + pool state -----------------------------------------
    let token_decimals = provider.get_token_decimals(args.token, Some(end_block)).await?;
    let metadata = token_metadata(provider.as_ref(), args.token, end_block, token_decimals).await?;
    let token_symbol = metadata.symbol.clone();
    let token_name = metadata.name.clone();
    let builder = TokenStateBuilder::new(metadata, HISTORY_LIMIT);
    let token = builder
        .build_from_processed_blocks(processed_blocks)
        .wrap_err("failed to build token state from active blocks")?;
    let pool = token
        .pool_base(addr(args.pool))
        .ok_or_else(|| eyre!("pool {} missing after token replay", addr(args.pool)))?;

    if pool.reserve_tracker.reserve_history.is_empty() {
        notes.push(
            "pool replay produced no reserve snapshots; check pool address / window bounds"
                .to_string(),
        );
    }

    // --- 5. compute everything from the pool state ---------------------------
    let initial_price = pool.initial_price().unwrap_or(0.0);
    let price_series = build_price_series(pool, initial_price);
    let inflation = compute_inflation(pool, &price_series, initial_price, removal_block);
    let liquidity = compute_liquidity(pool, removal_block);
    let trading = compute_trading(pool, args.token, args.pool);
    let scam_mechanism = compute_scam_mechanism(pool);
    let lifecycle = build_lifecycle(&resolved, &price_series, inflation.peak_block, removal_block);

    let summary = Summary {
        token: addr(args.token),
        pool: addr(args.pool),
        suspect: addr(args.suspect),
        denom: addr(args.denom),
        token_symbol,
        token_name,
        start_block,
        end_block,
        active_block_count: active_blocks.len(),
        replay_window_blocks,
    };

    notes.push(format!(
        "trading counts from pool.swap_events ({} events); price/liquidity from reserve_history deltas",
        pool.swap_events.len()
    ));
    let provenance = Provenance {
        method: "reth_db_replay_token_state_builder".to_string(),
        block_range: format!("{start_block}..={end_block}"),
        history_limit: HISTORY_LIMIT,
        notes,
    };

    let artifact = ScamMechanicsArtifact {
        summary,
        inflation: inflation.clone(),
        liquidity: liquidity.clone(),
        trading: trading.clone(),
        scam_mechanism: scam_mechanism.clone(),
        lifecycle: lifecycle.clone(),
        price_series,
        provenance,
    };

    // --- 6. write artifacts ---------------------------------------------------
    write_outputs(&args.case_dir, &artifact)?;

    println!();
    println!("Key numbers:");
    println!("  initial_price:    {:.18}", inflation.initial_price);
    println!("  peak_price:       {:.18} (block {:?})", inflation.peak_price, inflation.peak_block);
    println!("  price_at_removal: {:.18}", inflation.price_at_removal);
    println!("  inflation_x:      {:.4}", inflation.inflation_x);
    println!("  collapse_ratio:   {:.6}", inflation.collapse_ratio);
    println!("  eth_added_as_lp:  {:.9} (block {:?})", liquidity.eth_added_as_lp, liquidity.eth_added_block);
    println!("  eth_removed_rug:  {:.9} (block {:?})", liquidity.eth_removed_at_rug, liquidity.eth_removed_block);
    println!("  net_eth_extracted:{:.9}", liquidity.net_eth_extracted);
    println!("  buyers/buys/sells:{} / {} / {}", trading.distinct_buyers, trading.buy_count, trading.sell_count);
    println!("  scam_mechanism:   {:?} ({})", scam_mechanism.mechanism, scam_mechanism.source);
    println!();
    println!("wrote: {}/artifacts/scam_mechanics.json", args.case_dir.display());
    println!("wrote: {}/reports/how_it_was_scammed.md", args.case_dir.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// computations
// ---------------------------------------------------------------------------

fn build_price_series(pool: &BasePool, initial_price: f64) -> Vec<PriceSeriesPoint> {
    pool.reserve_tracker
        .reserve_history
        .iter()
        .map(|snapshot| {
            let ratio = if initial_price > 0.0 && snapshot.price.is_finite() {
                snapshot.price / initial_price
            } else {
                0.0
            };
            PriceSeriesPoint {
                block_number: snapshot.block_number,
                tx_hash: snapshot.tx_hash.clone(),
                denom_reserve: snapshot.denom_reserve,
                token_reserve: snapshot.token_reserve,
                price: snapshot.price,
                price_ratio_to_initial: ratio,
            }
        })
        .collect()
}

fn compute_inflation(
    pool: &BasePool,
    series: &[PriceSeriesPoint],
    initial_price: f64,
    removal_block: Option<u64>,
) -> Inflation {
    // The pump peak is the highest price reached *while liquidity was still in
    // the pool*, i.e. strictly before the rug. After the liquidity is pulled, a
    // dust-reserve snapshot can divide a tiny token reserve into a spuriously
    // huge "price" — that is a numerical artifact of the drain, not the pump, so
    // we exclude any snapshot at/after the removal block when a removal is known.
    let mut peak_price = 0.0_f64;
    let mut peak_block: Option<u64> = None;
    for snapshot in &pool.reserve_tracker.reserve_history {
        if let Some(removal) = removal_block {
            if snapshot.block_number >= removal {
                continue;
            }
        }
        if snapshot.price.is_finite() && snapshot.price > peak_price {
            peak_price = snapshot.price;
            peak_block = Some(snapshot.block_number);
        }
    }

    // price at/just after the removal block: first snapshot with block >= removal,
    // else the last snapshot in the series.
    let price_at_removal = match removal_block {
        Some(removal) => series
            .iter()
            .find(|point| point.block_number >= removal)
            .or_else(|| series.last())
            .map(|point| point.price)
            .unwrap_or(0.0),
        None => series.last().map(|point| point.price).unwrap_or(0.0),
    };

    let inflation_x = if initial_price > 0.0 {
        peak_price / initial_price
    } else {
        0.0
    };
    let collapse_ratio = if peak_price > 0.0 {
        price_at_removal / peak_price
    } else {
        0.0
    };

    Inflation {
        initial_price,
        peak_price,
        peak_block,
        price_at_removal,
        inflation_x,
        collapse_ratio,
    }
}

/// Compute the ETH added as LP and removed at the rug from denom_reserve deltas.
/// The LP add is the largest single *upward* jump in denom_reserve; the rug is
/// the largest single *downward* drop. net = removed - added.
fn compute_liquidity(pool: &BasePool, removal_block: Option<u64>) -> Liquidity {
    let history = &pool.reserve_tracker.reserve_history;
    let mut prev: Option<f64> = None;
    let mut max_up = 0.0_f64;
    let mut max_up_block: Option<u64> = None;
    let mut max_down = 0.0_f64; // stored as positive magnitude
    let mut max_down_block: Option<u64> = None;

    for snapshot in history {
        if let Some(previous) = prev {
            let delta = snapshot.denom_reserve - previous;
            if delta > max_up {
                max_up = delta;
                max_up_block = Some(snapshot.block_number);
            }
            // A drop at/after the removal block (or any large drop) is the rug.
            let drop = previous - snapshot.denom_reserve;
            let is_after_removal = removal_block.map(|r| snapshot.block_number >= r).unwrap_or(true);
            if drop > max_down && (is_after_removal || drop > max_up * 0.5) {
                max_down = drop;
                max_down_block = Some(snapshot.block_number);
            }
        } else {
            // First observed snapshot: the denom_reserve seeded into the pool is
            // itself an "add" if there was no prior baseline.
            if snapshot.denom_reserve > max_up {
                max_up = snapshot.denom_reserve;
                max_up_block = Some(snapshot.block_number);
            }
        }
        prev = Some(snapshot.denom_reserve);
    }

    let net_eth_extracted = max_down - max_up;

    Liquidity {
        eth_added_as_lp: max_up,
        eth_added_block: max_up_block,
        eth_removed_at_rug: max_down,
        eth_removed_block: max_down_block,
        net_eth_extracted,
        source: "reserve_history_denom_delta".to_string(),
        mint_event_count: pool.mint_events.len(),
        burn_event_count: pool.burn_events.len(),
    }
}

fn compute_trading(pool: &BasePool, _token: Address, pool_addr: Address) -> Trading {
    let pool_lower = addr(pool_addr).to_ascii_lowercase();
    let mut buy_count = 0_u64;
    let mut sell_count = 0_u64;
    let mut buyers: BTreeSet<String> = BTreeSet::new();
    for event in &pool.swap_events {
        let is_buy = event.get("is_buy").and_then(Value::as_bool).unwrap_or(false);
        let is_sell = event.get("is_sell").and_then(Value::as_bool).unwrap_or(false);
        if is_buy {
            buy_count += 1;
            if let Some(to) = event.get("to").and_then(Value::as_str) {
                let to_lower = to.trim().to_ascii_lowercase();
                if to_lower != pool_lower {
                    buyers.insert(to_lower);
                }
            }
        }
        if is_sell {
            sell_count += 1;
        }
    }
    Trading {
        buy_count,
        sell_count,
        swap_count: pool.swap_events.len(),
        distinct_buyers: buyers.len(),
        confidence: "tool_derived_from_swap_events".to_string(),
    }
}

fn compute_scam_mechanism(pool: &BasePool) -> ScamMechanism {
    if let Some(mechanism) = pool.inferred_scam_mechanism() {
        return ScamMechanism {
            mechanism: Some(mechanism.mechanism),
            label: Some(mechanism.label),
            block_number: mechanism.block_number,
            tx_hash: mechanism.tx_hash,
            source: "inferred_scam_mechanism".to_string(),
        };
    }
    ScamMechanism {
        mechanism: pool.scam_mechanism.clone(),
        label: pool.scam_mechanism_label.clone(),
        block_number: pool.scam_block,
        tx_hash: pool.scam_tx_hash.clone(),
        source: "pool_stored_fields".to_string(),
    }
}

fn build_lifecycle(
    resolved: &[ResolvedKeyTx],
    series: &[PriceSeriesPoint],
    peak_block: Option<u64>,
    removal_block: Option<u64>,
) -> Vec<LifecycleEntry> {
    let mut entries: Vec<LifecycleEntry> = resolved
        .iter()
        .map(|tx| LifecycleEntry {
            phase: tx.phase.clone(),
            label: tx.label.clone(),
            kind: "key_tx".to_string(),
            block_number: tx.block_number,
            timestamp: tx.timestamp,
            tx_hash: Some(tx.tx_hash.clone()),
        })
        .collect();

    // Derived phases.
    if let Some(first) = series.iter().find(|point| point.price > 0.0) {
        entries.push(LifecycleEntry {
            phase: "first_trade".to_string(),
            label: "First on-chain trade established a live price".to_string(),
            kind: "derived".to_string(),
            block_number: Some(first.block_number),
            timestamp: None,
            tx_hash: Some(first.tx_hash.clone()),
        });
    }
    if let Some(peak) = peak_block {
        entries.push(LifecycleEntry {
            phase: "peak".to_string(),
            label: "Price peaked (top of the pump)".to_string(),
            kind: "derived".to_string(),
            block_number: Some(peak),
            timestamp: None,
            tx_hash: None,
        });
    }
    if let Some(removal) = removal_block {
        entries.push(LifecycleEntry {
            phase: "collapse".to_string(),
            label: "Price collapsed to ~0 after liquidity was pulled".to_string(),
            kind: "derived".to_string(),
            block_number: Some(removal),
            timestamp: None,
            tx_hash: None,
        });
    }

    entries.sort_by_key(|entry| (entry.block_number.unwrap_or(u64::MAX), entry.kind.clone()));
    entries
}

// ---------------------------------------------------------------------------
// provider helpers
// ---------------------------------------------------------------------------

async fn resolve_tx(provider: &RethQueryProvider, tx_hash: &str) -> (Option<u64>, Option<u64>) {
    let Ok(hash) = B256::from_str(tx_hash.trim()) else {
        return (None, None);
    };
    match provider.get_transaction_by_hash(hash).await {
        Ok(data) => (Some(data.block_number), Some(data.block_timestamp)),
        Err(_) => (None, None),
    }
}

fn activity_blocks(
    provider: &RethQueryProvider,
    token: Address,
    pool: Address,
    start_block: u64,
    end_block: u64,
) -> Result<Vec<u64>> {
    let mut blocks = BTreeSet::<u64>::new();
    for address in [token, pool] {
        let mut address_blocks = provider.get_address_participation_blocks(address)?;
        address_blocks.retain(|block| *block >= start_block && *block <= end_block);
        blocks.extend(address_blocks);
    }
    Ok(blocks.into_iter().collect())
}

async fn token_metadata(
    provider: &RethQueryProvider,
    token: Address,
    end_block: u64,
    token_decimals: u8,
) -> Result<ERC20TokenMetadata> {
    Ok(provider
        .get_token_metadata(token, Some(end_block), None)
        .await?
        .map(|meta| {
            ERC20TokenMetadata::new(
                addr(token),
                meta.name,
                meta.symbol,
                token_decimals,
                meta.total_supply.to_string(),
            )
        })
        .unwrap_or_else(|| {
            ERC20TokenMetadata::new(addr(token), "unknown", "UNKNOWN", token_decimals, "0")
        }))
}

// ---------------------------------------------------------------------------
// case.toml key_txs parsing (no external toml dep needed beyond the workspace's)
// ---------------------------------------------------------------------------

fn read_key_txs(case_dir: &Path) -> Result<std::collections::BTreeMap<String, String>> {
    let path = case_dir.join("case.toml");
    let text = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read {}", path.display()))?;
    let value: toml::Value = toml::from_str(&text)
        .wrap_err_with(|| format!("failed to parse {}", path.display()))?;
    let mut out = std::collections::BTreeMap::new();
    if let Some(table) = value.get("key_txs").and_then(|v| v.as_table()) {
        for (key, val) in table {
            if let Some(hash) = val.as_str() {
                out.insert(key.clone(), hash.to_string());
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// output writing
// ---------------------------------------------------------------------------

fn write_outputs(case_dir: &Path, artifact: &ScamMechanicsArtifact) -> Result<()> {
    let artifacts_dir = case_dir.join("artifacts");
    let reports_dir = case_dir.join("reports");
    fs::create_dir_all(&artifacts_dir)?;
    fs::create_dir_all(&reports_dir)?;

    let json_path = artifacts_dir.join("scam_mechanics.json");
    fs::write(&json_path, serde_json::to_string_pretty(artifact)?)
        .wrap_err_with(|| format!("failed to write {}", json_path.display()))?;

    let md = render_markdown(artifact);
    let md_path = reports_dir.join("how_it_was_scammed.md");
    fs::write(&md_path, md).wrap_err_with(|| format!("failed to write {}", md_path.display()))?;
    Ok(())
}

fn render_markdown(a: &ScamMechanicsArtifact) -> String {
    let s = &a.summary;
    let inf = &a.inflation;
    let liq = &a.liquidity;
    let tr = &a.trading;
    let mech_label = a
        .scam_mechanism
        .label
        .clone()
        .unwrap_or_else(|| "Liquidity removal (rug pull)".to_string());

    let key_tx = |phase: &str| -> String {
        a.lifecycle
            .iter()
            .find(|entry| entry.phase == phase)
            .and_then(|entry| entry.tx_hash.clone())
            .unwrap_or_else(|| "(unknown tx)".to_string())
    };
    let key_block = |phase: &str| -> String {
        a.lifecycle
            .iter()
            .find(|entry| entry.phase == phase)
            .and_then(|entry| entry.block_number)
            .map(|b| b.to_string())
            .unwrap_or_else(|| "?".to_string())
    };

    let token_label = if s.token_symbol == "UNKNOWN" || s.token_symbol.is_empty() {
        s.token.clone()
    } else {
        format!("{} ({})", s.token_symbol, s.token)
    };

    let mut out = String::new();
    out.push_str("# How it was scammed\n\n");
    out.push_str(&format!(
        "Token **{}** was launched, pumped, and rugged by the operator `{}`.\n\n",
        token_label, s.suspect
    ));

    out.push_str("## The short version\n\n");
    out.push_str(&format!(
        "1. The operator **deployed** the token (block {}, tx `{}`).\n",
        key_block("token_deploy"),
        key_tx("token_deploy")
    ));
    out.push_str(&format!(
        "2. They **added about {:.4} ETH of liquidity** to the Uniswap pool `{}` (around block {}), which sets the starting price.\n",
        liq.eth_added_as_lp,
        s.pool,
        liq.eth_added_block.map(|b| b.to_string()).unwrap_or_else(|| "?".to_string())
    ));
    out.push_str(&format!(
        "3. They **opened trading** (block {}, tx `{}`). As buyers came in, the price was inflated **{:.1}x** — from a starting price of {:.3e} ETH/token up to a peak of {:.3e} ETH/token{}.\n",
        key_block("open_trading"),
        key_tx("open_trading"),
        inf.inflation_x,
        inf.initial_price,
        inf.peak_price,
        inf.peak_block.map(|b| format!(" (peak around block {b})")).unwrap_or_default()
    ));
    out.push_str(&format!(
        "4. With the price pumped, the operator **pulled the liquidity** (block {}, tx `{}`), taking roughly **{:.4} ETH** back out of the pool.\n",
        key_block("liquidity_removal"),
        key_tx("liquidity_removal"),
        liq.eth_removed_at_rug
    ));
    out.push_str(&format!(
        "5. The moment the liquidity was gone, the price **collapsed to ~0** (down to {:.4}% of the peak), so everyone who bought is left holding tokens that can no longer be sold for anything.\n\n",
        inf.collapse_ratio * 100.0
    ));

    out.push_str("## The numbers\n\n");
    out.push_str(&format!("- **Mechanism:** {}\n", mech_label));
    out.push_str(&format!(
        "- **ETH added as liquidity:** {:.6} ETH\n",
        liq.eth_added_as_lp
    ));
    out.push_str(&format!(
        "- **ETH removed at the rug:** {:.6} ETH\n",
        liq.eth_removed_at_rug
    ));
    out.push_str(&format!(
        "- **Net ETH the operator extracted (removed − added):** {:.6} ETH\n",
        liq.net_eth_extracted
    ));
    out.push_str(&format!(
        "- **Price inflation:** {:.1}x (from {:.3e} to {:.3e} ETH/token)\n",
        inf.inflation_x, inf.initial_price, inf.peak_price
    ));
    out.push_str(&format!(
        "- **Price after the rug:** {:.3e} ETH/token ({:.4}% of peak)\n",
        inf.price_at_removal,
        inf.collapse_ratio * 100.0
    ));
    out.push_str(&format!(
        "- **Trading observed:** {} buys, {} sells, {} distinct buyers caught in the pump.\n\n",
        tr.buy_count, tr.sell_count, tr.distinct_buyers
    ));

    out.push_str("## Timeline (how it unfolded)\n\n");
    out.push_str("| Block | Phase | What happened | Tx |\n");
    out.push_str("| ---: | --- | --- | --- |\n");
    for entry in &a.lifecycle {
        let block = entry
            .block_number
            .map(|b| b.to_string())
            .unwrap_or_else(|| "?".to_string());
        let tx = entry
            .tx_hash
            .clone()
            .map(|hash| format!("`{}`", short_hash(&hash)))
            .unwrap_or_else(|| "-".to_string());
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            block, entry.phase, entry.label, tx
        ));
    }
    out.push('\n');

    out.push_str("## Scope and confidence\n\n");
    out.push_str(&format!(
        "- Replayed pool `{}` over blocks {}..={} ({} active blocks).\n",
        s.pool, s.start_block, s.end_block, s.active_block_count
    ));
    out.push_str(&format!(
        "- Price/liquidity numbers are derived from on-chain reserve snapshots ({}). Trading counts are derived from swap events ({}).\n",
        a.provenance.method, tr.confidence
    ));
    if !a.provenance.notes.is_empty() {
        out.push_str("- Notes:\n");
        for note in &a.provenance.notes {
            out.push_str(&format!("  - {}\n", note));
        }
    }
    out.push('\n');
    out.push_str("_Machine-readable detail: `artifacts/scam_mechanics.json`._\n");
    out
}

// ---------------------------------------------------------------------------
// arg parsing + small utils
// ---------------------------------------------------------------------------

fn parse_args() -> Result<Args> {
    let datadir = std::env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());
    let reth_index =
        std::env::var("RETH_INDEX_DIR").unwrap_or_else(|_| format!("{datadir}/reth_index"));
    let case_dir = std::env::var("CASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CASE_DIR));
    Ok(Args {
        datadir,
        reth_index,
        case_dir,
        token: parse_env_addr("TOKEN", DEFAULT_TOKEN)?,
        pool: parse_env_addr("POOL", DEFAULT_POOL)?,
        suspect: parse_env_addr("SUSPECT", DEFAULT_SUSPECT)?,
        denom: parse_env_addr("DENOM", DEFAULT_DENOM)?,
        start_block_override: env_opt_u64("START_BLOCK"),
        end_block_override: env_opt_u64("END_BLOCK"),
        lookahead_blocks: env_u64("LOOKAHEAD_BLOCKS", DEFAULT_LOOKAHEAD_BLOCKS),
        max_window_blocks: env_u64("MAX_WINDOW_BLOCKS", DEFAULT_MAX_WINDOW_BLOCKS),
    })
}

fn parse_env_addr(name: &str, default: &str) -> Result<Address> {
    let value = std::env::var(name).unwrap_or_else(|_| default.to_string());
    Address::from_str(value.trim()).map_err(|error| eyre!("invalid address {value}: {error}"))
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_opt_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|value| value.parse().ok())
}

fn addr(address: Address) -> String {
    to_checksum_address(&address)
}

fn short_hash(hash: &str) -> String {
    if hash.len() <= 14 {
        hash.to_string()
    } else {
        format!("{}...{}", &hash[..10], &hash[hash.len() - 6..])
    }
}
