//! Build the Risk Atlas buyer-outcome and fund-flow packet for the Session
//! owner-controlled balance-drain case.
//!
//! Run:
//!   cargo run -p eth_token --example custody_session_scammer_case_report

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use eth_token::custody::{
    reconcile_custody_drain, CustodyDrainVictim, HolderBalanceLedger, HolderBalanceReader,
    ReconcileConfig,
};
use eth_token::erc20::ERC20TokenMetadata;
use eth_token::pnl::PnlAddressPositionExport;
use eth_token::tracking::TokenStateBuilder;
use eyre::{bail, eyre, Result, WrapErr};
use reth_chain_query::common_addresses::is_burn_address_str;
use reth_chain_query::{to_checksum_address, RethQueryProvider};
use serde::Serialize;
use tx_fund_flow_fundflownetwork::extract_fund_flows_from_processed_tx;
use tx_processor::{BlockProcessor, ProcessedBlock, ProcessedTransaction};

const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_CASE_DIR: &str = "/home/nima/code/crypto/blockchains/eth/risk_atlas/scammer_analytics/cases/eth_0x02467dd0_session_vault_balance_drain_25202411";
const DEFAULT_TOKEN: &str = "0xc3A640bD249381F8097f44C1b61C46172068cDff";
const DEFAULT_POOL: &str = "0x77a43d235c261436f0ad4577ce575ff0991adc1a";
const DEFAULT_SUSPECT: &str = "0x02467dd05200e5B3FBcdddbF43735aE4d0681a59";
const DEFAULT_CONTROL: &str = "0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4";
const DEFAULT_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const DEFAULT_START_BLOCK: u64 = 25_202_408;
const DEFAULT_DRAIN_BLOCK: u64 = 25_202_411;
const DEFAULT_LOOKBACK_BLOCKS: u64 = 50_000;
const DEFAULT_LOOKAHEAD_BLOCKS: u64 = 300;
const HISTORY_LIMIT: usize = 20_000;
const MIN_BUYER_BALANCE: f64 = 1.0;
const MIN_DRAIN_FRACTION: f64 = 0.9;

#[derive(Debug)]
struct Args {
    datadir: String,
    reth_index: String,
    case_dir: PathBuf,
    token: Address,
    pool: Address,
    suspect: Address,
    control: Address,
    vault: Address,
    start_block: u64,
    end_block: Option<u64>,
    drain_block: u64,
    trace_all_blocks: bool,
    lookback_blocks: u64,
    lookahead_blocks: u64,
}

#[derive(Clone)]
struct ProviderBalanceReader {
    provider: Arc<RethQueryProvider>,
    token: Address,
    decimals: u8,
}

#[async_trait]
impl HolderBalanceReader for ProviderBalanceReader {
    async fn balance_of(&self, holder: &str, block: u64) -> Result<f64> {
        let holder = Address::from_str(holder)
            .map_err(|error| eyre!("invalid holder address {holder}: {error}"))?;
        let raw = self
            .provider
            .get_token_balance(self.token, holder, Some(block))
            .await?;
        Ok(scale(raw, self.decimals))
    }
}

#[derive(Clone, Debug)]
struct TokenMove {
    tx_hash: String,
    block_number: u64,
    tx_index: u64,
    from_address: String,
    to_address: String,
    amount_scaled: f64,
    source: &'static str,
}

#[derive(Clone, Debug, Default)]
struct BuyerAccumulator {
    buy_amount: f64,
    buy_count: u64,
    first_buy_block: Option<u64>,
    first_buy_tx: Option<String>,
    last_buy_block: Option<u64>,
    last_buy_tx: Option<String>,
    emitted_out_to_pool: f64,
    emitted_out_to_other: f64,
    emitted_out_to_burn: f64,
    eventless_out_to_pool: f64,
    eventless_out_to_other: f64,
    eventless_out_to_burn: f64,
    eventless_in: f64,
    normal_sell_count: u64,
}

