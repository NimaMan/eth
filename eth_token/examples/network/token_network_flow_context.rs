use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::Address;
use eth_token::chain_metadata::RethChainMetadataProvider;
use eth_token::erc20::ERC20TokenMetadata;
use eth_token::network::flow_context::block_loader::ProcessedBlockLoader;
use eth_token::network::flow_context::index_query::AddressParticipationQuery;
use eth_token::network::flow_context::model::BlockRange;
use eth_token::network::flow_context::windows::build_flow_context_windows;
use eth_token::network::flow_context::{
    select_flow_context_seeds, FlowContextBuilder, FlowContextConfig,
    TxFundFlowObservationExtractor,
};
use eth_token::network::model::normalize_network_address;
use eth_token::tracking::{BlockTokenProcessor, TokenRegistry};
use eyre::{bail, eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, PoolBuySellSimulator, ProcessedBlock, ProcessedBlockProvider};

const DEFAULT_TOKEN: &str = "0x133a79c66bc8789cf4d081159654aa378004541c";
const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
const DEFAULT_HISTORY_LIMIT: usize = 1_000;
const DEFAULT_MAX_TOKEN_BLOCKS: usize = 256;
const DEFAULT_MAX_SEED_ADDRESSES: usize = 16;
const DEFAULT_MAX_BLOCKS_PER_ADDRESS: usize = 64;
const DEFAULT_LOOKBACK_BLOCKS: u64 = 300;
const DEFAULT_LOOKAHEAD_BLOCKS: u64 = 80;
const DEFAULT_TOP: usize = 20;

#[derive(Debug)]
struct Args {
    token: Address,
    datadir: String,
    reth_index: String,
    start_block: Option<u64>,
    end_block: Option<u64>,
    history_limit: usize,
    max_token_blocks: usize,
    max_seed_addresses: usize,
    max_blocks_per_address: usize,
    lookback_blocks: u64,
    lookahead_blocks: u64,
    top: usize,
    progress_every: usize,
}

#[derive(Clone)]
struct RethIndexAddressParticipationQuery {
    provider: Arc<RethQueryProvider>,
}

impl AddressParticipationQuery for RethIndexAddressParticipationQuery {
    fn blocks_for_address(&self, address: &str, range: BlockRange) -> Result<Vec<u64>> {
        let address = Address::from_str(address)
            .map_err(|error| eyre!("invalid indexed address {address}: {error}"))?;
        let mut blocks = self.provider.get_address_participation_blocks(address)?;
        blocks.retain(|block| range.contains(*block));
        blocks.sort_unstable();
        Ok(blocks)
    }
}

#[derive(Clone, Debug)]
struct PreloadedProcessedBlockLoader {
    blocks_by_number: BTreeMap<u64, ProcessedBlock>,
}

