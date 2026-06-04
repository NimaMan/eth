//! Suspect cash-out fund-flow trace — follow a scammer's value (ETH/WETH +
//! stablecoins) outward, hop by hop, until it reaches a centralized exchange (or
//! a confidence-bounded sink).
//!
//! This is the actor-side complement to `session_scammer_case_report` (which
//! reconstructs *buyer* outcomes). It answers the law-enforcement question
//! "where did the operator take the funds?" by tracing the suspect/control
//! wallets' value forward to a labeled CEX deposit/hot wallet, and by recording
//! the funding source(s) that seeded the operation (the KYC entry point).
//!
//! # Why this design
//! - Objective: a submittable per-scammer packet showing the cash-out path to an
//!   exchange. We reuse the proven reth-sourced edge primitive
//!   (`extract_fund_flows_from_processed_tx`, which yields BOTH native ETH and
//!   ERC-20 token movements) and label terminals with the in-tree
//!   `reth_chain_query` CEX catalog (2,776 named addresses) via
//!   `identify_known_address` / `get_cex_by_address`. The Postgres graph-discovery
//!   engine is NOT used: its `eth_db.addresses`/`tx_participants` tables are absent
//!   here, whereas reth + the catalog are present.
//! - A DeFi rug operator's proceeds are WETH/stablecoins, not native ETH, so we
//!   track three value assets: ETH (native ETH and WETH unified 1:1), USDC, USDT.
//!
//! # Algorithm
//!  1. Seeds = {suspect, control}, BFS roots at depth 0 with unbounded taint in
//!     every tracked asset (everything an origin wallet sends is scammer funds).
//!  2. Level-synchronous BFS. For each frontier node, read the blocks it touched
//!     in [window_start, window_end] and, per transaction (in time order),
//!     collect the node's outgoing and incoming asset movements.
//!  3. VALUE-CONSERVING (taint) attribution, per asset: a node may forward asset
//!     A only up to the asset-A value it received from the scammer subgraph
//!     (`tainted_in[A]`). Each outgoing movement's tainted portion is recorded on
//!     the edge; the destination accrues that taint. Once a non-seed node's
//!     asset-A taint is spent, its further asset-A movements are its own funds and
//!     are not followed. This stops the trace from absorbing an intermediary's
//!     unrelated throughput.
//!  4. SWAP-CARRY: if a node spent tainted value in a transaction and received a
//!     different asset in the same transaction (a DEX swap / unwrap), the received
//!     asset is treated as scammer proceeds and becomes tainted — so taint follows
//!     value across WETH<->USDC<->ETH conversions.
//!  5. Labels & terminals via `identify_known_address`. A node is a reported
//!     cash-out TERMINAL when it is a CEX. CEX, burn, DEX routers/factories, and
//!     the case pool/token are NON-expandable (we stop there): for routers/pools
//!     the value continues with the spending wallet via swap-carry, not through
//!     the router's own liquidity.
//!  6. FUNDING scan: for seeds (depth 0), incoming movements are recorded as
//!     funding observations (a CEX source is the operator's KYC entry point).
//!  7. Value is reported per asset AND as an ETH-equivalent (stablecoins via a
//!     configurable ETH/USD price) so a single headline number exists. Outputs:
//!       artifacts/tx_fund_flow/suspect_cashout_trace.json
//!       reports/fund_flow_to_exchange.md
//!     Unlabeled terminal sinks carrying real tainted value are reported as
//!     "next-step: CEX-deposit check" leads. Confirmed CEX addresses should be
//!     added to `reth_chain_query/common_addresses/cex` so future traces resolve
//!     them automatically.
//!
//! Run (defaults to the Session vault-drain case):
//!   cargo run -p eth_token --example custody_suspect_cashout_trace
//! Env overrides: RETH_DATADIR, RETH_INDEX_DIR, SESSION_CASE_DIR,
//!   SESSION_CASE_{SUSPECT,CONTROL,VAULT,TOKEN,POOL,DRAIN_BLOCK},
//!   CASHOUT_LOOKBACK_BLOCKS, CASHOUT_MAX_DEPTH, CASHOUT_MAX_NODES,
//!   CASHOUT_MAX_BLOCKS_PER_ADDR, CASHOUT_MIN_VALUE_ETH, CASHOUT_ETH_USD,
//!   CASHOUT_INCLUDE_TRACES.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address, U256};
use eyre::{eyre, Result, WrapErr};
use reth_chain_query::common_addresses::{
    get_cex_by_address, identify_known_address, is_burn_address_str, KnownAddressKind,
};
use reth_chain_query::{to_checksum_address, RethQueryProvider};
use serde::Serialize;
use tx_fund_flow_fundflownetwork::extract_fund_flows_from_processed_tx;
use tx_processor::{BlockProcessor, ProcessedBlock};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_CASE_DIR: &str = "/home/nima/code/crypto/blockchains/eth/risk_atlas/scammer_analytics/cases/eth_0x02467dd0_session_vault_balance_drain_25202411";
const DEFAULT_TOKEN: &str = "0xc3A640bD249381F8097f44C1b61C46172068cDff";
const DEFAULT_POOL: &str = "0x77a43d235c261436f0ad4577ce575ff0991adc1a";
const DEFAULT_SUSPECT: &str = "0x02467dd05200e5B3FBcdddbF43735aE4d0681a59";
const DEFAULT_CONTROL: &str = "0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4";
const DEFAULT_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const DEFAULT_DRAIN_BLOCK: u64 = 25_202_411;
const DEFAULT_LOOKBACK_BLOCKS: u64 = 50_000;
const DEFAULT_MAX_DEPTH: u32 = 4;
const DEFAULT_MAX_NODES: usize = 60;
const DEFAULT_MAX_BLOCKS_PER_ADDR: usize = 200;
const DEFAULT_MIN_VALUE_ETH: f64 = 0.01;
const DEFAULT_ETH_USD: f64 = 2500.0;

// Tracked value assets. ETH unifies native ETH and WETH 1:1.
const ASSET_ETH: &str = "ETH";
const ASSET_USDC: &str = "USDC";
const ASSET_USDT: &str = "USDT";
const TRACKED_ASSETS: [&str; 3] = [ASSET_ETH, ASSET_USDC, ASSET_USDT];
const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