#[derive(Clone, Debug, Serialize)]
struct BuyerOutcomeRow {
    buyer: String,
    classification: String,
    evidence_level: String,
    bought_amount: f64,
    buy_count: u64,
    first_buy_block: Option<u64>,
    first_buy_tx: Option<String>,
    last_buy_block: Option<u64>,
    last_buy_tx: Option<String>,
    normal_out_to_pool: f64,
    normal_out_to_other: f64,
    normal_out_to_burn: f64,
    eventless_out_to_pool: f64,
    eventless_out_to_other: f64,
    eventless_out_to_burn: f64,
    eventless_in: f64,
    expected_balance: f64,
    actual_balance: f64,
    missing_balance: f64,
    drained_fraction: f64,
    token_balance_from_pnl: Option<String>,
    denom_cashflow_eth: Option<String>,
    native_fee_eth: Option<String>,
    pnl_proxy_eth: Option<String>,
    nearest_pre_buy_funder: Option<String>,
    nearest_pre_buy_funding_tx: Option<String>,
    nearest_pre_buy_funding_block: Option<u64>,
    nearest_pre_buy_funding_eth: Option<f64>,
    creator_connection: String,
    connection_confidence: String,
    connection_evidence: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct BuyerOutcomeSummary {
    token_address: String,
    pool_address: String,
    suspect_address: String,
    control_address: String,
    vault_address: String,
    start_block: u64,
    audit_block: u64,
    active_block_count: usize,
    buyer_count: usize,
    confiscated_count: usize,
    sold_normally_count: usize,
    partial_exit_count: usize,
    still_holding_count: usize,
    transferred_or_burned_count: usize,
    unknown_count: usize,
    direct_creator_connection_count: usize,
    shared_funder_cluster_count: usize,
    reconciled_custody_findings_added: usize,
    total_custody_findings_after_reconciliation: usize,
}

#[derive(Clone, Debug, Serialize)]
struct BuyerOutcomeArtifact {
    summary: BuyerOutcomeSummary,
    buyers: Vec<BuyerOutcomeRow>,
}

#[derive(Clone, Debug, Serialize)]
struct FundFlowEdge {
    tx_hash: String,
    block_number: u64,
    block_timestamp: u64,
    from_address: String,
    to_address: String,
    amount_wei: String,
    amount_eth: f64,
    movement_type: String,
    from_role: String,
    to_role: String,
}

#[derive(Clone, Debug, Serialize)]
struct FundingObservation {
    tx_hash: String,
    block_number: u64,
    block_timestamp: u64,
    from_address: String,
    to_address: String,
    amount_eth: f64,
    movement_type: String,
}

#[derive(Clone, Debug, Serialize)]
struct AddressCluster {
    cluster_id: String,
    cluster_type: String,
    confidence: String,
    hub_address: String,
    members: Vec<String>,
    evidence: Vec<String>,
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
    let start_block = args.start_block.min(latest);
    let mut active_blocks = activity_blocks(
        provider.as_ref(),
        args.token,
        args.pool,
        start_block,
        args.end_block.unwrap_or(latest).min(latest),
    )?;
    if active_blocks.is_empty() {
        bail!("no active blocks found for token/pool in requested range");
    }
    let audit_block = *active_blocks.last().expect("checked non-empty");

    println!("Session scammer case report");
    println!("token:      {}", addr(args.token));
    println!("pool:       {}", addr(args.pool));
    println!("case_dir:   {}", args.case_dir.display());
    println!(
        "active:     {} blocks, {}..={}",
        active_blocks.len(),
        active_blocks[0],
        audit_block
    );

    let tx_processor = BlockProcessor::new(provider.clone());
    let mut processed_blocks = BTreeMap::<u64, ProcessedBlock>::new();
    for (index, block_number) in active_blocks.iter().enumerate() {
        let include_traces = args.trace_all_blocks || *block_number == args.drain_block;
        let block = tx_processor
            .process_block_with_options(*block_number, include_traces)
            .await
            .wrap_err_with(|| format!("failed to process active block {block_number}"))?;
        processed_blocks.insert(*block_number, block);
        if (index + 1) % 10 == 0 || index + 1 == active_blocks.len() {
            println!(
                "processed active blocks: {}/{}",
                index + 1,
                active_blocks.len()
            );
        }
    }

    let active_processed_blocks = active_blocks
        .iter()
        .filter_map(|block| processed_blocks.get(block).cloned())
        .collect::<Vec<_>>();