impl ProcessedBlockLoader for PreloadedProcessedBlockLoader {
    fn load_blocks(&self, block_numbers: &[u64]) -> Result<Vec<ProcessedBlock>> {
        block_numbers
            .iter()
            .map(|block_number| {
                self.blocks_by_number
                    .get(block_number)
                    .cloned()
                    .ok_or_else(|| eyre!("context block {block_number} was not preloaded"))
            })
            .collect()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let started = Instant::now();
    let provider =
        Arc::new(RethQueryProvider::new(&args.datadir)?.with_reth_index(&args.reth_index)?);
    let block_provider = ProcessedBlockProvider::new(
        BlockProcessor::new(provider.clone()),
        provider.clone(),
        None,
    );
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let index_query = RethIndexAddressParticipationQuery {
        provider: provider.clone(),
    };

    let latest = provider.get_latest_block()?;
    let end_block = args.end_block.unwrap_or(latest).min(latest);
    let start_block = args.start_block.unwrap_or(0).min(end_block);
    let block_range = BlockRange::new(start_block, end_block);
    let token_address = normalize_address(args.token);

    println!("Token network + flow-context validation");
    println!("token:          {token_address}");
    println!("datadir:        {}", args.datadir);
    println!("reth_index:     {}", args.reth_index);
    println!(
        "range:          {}..={}",
        block_range.start_block, block_range.end_block
    );
    println!("latest local:   {latest}");

    let metadata =
        load_token_metadata(provider.as_ref(), args.token, block_range.end_block).await?;
    println!(
        "metadata:       {} ({}) decimals={}",
        metadata.name, metadata.symbol, metadata.decimals
    );

    let token_blocks = token_participation_blocks(
        &index_query,
        &token_address,
        block_range,
        args.max_token_blocks,
    )?;
    if token_blocks.is_empty() {
        bail!(
            "no indexed token participation blocks found for {token_address}; populate reth_index/address_to_blocks first or adjust --start/--end"
        );
    }
    println!("token_blocks:   {} selected", token_blocks.len());

    let token_blocks_by_number =
        load_blocks(&block_provider, &token_blocks, args.progress_every).await?;
    let mut token_processor = token_processor(metadata, args.history_limit);
    for block in token_blocks_by_number.values() {
        token_processor
            .process_block_with_discovery_provider(block, &discovery_provider, &pool_simulator)
            .await;
    }

    let Some(graph) = token_processor.network_graphs.get(&token_address) else {
        bail!("token graph for {token_address} is empty after replaying selected blocks");
    };

    println!("graph_nodes:    {}", graph.nodes.len());
    println!("graph_edges:    {}", graph.edges.len());
    println!("graph_addresses: {}", graph.address_activity.len());

    let config = flow_context_config(&args);
    let context_blocks = context_block_numbers(graph, &config, &index_query)?;
    println!("context_blocks: {} selected", context_blocks.len());

    let mut all_blocks = token_blocks_by_number;
    let missing_context_blocks = context_blocks
        .iter()
        .copied()
        .filter(|block_number| !all_blocks.contains_key(block_number))
        .collect::<Vec<_>>();
    all_blocks.extend(
        load_blocks(
            &block_provider,
            &missing_context_blocks,
            args.progress_every,
        )
        .await?,
    );

    let flow_builder = FlowContextBuilder::new(
        config,
        index_query,
        PreloadedProcessedBlockLoader {
            blocks_by_number: all_blocks,
        },
        TxFundFlowObservationExtractor::default(),
    );
    let layer = flow_builder.build(graph)?;

    println!();
    println!("Flow Context");
    println!("seeds:          {}", layer.seed_count);
    println!("inspected_blocks: {}", layer.inspected_block_count);
    println!("nodes:          {}", layer.nodes.len());
    println!("edges:          {}", layer.edges.len());
    println!("clusters:       {}", layer.clusters.len());
    println!("suppressed_hubs: {}", layer.suppressed_hubs.len());
    println!("elapsed:        {:.3?}", started.elapsed());

    print_top_edges(&layer.edges, args.top);
    print_top_clusters(&layer.clusters, args.top);

    Ok(())
}

fn flow_context_config(args: &Args) -> FlowContextConfig {
    FlowContextConfig {
        max_seed_addresses: args.max_seed_addresses,
        lookback_blocks: args.lookback_blocks,
        lookahead_blocks: args.lookahead_blocks,
        max_blocks_per_address: args.max_blocks_per_address,
        ..FlowContextConfig::default()
    }
}

async fn load_token_metadata(
    provider: &RethQueryProvider,
    token: Address,
    block_number: u64,
) -> Result<ERC20TokenMetadata> {
    let metadata = provider
        .get_token_metadata(token, Some(block_number), None)
        .await?
        .ok_or_else(|| {
            eyre!(
                "token {:#x} did not return ERC20 metadata at block {block_number}",
                token
            )
        })?;

    Ok(ERC20TokenMetadata {
        address: normalize_address(metadata.address),
        name: metadata.name,
        symbol: metadata.symbol,
        decimals: metadata.decimals,
        total_supply: metadata.total_supply.to_string(),
    })
}

fn token_processor(metadata: ERC20TokenMetadata, history_limit: usize) -> BlockTokenProcessor {
    let mut registry = TokenRegistry::new();
    registry.add_token(metadata);
    BlockTokenProcessor::with_registry(registry, history_limit)
}

fn token_participation_blocks(
    index_query: &RethIndexAddressParticipationQuery,
    token_address: &str,
    range: BlockRange,
    max_blocks: usize,
) -> Result<Vec<u64>> {
    let mut blocks = index_query.blocks_for_address(token_address, range)?;
    blocks.truncate(max_blocks);
    Ok(blocks)
}

fn context_block_numbers(
    graph: &eth_token::network::graph::RawTokenNetworkGraph,
    config: &FlowContextConfig,
    index_query: &RethIndexAddressParticipationQuery,
) -> Result<Vec<u64>> {
    let seeds = select_flow_context_seeds(graph, config);
    let windows = build_flow_context_windows(graph, &seeds, config);
    let mut selected = BTreeSet::new();

    for window in windows {
        let mut blocks = index_query.blocks_for_address(&window.address, window.range)?;
        blocks.truncate(config.max_blocks_per_address);
        selected.extend(blocks);
    }

    Ok(selected.into_iter().collect())
}

async fn load_blocks(
    block_provider: &ProcessedBlockProvider,
    block_numbers: &[u64],
    progress_every: usize,
) -> Result<BTreeMap<u64, ProcessedBlock>> {
    let mut blocks_by_number = BTreeMap::new();
    for (index, block_number) in block_numbers.iter().copied().enumerate() {
        let loaded = block_provider.load_block(block_number).await?;
        blocks_by_number.insert(loaded.block.header.number, loaded.block);
        if progress_every > 0
            && ((index + 1) == block_numbers.len() || (index + 1) % progress_every == 0)
        {
            println!(
                "loaded_blocks:  {:>5}/{} latest={} source={}",
                index + 1,
                block_numbers.len(),
                block_number,
                loaded.source,
            );
        }
    }
    Ok(blocks_by_number)
}

fn print_top_edges(edges: &[eth_token::network::flow_context::FlowContextEdge], top: usize) {
    if edges.is_empty() {
        return;
    }

    println!();
    println!("Top Flow Edges");
    for edge in edges.iter().take(top) {
        let evidence = edge.evidence.first();
        println!(
            "- {} {} -> {} weight={} confidence={:.2} total={:?} asset={:?} tx={:?}",
            edge.kind.stable_key(),
            edge.source,
            edge.target,
            edge.weight,
            edge.confidence,
            edge.total_scaled_amount,
            evidence.and_then(|evidence| evidence.symbol.as_deref()),
            evidence.and_then(|evidence| evidence.observation.tx_hash.as_deref()),
        );
    }
}

fn print_top_clusters(
    clusters: &[eth_token::network::flow_context::FlowContextCluster],
    top: usize,
) {
    if clusters.is_empty() {
        return;
    }

    println!();
    println!("Top Flow Clusters");
    for cluster in clusters.iter().take(top) {
        println!(
            "- {} connector={:?} members={} confidence={:.2}",
            cluster.kind.stable_key(),
            cluster.connector,
            cluster.members.len(),
            cluster.confidence,
        );
    }
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        token: Address::from_str(DEFAULT_TOKEN)?,
        datadir: env::var("RETH_DATADIR").unwrap_or_else(|_| DEFAULT_RETH_DATADIR.to_string()),
        reth_index: env::var("RETH_INDEX_DIR").unwrap_or_default(),
        start_block: None,
        end_block: None,
        history_limit: DEFAULT_HISTORY_LIMIT,
        max_token_blocks: DEFAULT_MAX_TOKEN_BLOCKS,
        max_seed_addresses: DEFAULT_MAX_SEED_ADDRESSES,
        max_blocks_per_address: DEFAULT_MAX_BLOCKS_PER_ADDRESS,
        lookback_blocks: DEFAULT_LOOKBACK_BLOCKS,
        lookahead_blocks: DEFAULT_LOOKAHEAD_BLOCKS,
        top: DEFAULT_TOP,
        progress_every: 25,
    };