// Recoverability buckets (DESIGN §4.1). Each final node maps to exactly one.
const RECOVER_AT_EXCHANGE: &str = "at_exchange";
const RECOVER_IN_WALLET: &str = "in_wallet";
const RECOVER_BRIDGED: &str = "bridged";
const RECOVER_DESTROYED: &str = "destroyed";
const RECOVER_NONE: &str = "none";

// Known cross-chain bridge contracts (checksummed). Value reaching one of these
// is classified as `bridged` (cross-chain follow-up) for recoverability. Kept to
// a small high-confidence set; bridges are non-expandable terminals because the
// trace cannot follow value to another chain.
const KNOWN_BRIDGES: [(&str, &str); 9] = [
    ("0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE", "LI.FI Diamond"),
    ("0x5C7BCd6E7De5423a257D81B442095A1a6ced35C5", "Across SpokePool"),
    ("0x8731d54E9D02c286767d56ac03e8037C07e01e98", "Stargate Router"),
    ("0x2796317b0fF8538F253012862c06787Adfb8cEb6", "Synapse Bridge"),
    ("0xb8901acB165ed027E32754E0FFe830802919727f", "Hop L1 ETH Bridge"),
    ("0x3666f603Cc164936C1b87e207F36BEBa4AC5f18a", "cBridge (Celer)"),
    ("0xa0c68C638235ee32657e8f720a23ceC1bFc77C77", "Polygon PoS Bridge"),
    ("0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf", "Polygon ERC20 Bridge"),
    ("0x99C9fc46f92E8a1c0deC1b1747d010903E884bE1", "Optimism Gateway"),
];

#[derive(Debug, Clone)]
struct Args {
    datadir: String,
    reth_index: String,
    case_dir: PathBuf,
    token: Address,
    pool: Address,
    suspect: Address,
    control: Address,
    vault: Address,
    drain_block: u64,
    lookback_blocks: u64,
    max_depth: u32,
    max_nodes: usize,
    max_blocks_per_addr: usize,
    min_value_eth: f64,
    eth_usd: f64,
    include_traces: bool,
    autoseed_sellers: bool,
    max_seller_seeds: usize,
    max_discovery_blocks: usize,
    discovery_lookahead: u64,
}