    let token_decimals = provider
        .get_token_decimals(args.token, Some(audit_block))
        .await?;
    let metadata =
        token_metadata(provider.as_ref(), args.token, audit_block, token_decimals).await?;
    let builder = TokenStateBuilder::new(metadata, HISTORY_LIMIT);
    let mut token = builder
        .build_from_processed_blocks(active_processed_blocks.clone())
        .wrap_err("failed to build token state from active blocks")?;
    let pool = token
        .pool_base(addr(args.pool))
        .ok_or_else(|| eyre!("pool {} missing after token replay", addr(args.pool)))?;
    let mark_price = (pool.price().is_finite() && pool.price() > 0.0).then_some(pool.price());
    let pnl_positions = token
        .pnl
        .pool(addr(args.pool))
        .map(|pool| {
            pool.export(Some("uniswap_v2"), mark_price)
                .address_positions
                .into_iter()
                .map(|position| (normalize(&position.address), position))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let all_active_txs = active_processed_blocks
        .iter()
        .flat_map(|block| block.transactions.iter().map(|tx| tx.processed.clone()))
        .collect::<Vec<_>>();
    let moves = collect_token_moves(&all_active_txs, args.token, token_decimals);
    let mut accumulators = collect_buyer_accumulators(&moves, args.pool, args.token);

    let reader = ProviderBalanceReader {
        provider: provider.clone(),
        token: args.token,
        decimals: token_decimals,
    };
    let config = ReconcileConfig {
        min_expected_balance: MIN_BUYER_BALANCE,
        min_drop_fraction: MIN_DRAIN_FRACTION,
    };
    let ledger = HolderBalanceLedger::from_token_through_block(&token, audit_block);
    let _all_victims = reconcile_custody_drain(&ledger, &reader, audit_block, &config).await?;

    let mut fund_blocks = BTreeSet::<u64>::new();
    let mut buyer_addresses = BTreeSet::<String>::new();
    for buyer in accumulators.keys() {
        buyer_addresses.insert(buyer.clone());
    }
    let relevant_addresses = relevant_addresses(&buyer_addresses, &args);
    let fund_start = start_block.saturating_sub(args.lookback_blocks);
    let fund_end = audit_block
        .saturating_add(args.lookahead_blocks)
        .min(latest);
    for address in &relevant_addresses {
        if let Ok(mut blocks) = provider.get_address_participation_blocks(parse_addr(address)?) {
            blocks.retain(|block| *block >= fund_start && *block <= fund_end);
            fund_blocks.extend(blocks);
        }
    }
    for block_number in &active_blocks {
        fund_blocks.insert(*block_number);
    }
    let mut fund_processed = 0usize;
    let fund_total = fund_blocks.len();
    for block_number in fund_blocks.iter().copied() {
        if processed_blocks.contains_key(&block_number) {
            continue;
        }
        let include_traces = args.trace_all_blocks || block_number == args.drain_block;
        let block = tx_processor
            .process_block_with_options(block_number, include_traces)
            .await
            .wrap_err_with(|| format!("failed to process fund-flow block {block_number}"))?;
        processed_blocks.insert(block_number, block);
        fund_processed += 1;
        if fund_processed % 25 == 0 {
            println!("processed additional fund-flow blocks: {fund_processed}/{fund_total}");
        }
    }

    let fund_txs = processed_blocks
        .values()
        .flat_map(|block| block.transactions.iter().map(|tx| tx.processed.clone()))
        .filter(|tx| tx_involves_any(tx, &relevant_addresses))
        .collect::<Vec<_>>();
    let fund_edges = collect_fund_flow_edges(&fund_txs, &relevant_addresses, &args)?;
    let funding_by_buyer = nearest_pre_buy_funding(&fund_edges, &accumulators);
    let clusters = build_clusters(&fund_edges, &buyer_addresses, &args);

    let mut victims = Vec::<CustodyDrainVictim>::new();
    let mut reconciliation_contexts = HashMap::<String, serde_json::Value>::new();
    let mut rows = Vec::<BuyerOutcomeRow>::new();
    for (buyer, accumulator) in &mut accumulators {
        let expected = ledger.expected_balance(buyer);
        let actual = reader.balance_of(buyer, audit_block).await?;
        let missing = (expected - actual).max(0.0);
        let drained_fraction = if expected > 0.0 {
            (missing / expected).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if expected >= MIN_BUYER_BALANCE && drained_fraction >= MIN_DRAIN_FRACTION {
            victims.push(CustodyDrainVictim {
                holder: buyer.clone(),
                block_number: audit_block,
                expected_balance: expected,
                actual_balance: actual,
                missing_balance: missing,
                drained_fraction,
                expected_supply_share: ledger.share_of_supply(buyer),
            });
            reconciliation_contexts.insert(
                buyer.clone(),
                serde_json::json!({
                    "candidate_reason": "session_case_buyer_outcome_report",
                    "bought_amount": accumulator.buy_amount,
                    "normal_out_to_pool": accumulator.emitted_out_to_pool,
                    "normal_out_to_other": accumulator.emitted_out_to_other,
                    "normal_out_to_burn": accumulator.emitted_out_to_burn,
                    "eventless_out_to_pool": accumulator.eventless_out_to_pool,
                    "eventless_out_to_other": accumulator.eventless_out_to_other,
                    "eventless_out_to_burn": accumulator.eventless_out_to_burn,
                    "audit_block": audit_block,
                }),
            );
        }
        let pnl = pnl_positions.get(buyer);
        let funding = funding_by_buyer.get(buyer);
        let connection = creator_connection(buyer, funding, &fund_edges, &clusters, &args);
        rows.push(BuyerOutcomeRow {
            buyer: buyer.clone(),
            classification: classify_buyer(accumulator, expected, actual, drained_fraction),
            evidence_level: evidence_level(expected, actual, drained_fraction).to_string(),
            bought_amount: accumulator.buy_amount,
            buy_count: accumulator.buy_count,
            first_buy_block: accumulator.first_buy_block,
            first_buy_tx: accumulator.first_buy_tx.clone(),
            last_buy_block: accumulator.last_buy_block,
            last_buy_tx: accumulator.last_buy_tx.clone(),
            normal_out_to_pool: accumulator.emitted_out_to_pool,
            normal_out_to_other: accumulator.emitted_out_to_other,
            normal_out_to_burn: accumulator.emitted_out_to_burn,
            eventless_out_to_pool: accumulator.eventless_out_to_pool,
            eventless_out_to_other: accumulator.eventless_out_to_other,
            eventless_out_to_burn: accumulator.eventless_out_to_burn,
            eventless_in: accumulator.eventless_in,
            expected_balance: expected,
            actual_balance: actual,
            missing_balance: missing,
            drained_fraction,
            token_balance_from_pnl: pnl.map(|p| p.token_balance.to_string()),
            denom_cashflow_eth: pnl.map(|p| p.denom_cashflow.to_string()),
            native_fee_eth: pnl.map(|p| p.native_fee.to_string()),
            pnl_proxy_eth: pnl.and_then(|p| p.pnl_proxy_denom.map(|value| value.to_string())),
            nearest_pre_buy_funder: funding.map(|f| f.from_address.clone()),
            nearest_pre_buy_funding_tx: funding.map(|f| f.tx_hash.clone()),
            nearest_pre_buy_funding_block: funding.map(|f| f.block_number),
            nearest_pre_buy_funding_eth: funding.map(|f| f.amount_eth),
            creator_connection: connection.connection,
            connection_confidence: connection.confidence,
            connection_evidence: connection.evidence,
        });
    }
    let reconciled_added =
        token.record_reconciled_holder_confiscations(&victims, Some(&reconciliation_contexts));

    rows.sort_by(|left, right| {
        rank_classification(&left.classification)
            .cmp(&rank_classification(&right.classification))
            .then_with(|| {
                right
                    .bought_amount
                    .partial_cmp(&left.bought_amount)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let summary = summarize(
        &rows,
        &clusters,
        &args,
        start_block,
        audit_block,
        active_blocks.len(),
        reconciled_added,
        token.custody_findings().len(),
    );
    let artifact = BuyerOutcomeArtifact {
        summary,
        buyers: rows,
    };

    write_case_outputs(
        &args.case_dir,
        &artifact,
        &fund_edges,
        &clusters,
        &pnl_positions,
    )?;
    println!(
        "wrote buyer outcomes: buyers={} confiscated={} clusters={} fund_edges={}",
        artifact.summary.buyer_count,
        artifact.summary.confiscated_count,
        clusters.len(),
        fund_edges.len()
    );
    println!("audit_block: {}", artifact.summary.audit_block);
    println!("case: {}", args.case_dir.display());

    active_blocks.clear();
    Ok(())
}

fn parse_args() -> Result<Args> {
    let datadir =
        std::env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string());
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
        start_block: env_u64("SESSION_CASE_START_BLOCK", DEFAULT_START_BLOCK),
        end_block: std::env::var("SESSION_CASE_END_BLOCK")
            .ok()
            .and_then(|value| value.parse().ok()),
        drain_block: env_u64("SESSION_CASE_DRAIN_BLOCK", DEFAULT_DRAIN_BLOCK),
        trace_all_blocks: std::env::var("SESSION_CASE_TRACE_ALL")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false),
        lookback_blocks: env_u64("SESSION_CASE_LOOKBACK_BLOCKS", DEFAULT_LOOKBACK_BLOCKS),
        lookahead_blocks: env_u64("SESSION_CASE_LOOKAHEAD_BLOCKS", DEFAULT_LOOKAHEAD_BLOCKS),
    })
}

fn parse_env_addr(name: &str, default: &str) -> Result<Address> {
    let value = std::env::var(name).unwrap_or_else(|_| default.to_string());
    parse_addr(&value)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
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

fn collect_token_moves(
    txs: &[ProcessedTransaction],
    token_address: Address,
    token_decimals: u8,
) -> Vec<TokenMove> {
    let mut moves = Vec::new();
    for tx in txs {
        for event in &tx.erc20_transfers {
            if event.token_address != token_address {
                continue;
            }
            moves.push(TokenMove {
                tx_hash: hash(tx.hash),
                block_number: tx.block_number,
                tx_index: tx.tx_index,
                from_address: addr(event.from_address),
                to_address: addr(event.to_address),
                amount_scaled: scale(event.amount, token_decimals),
                source: "event",
            });
        }
        for transfer in &tx.internal_erc20_transfers {
            if transfer.token_address != token_address {
                continue;
            }
            moves.push(TokenMove {
                tx_hash: hash(tx.hash),
                block_number: tx.block_number,
                tx_index: tx.tx_index,
                from_address: addr(transfer.from_address),
                to_address: addr(transfer.to_address),
                amount_scaled: scale(transfer.amount, token_decimals),
                source: "eventless_internal",
            });
        }
    }
    moves.sort_by_key(|m| (m.block_number, m.tx_index));
    moves
}

fn collect_buyer_accumulators(
    moves: &[TokenMove],
    pool_address: Address,
    token_address: Address,
) -> BTreeMap<String, BuyerAccumulator> {
    let pool = normalize(&addr(pool_address));
    let token = normalize(&addr(token_address));
    let mut buyers = BTreeMap::<String, BuyerAccumulator>::new();
    for movement in moves {
        if movement.source != "event" {
            continue;
        }
        let from = normalize(&movement.from_address);
        let to = normalize(&movement.to_address);
        if from == pool && to != pool && to != token && !is_burn_address_str(&to) {
            let buyer = buyers
                .entry(to.clone())
                .or_insert_with(|| BuyerAccumulator {
                    ..BuyerAccumulator::default()
                });
            buyer.buy_amount += movement.amount_scaled;
            buyer.buy_count = buyer.buy_count.saturating_add(1);
            if buyer.first_buy_block.is_none() {
                buyer.first_buy_block = Some(movement.block_number);
                buyer.first_buy_tx = Some(movement.tx_hash.clone());
            }
            buyer.last_buy_block = Some(movement.block_number);
            buyer.last_buy_tx = Some(movement.tx_hash.clone());
        }
    }
    for movement in moves {
        let from = normalize(&movement.from_address);
        let to = normalize(&movement.to_address);
        let Some(buyer) = buyers.get_mut(&from) else {
            if let Some(buyer) = buyers.get_mut(&to) {
                if movement.source == "eventless_internal" {
                    buyer.eventless_in += movement.amount_scaled;
                }
            }
            continue;
        };
        let is_eventless = movement.source == "eventless_internal";
        if to == pool {
            if is_eventless {
                buyer.eventless_out_to_pool += movement.amount_scaled;
            } else {
                buyer.emitted_out_to_pool += movement.amount_scaled;
                buyer.normal_sell_count = buyer.normal_sell_count.saturating_add(1);
            }
        } else if is_burn_address_str(&to) {
            if is_eventless {
                buyer.eventless_out_to_burn += movement.amount_scaled;
            } else {
                buyer.emitted_out_to_burn += movement.amount_scaled;
            }
        } else if is_eventless {
            buyer.eventless_out_to_other += movement.amount_scaled;
        } else {
            buyer.emitted_out_to_other += movement.amount_scaled;
        }
    }
    buyers
}

fn relevant_addresses(buyers: &BTreeSet<String>, args: &Args) -> BTreeSet<String> {
    let mut addresses = buyers.clone();
    for address in [
        args.suspect,
        args.control,
        args.vault,
        args.pool,
        args.token,
    ] {
        addresses.insert(normalize(&addr(address)));
    }
    addresses
}

fn tx_involves_any(tx: &ProcessedTransaction, addresses: &BTreeSet<String>) -> bool {
    if addresses.contains(&normalize(&addr(tx.from_address))) {
        return true;
    }
    if tx
        .to_address
        .map(|address| addresses.contains(&normalize(&addr(address))))
        .unwrap_or(false)
    {
        return true;
    }
    tx.unique_addresses
        .iter()
        .any(|address| addresses.contains(&normalize(&addr(*address))))
}

fn collect_fund_flow_edges(
    txs: &[ProcessedTransaction],
    relevant_addresses: &BTreeSet<String>,
    args: &Args,
) -> Result<Vec<FundFlowEdge>> {
    let mut edges = Vec::new();
    let mut seen = BTreeSet::<String>::new();
    for tx in txs {
        let flows = extract_fund_flows_from_processed_tx(tx)?;
        for movement in flows.eth_movements {
            if movement.amount.is_zero() {
                continue;
            }
            let from = normalize(&addr(movement.from));
            let to = normalize(&addr(movement.to));
            if !relevant_addresses.contains(&from) && !relevant_addresses.contains(&to) {
                continue;
            }
            let movement_type = format!("{:?}", movement.movement_type);
            let key = format!(
                "{}:{}:{}:{}:{}",
                hash(tx.hash),
                from,
                to,
                movement.amount,
                movement_type
            );
            if !seen.insert(key) {
                continue;
            }
            edges.push(FundFlowEdge {
                tx_hash: hash(tx.hash),
                block_number: tx.block_number,
                block_timestamp: tx.block_timestamp,
                from_role: role_for(&from, args),
                to_role: role_for(&to, args),
                from_address: from,
                to_address: to,
                amount_wei: movement.amount.to_string(),
                amount_eth: scale(movement.amount, 18),
                movement_type,
            });
        }
    }
    edges.sort_by_key(|edge| (edge.block_number, edge.tx_hash.clone()));
    Ok(edges)
}

fn nearest_pre_buy_funding(
    edges: &[FundFlowEdge],
    buyers: &BTreeMap<String, BuyerAccumulator>,
) -> BTreeMap<String, FundingObservation> {
    let mut by_buyer = BTreeMap::new();
    for (buyer, accumulator) in buyers {
        let Some(first_buy_block) = accumulator.first_buy_block else {
            continue;
        };
        let mut candidates = edges
            .iter()
            .filter(|edge| edge.to_address == *buyer)
            .filter(|edge| edge.from_address != *buyer)
            .filter(|edge| edge.block_number <= first_buy_block)
            .filter(|edge| edge.movement_type == "Direct" || edge.movement_type == "Internal")
            .collect::<Vec<_>>();
        candidates.sort_by_key(|edge| (edge.block_number, edge.tx_hash.clone()));
        if let Some(edge) = candidates.last() {
            by_buyer.insert(
                buyer.clone(),
                FundingObservation {
                    tx_hash: edge.tx_hash.clone(),
                    block_number: edge.block_number,
                    block_timestamp: edge.block_timestamp,
                    from_address: edge.from_address.clone(),
                    to_address: edge.to_address.clone(),
                    amount_eth: edge.amount_eth,
                    movement_type: edge.movement_type.clone(),
                },
            );
        }
    }
    by_buyer
}

fn build_clusters(
    edges: &[FundFlowEdge],
    buyers: &BTreeSet<String>,
    args: &Args,
) -> Vec<AddressCluster> {
    let mut funded_by = BTreeMap::<String, BTreeSet<String>>::new();
    for edge in edges {
        if !buyers.contains(&edge.to_address)
            || edge.from_address == edge.to_address
            || is_noisy_hub(&edge.from_address, args)
        {
            continue;
        }
        if edge.movement_type != "Direct" && edge.movement_type != "Internal" {
            continue;
        }
        funded_by
            .entry(edge.from_address.clone())
            .or_default()
            .insert(edge.to_address.clone());
    }

    let mut clusters = Vec::new();
    for (funder, members) in funded_by {
        if members.len() < 2
            && !same(&funder, &addr(args.suspect))
            && !same(&funder, &addr(args.control))
        {
            continue;
        }
        let confidence = if same(&funder, &addr(args.suspect)) || same(&funder, &addr(args.control))
        {
            "direct"
        } else if members.len() >= 3 {
            "strong"
        } else {
            "weak"
        };
        clusters.push(AddressCluster {
            cluster_id: format!("shared_funder_{}", short_addr(&funder)),
            cluster_type: "shared_funder".to_string(),
            confidence: confidence.to_string(),
            hub_address: funder.clone(),
            members: members.iter().cloned().collect(),
            evidence: vec![format!(
                "{} funded {} buyer address(es) in the token investigation window",
                funder,
                members.len()
            )],
        });
    }
    clusters.sort_by(|left, right| {
        right
            .members
            .len()
            .cmp(&left.members.len())
            .then_with(|| left.hub_address.cmp(&right.hub_address))
    });
    clusters
}

struct ConnectionVerdict {
    connection: String,
    confidence: String,
    evidence: Vec<String>,
}

fn creator_connection(
    buyer: &str,
    funding: Option<&FundingObservation>,
    edges: &[FundFlowEdge],
    clusters: &[AddressCluster],
    args: &Args,
) -> ConnectionVerdict {
    let suspect = normalize(&addr(args.suspect));
    let control = normalize(&addr(args.control));
    if same(buyer, &suspect) {
        return ConnectionVerdict {
            connection: "buyer_is_suspect".to_string(),
            confidence: "direct".to_string(),
            evidence: vec!["buyer address is the configured suspect address".to_string()],
        };
    }
    if same(buyer, &control) {
        return ConnectionVerdict {
            connection: "buyer_is_control_contract".to_string(),
            confidence: "direct".to_string(),
            evidence: vec!["buyer address is the configured control contract".to_string()],
        };
    }
    if let Some(funding) = funding {
        if same(&funding.from_address, &suspect) || same(&funding.from_address, &control) {
            return ConnectionVerdict {
                connection: "creator_or_control_funded_buyer".to_string(),
                confidence: "direct".to_string(),
                evidence: vec![format!(
                    "{} funded buyer before first buy in tx {} at block {}",
                    funding.from_address, funding.tx_hash, funding.block_number
                )],
            };
        }
    }
    let direct_post_flow = edges.iter().find(|edge| {
        same(&edge.from_address, buyer)
            && (same(&edge.to_address, &suspect) || same(&edge.to_address, &control))
            && edge.movement_type != "Gas"
    });
    if let Some(edge) = direct_post_flow {
        return ConnectionVerdict {
            connection: "buyer_sent_eth_to_creator_or_control".to_string(),
            confidence: "direct".to_string(),
            evidence: vec![format!(
                "buyer sent {:.9} ETH to {} in tx {} at block {}",
                edge.amount_eth, edge.to_address, edge.tx_hash, edge.block_number
            )],
        };
    }
    for cluster in clusters {
        if cluster.members.iter().any(|member| same(member, buyer)) {
            return ConnectionVerdict {
                connection: "shared_funder_cluster".to_string(),
                confidence: cluster.confidence.clone(),
                evidence: vec![format!(
                    "buyer shares funder {} with {} buyer address(es)",
                    cluster.hub_address,
                    cluster.members.len()
                )],
            };
        }
    }
    ConnectionVerdict {
        connection: "no_direct_creator_link_in_window".to_string(),
        confidence: "none".to_string(),
        evidence: Vec::new(),
    }
}

fn classify_buyer(
    row: &BuyerAccumulator,
    expected: f64,
    actual: f64,
    drained_fraction: f64,
) -> String {
    if expected >= MIN_BUYER_BALANCE && drained_fraction >= MIN_DRAIN_FRACTION {
        return "confiscated".to_string();
    }
    let total_normal_out =
        row.emitted_out_to_pool + row.emitted_out_to_other + row.emitted_out_to_burn;
    if row.emitted_out_to_pool >= row.buy_amount * 0.98 && expected < MIN_BUYER_BALANCE {
        return "sold_normally".to_string();
    }
    if total_normal_out >= row.buy_amount * 0.98 && expected < MIN_BUYER_BALANCE {
        return "transferred_or_burned_normally".to_string();
    }
    if row.emitted_out_to_pool > 0.0 && actual > 0.0 {
        return "partial_exit_still_holding".to_string();
    }
    if actual > 0.0 && expected >= MIN_BUYER_BALANCE {
        return "still_holding_at_audit".to_string();
    }
    "unknown_or_dust".to_string()
}

fn evidence_level(expected: f64, actual: f64, drained_fraction: f64) -> &'static str {
    if expected >= MIN_BUYER_BALANCE && drained_fraction >= MIN_DRAIN_FRACTION {
        "state_reconciliation"
    } else if expected < MIN_BUYER_BALANCE && actual <= MIN_BUYER_BALANCE {
        "event_ledger"
    } else {
        "snapshot"
    }
}

fn summarize(
    rows: &[BuyerOutcomeRow],
    clusters: &[AddressCluster],
    args: &Args,
    start_block: u64,
    audit_block: u64,
    active_block_count: usize,
    reconciled_added: usize,
    total_custody_findings: usize,
) -> BuyerOutcomeSummary {
    let count = |prefix: &str| {
        rows.iter()
            .filter(|row| row.classification.starts_with(prefix))
            .count()
    };
    BuyerOutcomeSummary {
        token_address: addr(args.token),
        pool_address: addr(args.pool),
        suspect_address: addr(args.suspect),
        control_address: addr(args.control),
        vault_address: addr(args.vault),
        start_block,
        audit_block,
        active_block_count,
        buyer_count: rows.len(),
        confiscated_count: count("confiscated"),
        sold_normally_count: count("sold_normally"),
        partial_exit_count: count("partial_exit"),
        still_holding_count: count("still_holding"),
        transferred_or_burned_count: count("transferred_or_burned"),
        unknown_count: count("unknown"),
        direct_creator_connection_count: rows
            .iter()
            .filter(|row| row.connection_confidence == "direct")
            .count(),
        shared_funder_cluster_count: clusters.len(),
        reconciled_custody_findings_added: reconciled_added,
        total_custody_findings_after_reconciliation: total_custody_findings,
    }
}

fn write_case_outputs(
    case_dir: &Path,
    artifact: &BuyerOutcomeArtifact,
    fund_edges: &[FundFlowEdge],
    clusters: &[AddressCluster],
    pnl_positions: &BTreeMap<String, PnlAddressPositionExport>,
) -> Result<()> {
    let artifacts_dir = case_dir.join("artifacts");
    let tx_dir = artifacts_dir.join("tx_fund_flow");
    let reports_dir = case_dir.join("reports");
    fs::create_dir_all(&artifacts_dir)?;
    fs::create_dir_all(&tx_dir)?;
    fs::create_dir_all(&reports_dir)?;

    write_json(artifacts_dir.join("buyer_token_outcomes.json"), artifact)?;
    write_buyer_outcomes_csv(
        artifacts_dir.join("buyer_token_outcomes.csv"),
        &artifact.buyers,
    )?;
    write_buyer_pnl_csv(artifacts_dir.join("buyer_pnl.csv"), pnl_positions)?;
    write_json(tx_dir.join("buyer_eth_edges.json"), fund_edges)?;
    write_fund_edges_csv(tx_dir.join("buyer_eth_edges.csv"), fund_edges)?;
    write_json(tx_dir.join("address_clusters.json"), clusters)?;
    write_buyer_report(
        reports_dir.join("buyer_outcome_report.md"),
        artifact,
        clusters,
    )?;
    Ok(())
}

fn write_json<T: Serialize + ?Sized>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let path = path.as_ref();
    let contents = serde_json::to_string_pretty(value)?;
    fs::write(path, contents).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn write_buyer_outcomes_csv(path: impl AsRef<Path>, rows: &[BuyerOutcomeRow]) -> Result<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "buyer,classification,evidence_level,bought_amount,first_buy_block,first_buy_tx,normal_out_to_pool,normal_out_to_other,normal_out_to_burn,eventless_out_to_burn,expected_balance,actual_balance,missing_balance,drained_fraction,nearest_pre_buy_funder,nearest_pre_buy_funding_tx,nearest_pre_buy_funding_eth,creator_connection,connection_confidence")?;
    for row in rows {
        writeln!(
            writer,
            "{},{},{},{:.9},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{},{},{},{}",
            row.buyer,
            csv(&row.classification),
            csv(&row.evidence_level),
            row.bought_amount,
            opt_u64(row.first_buy_block),
            csv(row.first_buy_tx.as_deref().unwrap_or("")),
            row.normal_out_to_pool,
            row.normal_out_to_other,
            row.normal_out_to_burn,
            row.eventless_out_to_burn,
            row.expected_balance,
            row.actual_balance,
            row.missing_balance,
            row.drained_fraction,
            csv(row.nearest_pre_buy_funder.as_deref().unwrap_or("")),
            csv(row.nearest_pre_buy_funding_tx.as_deref().unwrap_or("")),
            row.nearest_pre_buy_funding_eth
                .map(|v| format!("{v:.18}"))
                .unwrap_or_default(),
            csv(&row.creator_connection),
            csv(&row.connection_confidence),
        )?;
    }
    Ok(())
}

fn write_buyer_pnl_csv(
    path: impl AsRef<Path>,
    positions: &BTreeMap<String, PnlAddressPositionExport>,
) -> Result<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "address,token_balance,denom_cashflow,native_fee,native_priority_fee,pnl_proxy,first_block,latest_block,movement_count")?;
    for position in positions.values() {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{}",
            normalize(&position.address),
            position.token_balance,
            position.denom_cashflow,
            position.native_fee,
            position.native_priority_fee,
            position
                .pnl_proxy_denom
                .map(|v| v.to_string())
                .unwrap_or_default(),
            opt_u64(position.first_block),
            opt_u64(position.latest_block),
            position.movement_count,
        )?;
    }
    Ok(())
}