    let mut raw_args = env::args().skip(1);
    while let Some(arg) = raw_args.next() {
        match arg.as_str() {
            "--token" => args.token = parse_address_arg(&mut raw_args, "--token")?,
            "--datadir" => args.datadir = parse_string_arg(&mut raw_args, "--datadir")?,
            "--reth-index" => args.reth_index = parse_string_arg(&mut raw_args, "--reth-index")?,
            "--start" => args.start_block = Some(parse_u64_arg(&mut raw_args, "--start")?),
            "--end" => args.end_block = Some(parse_u64_arg(&mut raw_args, "--end")?),
            "--history-limit" => {
                args.history_limit = parse_usize_arg(&mut raw_args, "--history-limit")?;
            }
            "--max-token-blocks" => {
                args.max_token_blocks = parse_usize_arg(&mut raw_args, "--max-token-blocks")?;
            }
            "--max-seeds" => {
                args.max_seed_addresses = parse_usize_arg(&mut raw_args, "--max-seeds")?;
            }
            "--max-blocks-per-address" => {
                args.max_blocks_per_address =
                    parse_usize_arg(&mut raw_args, "--max-blocks-per-address")?;
            }
            "--lookback" => args.lookback_blocks = parse_u64_arg(&mut raw_args, "--lookback")?,
            "--lookahead" => args.lookahead_blocks = parse_u64_arg(&mut raw_args, "--lookahead")?,
            "--top" => args.top = parse_usize_arg(&mut raw_args, "--top")?,
            "--progress-every" => {
                args.progress_every = parse_usize_arg(&mut raw_args, "--progress-every")?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other if !other.starts_with("--") => args.token = Address::from_str(other)?,
            other => bail!("unknown argument {other}; pass --help for usage"),
        }
    }

    if args.reth_index.is_empty() {
        args.reth_index = PathBuf::from(&args.datadir)
            .join("reth_index")
            .to_string_lossy()
            .to_string();
    }
    if args.max_token_blocks == 0 {
        bail!("--max-token-blocks must be greater than zero");
    }
    if args.max_seed_addresses == 0 {
        bail!("--max-seeds must be greater than zero");
    }
    if args.max_blocks_per_address == 0 {
        bail!("--max-blocks-per-address must be greater than zero");
    }

    Ok(args)
}

fn parse_address_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<Address> {
    Ok(Address::from_str(&parse_string_arg(args, name)?)?)
}

fn parse_string_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    args.next().ok_or_else(|| eyre!("{name} requires a value"))
}