impl Args {
    /// ETH-equivalent value of `amount` units of `asset`.
    fn value_eth(&self, asset: &str, amount: f64) -> f64 {
        if asset == ASSET_ETH {
            amount
        } else {
            // Stablecoins are priced ~1 USD.
            amount / self.eth_usd
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct TraceNode {
    address: String,
    role: String,
    kind: String,
    name: Option<String>,
    exchange: Option<String>,
    depth: u32,
    is_terminal: bool,
    expandable: bool,
    explored: bool,
    value_in_eth: f64,
    value_out_eth: f64,
    tainted_value_in_eth: f64,
    /// Recoverability classification (DESIGN §4.1) for this node's tainted value:
    /// "at_exchange" | "in_wallet" | "bridged" | "destroyed" | "none".
    recoverability_bucket: String,
}

/// Recoverability split of traced value into actionability buckets (DESIGN §4.1).
/// Value-conserving: derived from the same per-node tainted ETH-equivalents.
#[derive(Clone, Debug, Default, Serialize)]
struct Recoverability {
    /// Σ tainted value at CEX terminals — freeze-able via the exchange.
    at_exchange_eth: f64,
    /// Σ tainted value at unexpanded, unlabeled, non-venue/non-seed wallet leads
    /// — still traceable / potentially recoverable.
    in_wallet_eth: f64,
    /// Σ tainted value at known-bridge nodes — cross-chain follow-up.
    bridged_eth: f64,
    /// Σ tainted value at burn nodes — destroyed.
    destroyed_eth: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TraceEdge {
    direction: String, // "outgoing" | "funding"
    asset: String,
    from_address: String,
    to_address: String,
    from_label: String,
    to_label: String,
    tx_hash: String,
    block_number: u64,
    block_timestamp: u64,
    amount: f64,      // in asset units
    amount_raw: String,
    value_eth: f64,   // ETH-equivalent gross
    tainted_amount: f64, // scammer-attributed, asset units
    tainted_value_eth: f64,
    movement_type: String,
    depth: u32,
}

#[derive(Clone, Debug, Serialize)]
struct TraceTerminal {
    address: String,
    kind: String,
    exchange: Option<String>,
    name: Option<String>,
    depth: u32,
    total_value_received_eth: f64,
    received_by_asset: BTreeMap<String, f64>,
    shortest_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct FundingSource {
    seed: String,
    seed_role: String,
    source_address: String,
    source_label: String,
    source_kind: String,
    source_exchange: Option<String>,
    asset: String,
    amount: f64,
    value_eth: f64,
    tx_hash: String,
    block_number: u64,
}

#[derive(Clone, Debug, Serialize)]
struct TraceSummary {
    token_address: String,
    pool_address: String,
    suspect_address: String,
    control_address: String,
    vault_address: String,
    drain_block: u64,
    window_start: u64,
    window_end: u64,
    max_depth: u32,
    min_value_eth: f64,
    eth_usd: f64,
    tracked_assets: Vec<String>,
    seed_addresses: Vec<String>,
    node_count: usize,
    edge_count: usize,
    cex_terminal_count: usize,
    unlabeled_lead_count: usize,
    total_value_out_traced_eth: f64,
    total_value_to_cex_eth: f64,
    out_traced_by_asset: BTreeMap<String, f64>,
    funding_source_count: usize,
    cex_funding_source_count: usize,
    /// Recoverability split (DESIGN §4.1) of the traced value.
    recoverability: Recoverability,
}

#[derive(Clone, Debug, Serialize)]
struct TraceArtifact {
    summary: TraceSummary,
    nodes: Vec<TraceNode>,
    edges: Vec<TraceEdge>,
    terminals: Vec<TraceTerminal>,
    funding_sources: Vec<FundingSource>,
    unlabeled_leads: Vec<TraceNode>,
}

#[derive(Clone, Debug)]
struct NodeState {
    address: Address,
    depth: u32,
    is_terminal: bool,
    expandable: bool,
    explored: bool,
    kind: KnownAddressKind2,
    name: Option<String>,
    exchange: Option<String>,
    value_in_eth: f64,
    value_out_eth: f64,
    /// Scammer-attributed inflow per asset. Seeds carry +inf in every asset.
    tainted_in: HashMap<String, f64>,
    tainted_value_in_eth: f64,
    parent: Option<Address>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnownAddressKind2 {
    Cex,
    Burn,
    Bridge,
    DexRouter,
    DexFactory,
    FeeRecipient,
    Stablecoin,
    DenomToken,
    Etf,
    Wallet,
    Named,
    Unlabeled,
}

impl KnownAddressKind2 {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cex => "cex",
            Self::Burn => "burn",
            Self::Bridge => "bridge",
            Self::DexRouter => "dex_router",
            Self::DexFactory => "dex_factory",
            Self::FeeRecipient => "fee_recipient",
            Self::Stablecoin => "stablecoin",
            Self::DenomToken => "denom_token",
            Self::Etf => "etf",
            Self::Wallet => "wallet",
            Self::Named => "named",
            Self::Unlabeled => "unlabeled",
        }
    }
}

fn classify_known_kind(kind: KnownAddressKind) -> KnownAddressKind2 {
    match kind {
        KnownAddressKind::Cex => KnownAddressKind2::Cex,
        KnownAddressKind::Burn => KnownAddressKind2::Burn,
        KnownAddressKind::DexRouter => KnownAddressKind2::DexRouter,
        KnownAddressKind::DexFactory => KnownAddressKind2::DexFactory,
        KnownAddressKind::FeeRecipient => KnownAddressKind2::FeeRecipient,
        KnownAddressKind::Stablecoin => KnownAddressKind2::Stablecoin,
        KnownAddressKind::DenomToken => KnownAddressKind2::DenomToken,
        KnownAddressKind::Etf => KnownAddressKind2::Etf,
        KnownAddressKind::Wallet => KnownAddressKind2::Wallet,
        KnownAddressKind::Named => KnownAddressKind2::Named,
    }
}

/// Look up a known cross-chain bridge contract by address (case-insensitive).
/// Returns its display name when the address is a known bridge.
fn known_bridge_name(address: Address) -> Option<&'static str> {
    let checksum = to_checksum_address(&address);
    KNOWN_BRIDGES
        .iter()
        .find(|(bridge, _)| bridge.eq_ignore_ascii_case(&checksum))
        .map(|(_, name)| *name)
}

fn label_address(address: Address) -> (KnownAddressKind2, Option<String>, Option<String>) {
    // Known bridges take precedence so cross-chain leakage is classified even
    // when the catalog has no entry for the address.
    if let Some(name) = known_bridge_name(address) {
        return (
            KnownAddressKind2::Bridge,
            Some(name.to_string()),
            Some(name.to_string()),
        );
    }
    if let Some(known) = identify_known_address(address) {
        let kind = classify_known_kind(known.kind);
        let exchange = if kind == KnownAddressKind2::Cex {
            get_cex_by_address(address)
                .map(|entry| entry.exchange.to_string())
                .or_else(|| known.group.map(str::to_string))
        } else {
            known.group.map(str::to_string)
        };
        (kind, Some(known.name.to_string()), exchange)
    } else if is_burn_address_str(&to_checksum_address(&address)) {
        (KnownAddressKind2::Burn, Some("burn".to_string()), None)
    } else {
        (KnownAddressKind2::Unlabeled, None, None)
    }
}

fn compact_label(
    kind: KnownAddressKind2,
    name: &Option<String>,
    exchange: &Option<String>,
) -> String {
    match (exchange, name) {
        (Some(exchange), _) => format!("{} ({})", exchange, kind.as_str()),
        (None, Some(name)) => format!("{} ({})", name, kind.as_str()),
        (None, None) => kind.as_str().to_string(),
    }
}

/// Map a token contract to a tracked value asset and its decimals.
fn token_asset(token: Address) -> Option<(&'static str, u8)> {
    let token = to_checksum_address(&token);
    if token.eq_ignore_ascii_case(WETH_ADDRESS) {
        Some((ASSET_ETH, 18))
    } else if token.eq_ignore_ascii_case(USDC_ADDRESS) {
        Some((ASSET_USDC, 6))
    } else if token.eq_ignore_ascii_case(USDT_ADDRESS) {
        Some((ASSET_USDT, 6))
    } else {
        None
    }
}

/// A single asset movement of the current node within one transaction.
#[derive(Clone, Debug)]
struct AssetMove {
    asset: &'static str,
    counterparty: Address,
    amount: f64,
    amount_raw: String,
    movement_type: String,
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
    let window_start = args.drain_block.saturating_sub(args.lookback_blocks);
    let window_end = latest;
    let tx_processor = BlockProcessor::new(provider.clone());

    println!("Suspect cash-out fund-flow trace (ETH/WETH + stablecoins)");
    println!("suspect:  {}", addr(args.suspect));
    println!("control:  {}", addr(args.control));
    println!("window:   {window_start}..={window_end}  (latest={latest})");
    println!(
        "limits:   depth<={} nodes<={} blocks/addr<={} min_value={} ETH eth_usd={}",
        args.max_depth, args.max_nodes, args.max_blocks_per_addr, args.min_value_eth, args.eth_usd
    );
    println!();

    let mut block_cache = HashMap::<u64, ProcessedBlock>::new();
    let mut nodes = HashMap::<Address, NodeState>::new();
    let mut edges = Vec::<TraceEdge>::new();
    let mut edge_seen = BTreeSet::<String>::new();
    let mut funding_sources = Vec::<FundingSource>::new();

    let mut seed_set: Vec<Address> = vec![args.suspect, args.control];
    let mut seller_seeds: BTreeSet<Address> = BTreeSet::new();
    if args.autoseed_sellers {
        let discovered =
            discover_seller_seeds(provider.as_ref(), &tx_processor, &mut block_cache, &args, window_start)
                .await?;
        for seller in discovered {
            if !seed_set.contains(&seller) {
                seed_set.push(seller);
                seller_seeds.insert(seller);
            }
        }
        println!("discovered operator-seller seeds: {}", seller_seeds.len());
        for seller in &seller_seeds {
            println!("  seller seed: {}", addr(*seller));
        }
        println!();
    }
    for seed in &seed_set {
        insert_node(&mut nodes, *seed, 0, None, &args);
        if let Some(state) = nodes.get_mut(seed) {
            state.expandable = true; // seeds are always expandable origin wallets
            for asset in TRACKED_ASSETS {
                state.tainted_in.insert(asset.to_string(), f64::INFINITY);
            }
        }
    }

    let mut frontier: Vec<Address> = seed_set.clone();
    let mut expanded: BTreeSet<Address> = BTreeSet::new();

    for _level in 0..args.max_depth {
        let mut next: Vec<Address> = Vec::new();
        let mut in_next: BTreeSet<Address> = BTreeSet::new();

        for current in std::mem::take(&mut frontier) {
            let (depth, expandable) = {
                let state = nodes.get(&current).expect("frontier node exists");
                (state.depth, state.expandable)
            };
            if !expandable {
                continue;
            }
            if !expanded.insert(current) {
                continue;
            }
            if nodes.len() > args.max_nodes {
                break;
            }

            // Per-asset remaining taint budget, evolving over the node's txs.
            let mut remaining: HashMap<String, f64> = nodes
                .get(&current)
                .map(|state| state.tainted_in.clone())
                .unwrap_or_default();

            let mut blocks = provider
                .get_address_participation_blocks(current)
                .unwrap_or_default();
            blocks.retain(|block| *block >= window_start && *block <= window_end);
            blocks.sort_unstable();
            blocks.truncate(args.max_blocks_per_addr);

            for block_number in blocks {
                let block = match block_cache.get(&block_number) {
                    Some(block) => block.clone(),
                    None => {
                        let processed = tx_processor
                            .process_block_with_options(block_number, args.include_traces)
                            .await
                            .wrap_err_with(|| format!("failed to process block {block_number}"))?;
                        block_cache.insert(block_number, processed.clone());
                        processed
                    }
                };
                for entry in &block.transactions {
                    let tx = &entry.processed;
                    if !tx_involves(tx, current) {
                        continue;
                    }
                    let flows = extract_fund_flows_from_processed_tx(tx)?;
                    let tx_hash = hash(tx.hash);

                    // Collect this node's outgoing and incoming asset movements
                    // in this tx.
                    let mut outs: Vec<AssetMove> = Vec::new();
                    let mut ins: Vec<AssetMove> = Vec::new();
                    for movement in &flows.eth_movements {
                        let movement_type = format!("{:?}", movement.movement_type);
                        if movement_type == "Gas" {
                            continue;
                        }
                        let amount = wei_to_eth(movement.amount);
                        if args.value_eth(ASSET_ETH, amount) < args.min_value_eth {
                            continue;
                        }
                        if movement.from == current {
                            outs.push(AssetMove {
                                asset: ASSET_ETH,
                                counterparty: resolve_zero(movement.to, tx),
                                amount,
                                amount_raw: movement.amount.to_string(),
                                movement_type,
                            });
                        } else if movement.to == current {
                            ins.push(AssetMove {
                                asset: ASSET_ETH,
                                counterparty: movement.from,
                                amount,
                                amount_raw: movement.amount.to_string(),
                                movement_type,
                            });
                        }
                    }
                    for movement in &flows.token_movements {
                        let Some((asset, decimals)) = token_asset(movement.token_address) else {
                            continue;
                        };
                        let amount = scale_units(movement.amount, decimals);
                        if args.value_eth(asset, amount) < args.min_value_eth {
                            continue;
                        }
                        if movement.from == current {
                            outs.push(AssetMove {
                                asset,
                                counterparty: movement.to,
                                amount,
                                amount_raw: movement.amount.to_string(),
                                movement_type: "Token".to_string(),
                            });
                        } else if movement.to == current {
                            ins.push(AssetMove {
                                asset,
                                counterparty: movement.from,
                                amount,
                                amount_raw: movement.amount.to_string(),
                                movement_type: "Token".to_string(),
                            });
                        }
                    }

                    // Attribute taint to outgoing movements (per asset, time order).
                    let mut tx_had_tainted_out = false;
                    for out in &outs {
                        let avail = remaining.get(out.asset).copied().unwrap_or(0.0);
                        let tainted = if avail.is_infinite() {
                            out.amount
                        } else {
                            out.amount.min(avail)
                        };
                        let value_eth = args.value_eth(out.asset, out.amount);
                        let tainted_value_eth = args.value_eth(out.asset, tainted);
                        record_edge(
                            &mut edges,
                            &mut edge_seen,
                            "outgoing",
                            out.asset,
                            &tx_hash,
                            tx.block_number,
                            tx.block_timestamp,
                            current,
                            out.counterparty,
                            out.amount,
                            &out.amount_raw,
                            value_eth,
                            tainted,
                            tainted_value_eth,
                            &out.movement_type,
                            depth,
                        );
                        if let Some(state) = nodes.get_mut(&current) {
                            state.value_out_eth += value_eth;
                        }
                        if !nodes.contains_key(&out.counterparty) {
                            insert_node(&mut nodes, out.counterparty, depth + 1, Some(current), &args);
                        }
                        let dest_expandable = if let Some(dest) = nodes.get_mut(&out.counterparty) {
                            dest.value_in_eth += value_eth;
                            if tainted > 0.0 {
                                *dest.tainted_in.entry(out.asset.to_string()).or_insert(0.0) +=
                                    tainted;
                                dest.tainted_value_in_eth += tainted_value_eth;
                            }
                            dest.expandable
                        } else {
                            false
                        };
                        if avail.is_finite() {
                            *remaining.entry(out.asset.to_string()).or_insert(0.0) -= tainted;
                        }
                        if tainted_value_eth >= args.min_value_eth {
                            tx_had_tainted_out = true;
                            if dest_expandable
                                && !expanded.contains(&out.counterparty)
                                && in_next.insert(out.counterparty)
                            {
                                next.push(out.counterparty);
                            }
                        }
                    }

                    // Swap-carry: tainted spend + same-tx receipt of another asset
                    // is swap/unwrap proceeds — taint the received asset.
                    if tx_had_tainted_out {
                        for inn in &ins {
                            *remaining.entry(inn.asset.to_string()).or_insert(0.0) += inn.amount;
                        }
                    }

                    // Funding observations on seeds (operation entry points).
                    if depth == 0 {
                        for inn in &ins {
                            if inn.counterparty == current {
                                continue;
                            }
                            let (kind, name, exchange) = label_address(inn.counterparty);
                            funding_sources.push(FundingSource {
                                seed: addr(current),
                                seed_role: role_for(current, &args),
                                source_address: addr(inn.counterparty),
                                source_label: compact_label(kind, &name, &exchange),
                                source_kind: kind.as_str().to_string(),
                                source_exchange: exchange,
                                asset: inn.asset.to_string(),
                                amount: inn.amount,
                                value_eth: args.value_eth(inn.asset, inn.amount),
                                tx_hash: tx_hash.clone(),
                                block_number: tx.block_number,
                            });
                        }
                    }
                }
            }

            if let Some(state) = nodes.get_mut(&current) {
                state.explored = true;
            }
        }

        frontier = next;
    }

    let artifact = build_artifact(
        &args,
        window_start,
        window_end,
        &nodes,
        edges,
        funding_sources,
        &seller_seeds,
    );
    write_outputs(&args.case_dir, &artifact)?;

    println!(
        "traced: nodes={} edges={} cex_terminals={} unlabeled_leads={} funding_sources={}",
        artifact.summary.node_count,
        artifact.summary.edge_count,
        artifact.summary.cex_terminal_count,
        artifact.summary.unlabeled_lead_count,
        artifact.summary.funding_source_count,
    );
    println!(
        "value out traced: {:.4} ETH-eq  to CEX: {:.4} ETH-eq  by-asset: {:?}",
        artifact.summary.total_value_out_traced_eth,
        artifact.summary.total_value_to_cex_eth,
        artifact.summary.out_traced_by_asset,
    );
    if artifact.terminals.is_empty() {
        println!(
            "No labeled CEX endpoint reached within depth {}. See unlabeled leads in the report for CEX-deposit follow-up; add confirmed CEX addresses to reth_chain_query/common_addresses/cex.",
            args.max_depth
        );
    } else {
        for terminal in &artifact.terminals {
            println!(
                "  CEX endpoint: {} {} depth={} received={:.4} ETH-eq",
                terminal.exchange.as_deref().unwrap_or("?"),
                terminal.address,
                terminal.depth,
                terminal.total_value_received_eth,
            );
        }
    }
    println!("case: {}", args.case_dir.display());
    Ok(())
}

fn insert_node(
    nodes: &mut HashMap<Address, NodeState>,
    address: Address,
    depth: u32,
    parent: Option<Address>,
    args: &Args,
) {
    if nodes.contains_key(&address) {
        return;
    }
    let (kind, name, exchange) = label_address(address);
    // CEX, burn, and bridge are terminal sinks: a bridge moves value to another
    // chain, so the trace cannot follow it further on this chain.
    let is_terminal = matches!(
        kind,
        KnownAddressKind2::Cex | KnownAddressKind2::Burn | KnownAddressKind2::Bridge
    );
    // Routers/factories and the case venue contracts are non-expandable: value
    // continues with the spending wallet via swap-carry, not through them.
    let is_service = matches!(kind, KnownAddressKind2::DexRouter | KnownAddressKind2::DexFactory);
    let is_venue = address == args.pool || address == args.token;
    let is_seed = address == args.suspect || address == args.control;
    let expandable = is_seed || (!is_terminal && !is_service && !is_venue);
    nodes.insert(
        address,
        NodeState {
            address,
            depth,
            is_terminal,
            expandable,
            explored: false,
            kind,
            name,
            exchange,
            value_in_eth: 0.0,
            value_out_eth: 0.0,
            tainted_in: HashMap::new(),
            tainted_value_in_eth: 0.0,
            parent,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn record_edge(
    edges: &mut Vec<TraceEdge>,
    seen: &mut BTreeSet<String>,
    direction: &str,
    asset: &str,
    tx_hash: &str,
    block_number: u64,
    block_timestamp: u64,
    from: Address,
    to: Address,
    amount: f64,
    amount_raw: &str,
    value_eth: f64,
    tainted_amount: f64,
    tainted_value_eth: f64,
    movement_type: &str,
    depth: u32,
) {
    let key = format!(
        "{}:{}:{}:{}:{}:{}",
        tx_hash,
        asset,
        addr(from).to_ascii_lowercase(),
        addr(to).to_ascii_lowercase(),
        amount_raw,
        movement_type
    );
    if !seen.insert(key) {
        return;
    }
    let (from_kind, from_name, from_exchange) = label_address(from);
    let (to_kind, to_name, to_exchange) = label_address(to);
    edges.push(TraceEdge {
        direction: direction.to_string(),
        asset: asset.to_string(),
        from_address: addr(from),
        to_address: addr(to),
        from_label: compact_label(from_kind, &from_name, &from_exchange),
        to_label: compact_label(to_kind, &to_name, &to_exchange),
        tx_hash: tx_hash.to_string(),
        block_number,
        block_timestamp,
        amount,
        amount_raw: amount_raw.to_string(),
        value_eth,
        tainted_amount,
        tainted_value_eth,
        movement_type: movement_type.to_string(),
        depth,
    });
}

fn build_artifact(
    args: &Args,
    window_start: u64,
    window_end: u64,
    nodes: &HashMap<Address, NodeState>,
    edges: Vec<TraceEdge>,
    funding_sources: Vec<FundingSource>,
    seller_seeds: &BTreeSet<Address>,
) -> TraceArtifact {
    let mut node_rows = Vec::<TraceNode>::new();
    let mut terminals = Vec::<TraceTerminal>::new();
    let mut unlabeled_leads = Vec::<TraceNode>::new();

    // Per-terminal received-by-asset, from tainted edge values.
    let mut received_by_asset: HashMap<Address, BTreeMap<String, f64>> = HashMap::new();
    for edge in &edges {
        if edge.direction != "outgoing" || edge.tainted_amount <= 0.0 {
            continue;
        }
        if let Ok(to) = Address::from_str(&edge.to_address) {
            *received_by_asset
                .entry(to)
                .or_default()
                .entry(edge.asset.clone())
                .or_insert(0.0) += edge.tainted_amount;
        }
    }

    let mut recoverability = Recoverability::default();

    for state in nodes.values() {
        // A lead is an unexpanded, unlabeled NON-venue sink carrying real traced
        // value — a candidate CEX-deposit address to confirm. Exclude the case
        // venue contracts and the seed wallets themselves.
        let is_venue_or_seed = state.address == args.token
            || state.address == args.pool
            || state.address == args.suspect
            || state.address == args.control
            || seller_seeds.contains(&state.address);
        let tainted_eth = finite_or_zero(state.tainted_value_in_eth);
        let is_wallet_lead = state.kind == KnownAddressKind2::Unlabeled
            && tainted_eth >= args.min_value_eth
            && !state.explored
            && !is_venue_or_seed;
        // Recoverability bucket (DESIGN §4.1): value-conserving split of the same
        // per-node tainted ETH-equivalents into actionability buckets.
        let bucket = recoverability_bucket(state.kind, is_wallet_lead);
        match bucket {
            RECOVER_AT_EXCHANGE => recoverability.at_exchange_eth += tainted_eth,
            RECOVER_IN_WALLET => recoverability.in_wallet_eth += tainted_eth,
            RECOVER_BRIDGED => recoverability.bridged_eth += tainted_eth,
            RECOVER_DESTROYED => recoverability.destroyed_eth += tainted_eth,
            _ => {}
        }

        let row = TraceNode {
            address: addr(state.address),
            role: if seller_seeds.contains(&state.address) {
                "operator_seller".to_string()
            } else {
                role_for(state.address, args)
            },
            kind: state.kind.as_str().to_string(),
            name: state.name.clone(),
            exchange: state.exchange.clone(),
            depth: state.depth,
            is_terminal: state.is_terminal,
            expandable: state.expandable,
            explored: state.explored,
            value_in_eth: state.value_in_eth,
            value_out_eth: state.value_out_eth,
            tainted_value_in_eth: tainted_eth,
            recoverability_bucket: bucket.to_string(),
        };
        if state.kind == KnownAddressKind2::Cex {
            terminals.push(TraceTerminal {
                address: addr(state.address),
                kind: state.kind.as_str().to_string(),
                exchange: state.exchange.clone(),
                name: state.name.clone(),
                depth: state.depth,
                total_value_received_eth: finite_or_zero(state.tainted_value_in_eth),
                received_by_asset: received_by_asset
                    .get(&state.address)
                    .cloned()
                    .unwrap_or_default(),
                shortest_path: reconstruct_path(nodes, state.address),
            });
        }
        if is_wallet_lead {
            unlabeled_leads.push(row.clone());
        }
        node_rows.push(row);
    }

    node_rows.sort_by(|left, right| {
        left.depth.cmp(&right.depth).then_with(|| {
            right
                .tainted_value_in_eth
                .partial_cmp(&left.tainted_value_in_eth)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    terminals.sort_by(|left, right| {
        right
            .total_value_received_eth
            .partial_cmp(&left.total_value_received_eth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    unlabeled_leads.sort_by(|left, right| {
        right
            .tainted_value_in_eth
            .partial_cmp(&left.tainted_value_in_eth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out_traced_by_asset: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_value_out_traced_eth = 0.0;
    for edge in &edges {
        if edge.direction != "outgoing" {
            continue;
        }
        *out_traced_by_asset.entry(edge.asset.clone()).or_insert(0.0) += edge.tainted_amount;
        total_value_out_traced_eth += edge.tainted_value_eth;
    }
    let total_value_to_cex_eth = terminals.iter().map(|t| t.total_value_received_eth).sum::<f64>();
    let cex_funding_source_count = funding_sources
        .iter()
        .filter(|f| f.source_kind == "cex")
        .count();

    let summary = TraceSummary {
        token_address: addr(args.token),
        pool_address: addr(args.pool),
        suspect_address: addr(args.suspect),
        control_address: addr(args.control),
        vault_address: addr(args.vault),
        drain_block: args.drain_block,
        window_start,
        window_end,
        max_depth: args.max_depth,
        min_value_eth: args.min_value_eth,
        eth_usd: args.eth_usd,
        tracked_assets: TRACKED_ASSETS.iter().map(|a| a.to_string()).collect(),
        seed_addresses: {
            let mut all = vec![addr(args.suspect), addr(args.control)];
            all.extend(seller_seeds.iter().map(|address| addr(*address)));
            all
        },
        node_count: node_rows.len(),
        edge_count: edges.len(),
        cex_terminal_count: terminals.len(),
        unlabeled_lead_count: unlabeled_leads.len(),
        total_value_out_traced_eth,
        total_value_to_cex_eth,
        out_traced_by_asset,
        funding_source_count: funding_sources.len(),
        cex_funding_source_count,
        recoverability,
    };

    TraceArtifact {
        summary,
        nodes: node_rows,
        edges,
        terminals,
        funding_sources,
        unlabeled_leads,
    }
}

fn reconstruct_path(nodes: &HashMap<Address, NodeState>, target: Address) -> Vec<String> {
    let mut path = Vec::new();
    let mut cursor = Some(target);
    let mut guard = 0;
    while let Some(address) = cursor {
        path.push(addr(address));
        cursor = nodes.get(&address).and_then(|state| state.parent);
        guard += 1;
        if guard > 64 {
            break;
        }
    }
    path.reverse();
    path
}

fn tx_involves(tx: &tx_processor::ProcessedTransaction, address: Address) -> bool {
    if tx.from_address == address {
        return true;
    }
    if tx.to_address == Some(address) {
        return true;
    }
    tx.unique_addresses.iter().any(|candidate| *candidate == address)
}

fn write_outputs(case_dir: &Path, artifact: &TraceArtifact) -> Result<()> {
    let tx_dir = case_dir.join("artifacts").join("tx_fund_flow");
    let reports_dir = case_dir.join("reports");
    fs::create_dir_all(&tx_dir)?;
    fs::create_dir_all(&reports_dir)?;
    let json = serde_json::to_string_pretty(artifact)?;
    let json_path = tx_dir.join("suspect_cashout_trace.json");
    fs::write(&json_path, json)
        .wrap_err_with(|| format!("failed to write {}", json_path.display()))?;
    write_report(reports_dir.join("fund_flow_to_exchange.md"), artifact)?;
    Ok(())
}

fn write_report(path: impl AsRef<Path>, artifact: &TraceArtifact) -> Result<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    let s = &artifact.summary;
    writeln!(writer, "# Suspect Cash-Out Fund Flow to Exchange")?;
    writeln!(writer)?;
    writeln!(writer, "Status: investigative — facts and confidence only, not attribution.")?;
    writeln!(writer)?;
    writeln!(writer, "Value is traced as ETH/WETH (unified 1:1) plus USDC/USDT, value-conserving (taint-limited) so an intermediary's unrelated throughput is excluded. ETH-equivalent uses ETH/USD = `{}`.", s.eth_usd)?;
    writeln!(writer)?;
    writeln!(writer, "## Scope")?;
    writeln!(writer)?;
    writeln!(writer, "- suspect: `{}`", s.suspect_address)?;
    writeln!(writer, "- control contract: `{}`", s.control_address)?;
    writeln!(writer, "- victim vault: `{}`", s.vault_address)?;
    writeln!(writer, "- token: `{}`", s.token_address)?;
    writeln!(writer, "- drain block: `{}`", s.drain_block)?;
    writeln!(writer, "- trace window: `{}..={}`", s.window_start, s.window_end)?;
    writeln!(writer, "- assets: `{}`; depth<=`{}`; min value `{}` ETH-eq", s.tracked_assets.join(", "), s.max_depth, s.min_value_eth)?;
    writeln!(writer)?;
    writeln!(writer, "## Result")?;
    writeln!(writer)?;
    writeln!(writer, "- addresses discovered: `{}`", s.node_count)?;
    writeln!(writer, "- value movement edges: `{}`", s.edge_count)?;
    writeln!(writer, "- total value out traced: `{:.4}` ETH-eq", s.total_value_out_traced_eth)?;
    writeln!(writer, "- by asset (units): `{:?}`", s.out_traced_by_asset)?;
    writeln!(writer, "- CEX endpoints reached: `{}` (`{:.4}` ETH-eq)", s.cex_terminal_count, s.total_value_to_cex_eth)?;
    writeln!(writer, "- unlabeled terminal leads: `{}`", s.unlabeled_lead_count)?;
    writeln!(writer, "- funding sources observed: `{}` (CEX: `{}`)", s.funding_source_count, s.cex_funding_source_count)?;
    writeln!(writer)?;

    writeln!(writer, "## Recoverability (ETH-eq)")?;
    writeln!(writer)?;
    writeln!(writer, "Value-conserving split of traced value into actionability buckets (DESIGN §4.1).")?;
    writeln!(writer)?;
    writeln!(writer, "- at an exchange (freeze-able via the exchange): `{:.4}`", s.recoverability.at_exchange_eth)?;
    writeln!(writer, "- in a wallet (still traceable / potentially recoverable): `{:.4}`", s.recoverability.in_wallet_eth)?;
    writeln!(writer, "- bridged (cross-chain follow-up): `{:.4}`", s.recoverability.bridged_eth)?;
    writeln!(writer, "- destroyed (burn): `{:.4}`", s.recoverability.destroyed_eth)?;
    writeln!(writer)?;

    writeln!(writer, "## Cash-Out Endpoints (Centralized Exchanges)")?;
    writeln!(writer)?;
    if artifact.terminals.is_empty() {
        writeln!(writer, "No labeled CEX deposit/hot-wallet endpoint was reached within the depth budget. See unlabeled leads below for CEX-deposit follow-up.")?;
    } else {
        writeln!(writer, "| Exchange | Address | Depth | Value (ETH-eq) | By Asset | Path |")?;
        writeln!(writer, "| --- | --- | ---: | ---: | --- | --- |")?;
        for terminal in &artifact.terminals {
            writeln!(
                writer,
                "| `{}` | `{}` | `{}` | `{:.4}` | `{:?}` | `{}` |",
                terminal.exchange.as_deref().unwrap_or("?"),
                terminal.address,
                terminal.depth,
                terminal.total_value_received_eth,
                terminal.received_by_asset,
                terminal
                    .shortest_path
                    .iter()
                    .map(|a| short_addr(a))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )?;
        }
    }
    writeln!(writer)?;

    writeln!(writer, "## Funding Sources (Operation Entry Points)")?;
    writeln!(writer)?;
    if artifact.funding_sources.is_empty() {
        writeln!(writer, "No incoming value to the seeds observed in the window.")?;
    } else {
        writeln!(writer, "| Seed | Source | Label | Asset | Amount | Tx | Block |")?;
        writeln!(writer, "| --- | --- | --- | --- | ---: | --- | ---: |")?;
        for funding in artifact.funding_sources.iter().take(40) {
            writeln!(
                writer,
                "| `{}` | `{}` | `{}` | `{}` | `{:.4}` | `{}` | `{}` |",
                short_addr(&funding.seed),
                short_addr(&funding.source_address),
                funding.source_label,
                funding.asset,
                funding.amount,
                short_addr(&funding.tx_hash),
                funding.block_number,
            )?;
        }
    }
    writeln!(writer)?;

    writeln!(writer, "## Unlabeled Terminal Leads (next step: CEX-deposit check)")?;
    writeln!(writer)?;
    if artifact.unlabeled_leads.is_empty() {
        writeln!(writer, "None.")?;
    } else {
        writeln!(writer, "These sinks received traced value but are not in the known-address catalogs. Confirm via explorer labels whether any is a CEX deposit address; if so, add it to `reth_chain_query/common_addresses/cex` so future traces resolve it automatically.")?;
        writeln!(writer)?;
        writeln!(writer, "| Address | Depth | Value Received (ETH-eq) |")?;
        writeln!(writer, "| --- | ---: | ---: |")?;
        for lead in artifact.unlabeled_leads.iter().take(40) {
            writeln!(
                writer,
                "| `{}` | `{}` | `{:.4}` |",
                lead.address, lead.depth, lead.tainted_value_in_eth
            )?;
        }
    }
    writeln!(writer)?;

    writeln!(writer, "## Artifacts")?;
    writeln!(writer)?;
    writeln!(writer, "- `artifacts/tx_fund_flow/suspect_cashout_trace.json`")?;
    Ok(())
}

fn parse_args() -> Result<Args> {
    let datadir = std::env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());
    let reth_index =
        std::env::var("RETH_INDEX_DIR").unwrap_or_else(|_| format!("{datadir}/reth_index"));
    let case_dir = std::env::var("SESSION_CASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CASE_DIR));
    Ok(Args {
        datadir,
        reth_index,
        case_dir,
        token: parse_env_addr("SESSION_CASE_TOKEN", DEFAULT_TOKEN)?,
        pool: parse_env_addr("SESSION_CASE_POOL", DEFAULT_POOL)?,
        suspect: parse_env_addr("SESSION_CASE_SUSPECT", DEFAULT_SUSPECT)?,
        control: parse_env_addr("SESSION_CASE_CONTROL", DEFAULT_CONTROL)?,
        vault: parse_env_addr("SESSION_CASE_VAULT", DEFAULT_VAULT)?,
        drain_block: env_u64("SESSION_CASE_DRAIN_BLOCK", DEFAULT_DRAIN_BLOCK),
        lookback_blocks: env_u64("CASHOUT_LOOKBACK_BLOCKS", DEFAULT_LOOKBACK_BLOCKS),
        max_depth: env_u64("CASHOUT_MAX_DEPTH", DEFAULT_MAX_DEPTH as u64) as u32,
        max_nodes: env_u64("CASHOUT_MAX_NODES", DEFAULT_MAX_NODES as u64) as usize,
        max_blocks_per_addr: env_u64("CASHOUT_MAX_BLOCKS_PER_ADDR", DEFAULT_MAX_BLOCKS_PER_ADDR as u64)
            as usize,
        min_value_eth: env_f64("CASHOUT_MIN_VALUE_ETH", DEFAULT_MIN_VALUE_ETH),
        eth_usd: env_f64("CASHOUT_ETH_USD", DEFAULT_ETH_USD).max(1.0),
        include_traces: env_bool("CASHOUT_INCLUDE_TRACES", false),
        autoseed_sellers: env_bool("CASHOUT_AUTOSEED_SELLERS", true),
        max_seller_seeds: env_u64("CASHOUT_MAX_SELLER_SEEDS", 6) as usize,
        max_discovery_blocks: env_u64("CASHOUT_MAX_DISCOVERY_BLOCKS", 1500) as usize,
        discovery_lookahead: env_u64("CASHOUT_DISCOVERY_LOOKAHEAD", 2000),
    })
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(default)
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

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn role_for(address: Address, args: &Args) -> String {
    if address == args.suspect {
        "suspect".to_string()
    } else if address == args.control {
        "control_contract".to_string()
    } else if address == args.vault {
        "victim_vault".to_string()
    } else if address == args.pool {
        "pool".to_string()
    } else if address == args.token {
        "token".to_string()
    } else {
        "address".to_string()
    }
}

/// A native-ETH "Direct" movement whose recipient is the zero address is almost
/// always a contract-creation value transfer; the real destination is the
/// created contract. Resolve it so the trace does not mislabel it as a burn.
fn resolve_zero(to: Address, tx: &tx_processor::ProcessedTransaction) -> Address {
    if to == Address::ZERO {
        if let Some(created) = tx.contract_address {
            return created;
        }
    }
    to
}

/// Discover the operator's realized-proceeds wallets: addresses that sold
/// materially more of the token into the pool than they ever bought from it (the
/// surplus is off-market supply they received from the creator/operator and
/// dumped for value). These are seeded into the cash-out trace alongside the
/// named suspect/control wallets.
async fn discover_seller_seeds(
    provider: &RethQueryProvider,
    tx_processor: &BlockProcessor,
    block_cache: &mut HashMap<u64, ProcessedBlock>,
    args: &Args,
    window_start: u64,
) -> Result<Vec<Address>> {
    let discovery_end = args.drain_block.saturating_add(args.discovery_lookahead);
    let decimals = provider
        .get_token_decimals(args.token, Some(discovery_end))
        .await
        .unwrap_or(18);
    let mut blocks = BTreeSet::<u64>::new();
    for anchor in [args.token, args.pool] {
        if let Ok(participation) = provider.get_address_participation_blocks(anchor) {
            for block in participation {
                if block >= window_start && block <= discovery_end {
                    blocks.insert(block);
                }
            }
        }
    }
    let blocks: Vec<u64> = blocks.into_iter().take(args.max_discovery_blocks).collect();

    let mut bought = HashMap::<Address, f64>::new();
    let mut sold = HashMap::<Address, f64>::new();
    for block_number in blocks {
        let block = match block_cache.get(&block_number) {
            Some(block) => block.clone(),
            None => {
                let processed = tx_processor
                    .process_block_with_options(block_number, false)
                    .await
                    .wrap_err_with(|| format!("failed to process discovery block {block_number}"))?;
                block_cache.insert(block_number, processed.clone());
                processed
            }
        };
        for entry in &block.transactions {
            for transfer in &entry.processed.erc20_transfers {
                if transfer.token_address != args.token {
                    continue;
                }
                let amount = scale_units(transfer.amount, decimals);
                if transfer.from_address == args.pool && transfer.to_address != args.pool {
                    *bought.entry(transfer.to_address).or_default() += amount;
                }
                if transfer.to_address == args.pool && transfer.from_address != args.pool {
                    *sold.entry(transfer.from_address).or_default() += amount;
                }
            }
        }
    }

    let mut candidates: Vec<(Address, f64)> = sold
        .iter()
        .filter_map(|(address, sold_amount)| {
            if *address == args.pool
                || *address == args.token
                || *address == args.suspect
                || *address == args.control
                || is_burn_address_str(&to_checksum_address(address))
            {
                return None;
            }
            if matches!(
                label_address(*address).0,
                KnownAddressKind2::Cex | KnownAddressKind2::DexRouter | KnownAddressKind2::DexFactory
            ) {
                return None;
            }
            let bought_amount = bought.get(address).copied().unwrap_or(0.0);
            let net = sold_amount - bought_amount;
            if net > 0.0 && *sold_amount > bought_amount * 1.05 {
                Some((*address, net))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(args.max_seller_seeds);
    Ok(candidates.into_iter().map(|(address, _)| address).collect())
}

fn addr(address: Address) -> String {
    to_checksum_address(&address)
}

fn hash(value: alloy_primitives::B256) -> String {
    format!("{value:?}")
}

fn wei_to_eth(value: U256) -> f64 {
    scale_units(value, 18)
}

fn scale_units(value: U256, decimals: u8) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(decimals as i32)
}

/// Classify a final node's tainted value into a recoverability bucket
/// (DESIGN §4.1). `is_wallet_lead` is true for an unexpanded, unlabeled,
/// non-venue/non-seed sink still carrying tainted value (traceable / recoverable).
fn recoverability_bucket(kind: KnownAddressKind2, is_wallet_lead: bool) -> &'static str {
    match kind {
        KnownAddressKind2::Cex => RECOVER_AT_EXCHANGE,
        KnownAddressKind2::Bridge => RECOVER_BRIDGED,
        KnownAddressKind2::Burn => RECOVER_DESTROYED,
        _ if is_wallet_lead => RECOVER_IN_WALLET,
        _ => RECOVER_NONE,
    }
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn short_addr(value: &str) -> String {
    if value.len() <= 14 {
        value.to_string()
    } else {
        format!("{}...{}", &value[..8], &value[value.len() - 6..])
    }
}