fn write_fund_edges_csv(path: impl AsRef<Path>, edges: &[FundFlowEdge]) -> Result<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "tx_hash,block_number,block_timestamp,from_address,to_address,amount_eth,movement_type,from_role,to_role")?;
    for edge in edges {
        writeln!(
            writer,
            "{},{},{},{},{},{:.18},{},{},{}",
            edge.tx_hash,
            edge.block_number,
            edge.block_timestamp,
            edge.from_address,
            edge.to_address,
            edge.amount_eth,
            edge.movement_type,
            csv(&edge.from_role),
            csv(&edge.to_role),
        )?;
    }
    Ok(())
}

fn write_buyer_report(
    path: impl AsRef<Path>,
    artifact: &BuyerOutcomeArtifact,
    clusters: &[AddressCluster],
) -> Result<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    let s = &artifact.summary;
    writeln!(writer, "# Session Buyer Outcome Report")?;
    writeln!(writer)?;
    writeln!(writer, "## Scope")?;
    writeln!(writer)?;
    writeln!(writer, "- token: `{}`", s.token_address)?;
    writeln!(writer, "- pool: `{}`", s.pool_address)?;
    writeln!(writer, "- suspect: `{}`", s.suspect_address)?;
    writeln!(writer, "- control contract: `{}`", s.control_address)?;
    writeln!(writer, "- audit block: `{}`", s.audit_block)?;
    writeln!(
        writer,
        "- active blocks replayed: `{}`",
        s.active_block_count
    )?;
    writeln!(writer)?;
    writeln!(writer, "## Buyer Outcome Summary")?;
    writeln!(writer)?;
    writeln!(writer, "- buyers: `{}`", s.buyer_count)?;
    writeln!(writer, "- confiscated: `{}`", s.confiscated_count)?;
    writeln!(writer, "- sold normally: `{}`", s.sold_normally_count)?;
    writeln!(
        writer,
        "- partial exit still holding: `{}`",
        s.partial_exit_count
    )?;
    writeln!(
        writer,
        "- still holding at audit: `{}`",
        s.still_holding_count
    )?;
    writeln!(
        writer,
        "- transferred or burned normally: `{}`",
        s.transferred_or_burned_count
    )?;
    writeln!(
        writer,
        "- direct creator/control connections found: `{}`",
        s.direct_creator_connection_count
    )?;
    writeln!(
        writer,
        "- shared-funder clusters: `{}`",
        s.shared_funder_cluster_count
    )?;
    writeln!(writer)?;
    writeln!(writer, "## Interpretation")?;
    writeln!(writer)?;
    writeln!(writer, "The custody verdict is split by evidence type. `confiscated` means the emitted-transfer ledger expected a material buyer balance but on-chain `balanceOf` at the audit block was missing at least 90%. `sold_normally` and `partial_exit_still_holding` are ordinary emitted pool-out/pool-in paths. `still_holding_at_audit` means no state loss was observed through the latest indexed token activity block used by this report.")?;
    writeln!(writer)?;
    writeln!(writer, "Hidden creator linkage is conservative. `direct` requires the buyer itself, its nearest pre-buy funder, or a post-buy ETH flow to touch the suspect/control address directly. `weak` or `strong` shared-funder clusters are not attribution by themselves; they are follow-up leads.")?;
    writeln!(writer)?;
    writeln!(writer, "## Buyer Rows")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "| Buyer | Class | Bought | Expected | Actual | Missing | Funder | Link |"
    )?;
    writeln!(
        writer,
        "| --- | --- | ---: | ---: | ---: | ---: | --- | --- |"
    )?;
    for row in &artifact.buyers {
        writeln!(
            writer,
            "| `{}` | `{}` | `{:.4}` | `{:.4}` | `{:.4}` | `{:.4}` | `{}` | `{}` |",
            short_addr(&row.buyer),
            row.classification,
            row.bought_amount,
            row.expected_balance,
            row.actual_balance,
            row.missing_balance,
            row.nearest_pre_buy_funder
                .as_deref()
                .map(short_addr)
                .unwrap_or_else(|| "-".to_string()),
            row.creator_connection,
        )?;
    }
    writeln!(writer)?;
    writeln!(writer, "## Address Clusters")?;
    writeln!(writer)?;
    if clusters.is_empty() {
        writeln!(
            writer,
            "No multi-buyer shared-funder cluster was found in the configured window."
        )?;
    } else {
        writeln!(writer, "| Cluster | Confidence | Hub | Members |")?;
        writeln!(writer, "| --- | --- | --- | ---: |")?;
        for cluster in clusters {
            writeln!(
                writer,
                "| `{}` | `{}` | `{}` | `{}` |",
                cluster.cluster_id,
                cluster.confidence,
                short_addr(&cluster.hub_address),
                cluster.members.len(),
            )?;
        }
    }
    writeln!(writer)?;
    writeln!(writer, "## Artifacts")?;
    writeln!(writer)?;
    writeln!(writer, "- `artifacts/buyer_token_outcomes.json`")?;
    writeln!(writer, "- `artifacts/buyer_token_outcomes.csv`")?;
    writeln!(writer, "- `artifacts/buyer_pnl.csv`")?;
    writeln!(writer, "- `artifacts/tx_fund_flow/buyer_eth_edges.json`")?;
    writeln!(writer, "- `artifacts/tx_fund_flow/buyer_eth_edges.csv`")?;
    writeln!(writer, "- `artifacts/tx_fund_flow/address_clusters.json`")?;
    Ok(())
}