fn parse_u64_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<u64> {
    Ok(parse_string_arg(args, name)?.parse()?)
}

fn parse_usize_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<usize> {
    Ok(parse_string_arg(args, name)?.parse()?)
}

fn normalize_address(address: Address) -> String {
    normalize_network_address(format!("{address:#x}"))
}

fn print_help() {
    println!(
        "Usage: cargo run -p eth_token --example token_network_flow_context -- [--token ADDRESS] [options]\n\
\n\
Options:\n\
  --token ADDRESS                 Token contract to analyze (default {DEFAULT_TOKEN})\n\
  --datadir PATH                  Reth datadir (default RETH_DATADIR or {DEFAULT_RETH_DATADIR})\n\
  --reth-index PATH               RethIndex path (default RETH_INDEX_DIR or <datadir>/reth_index)\n\
  --start BLOCK                   Inclusive lower block bound\n\
  --end BLOCK                     Inclusive upper block bound\n\
  --max-token-blocks N            Max indexed token blocks to replay (default {DEFAULT_MAX_TOKEN_BLOCKS})\n\
  --max-seeds N                   Max flow-context seed addresses (default {DEFAULT_MAX_SEED_ADDRESSES})\n\
  --max-blocks-per-address N      Max context blocks per seed/window (default {DEFAULT_MAX_BLOCKS_PER_ADDRESS})\n\
  --lookback BLOCKS               Context lookback around token activity (default {DEFAULT_LOOKBACK_BLOCKS})\n\
  --lookahead BLOCKS              Context lookahead around token activity (default {DEFAULT_LOOKAHEAD_BLOCKS})\n\
  --history-limit N               Token history limit (default {DEFAULT_HISTORY_LIMIT})\n\
  --top N                         Top edges/clusters to print (default {DEFAULT_TOP})\n\
  --progress-every N              Block-load progress interval; 0 disables progress (default 25)"
    );
}