fn rank_classification(classification: &str) -> u8 {
    match classification {
        "confiscated" => 0,
        "partial_exit_still_holding" => 1,
        "still_holding_at_audit" => 2,
        "sold_normally" => 3,
        "transferred_or_burned_normally" => 4,
        _ => 5,
    }
}

fn is_noisy_hub(address: &str, args: &Args) -> bool {
    same(address, &addr(args.pool))
        || same(address, &addr(args.token))
        || address == "0x0000000000000000000000000000000000000000"
}

fn role_for(address: &str, args: &Args) -> String {
    if same(address, &addr(args.suspect)) {
        "suspect".to_string()
    } else if same(address, &addr(args.control)) {
        "control_contract".to_string()
    } else if same(address, &addr(args.vault)) {
        "victim_vault".to_string()
    } else if same(address, &addr(args.pool)) {
        "pool".to_string()
    } else if same(address, &addr(args.token)) {
        "token".to_string()
    } else {
        "address".to_string()
    }
}

fn parse_addr(value: &str) -> Result<Address> {
    Address::from_str(value.trim()).map_err(|error| eyre!("invalid address {value}: {error}"))
}

fn addr(address: Address) -> String {
    to_checksum_address(&address)
}

fn hash(hash: alloy_primitives::B256) -> String {
    format!("{hash:?}")
}

fn normalize(address: &str) -> String {
    address.trim().to_ascii_lowercase()
}

fn same(left: &str, right: &str) -> bool {
    normalize(left) == normalize(right)
}

fn scale(raw: U256, decimals: u8) -> f64 {
    let divisor = 10f64.powi(decimals as i32);
    raw.to_string().parse::<f64>().unwrap_or(0.0) / divisor
}

fn csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn opt_u64(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn short_addr(value: &str) -> String {
    if value.len() <= 14 {
        value.to_string()
    } else {
        format!("{}...{}", &value[..8], &value[value.len() - 6..])
    }
}
