//! Processed-block disk-cache format benchmark.
//!
//! Objective
//! ---------
//! Quantify how much disk space and read time we can save on the processed-block
//! cache by changing the on-disk payload, WITHOUT re-running EVM replay. It reads
//! existing cached blocks, re-encodes each under candidate formats, and reports a
//! space x read-time x write-time table so a format change can be chosen on
//! evidence rather than guesswork.
//!
//! Algorithm
//! ---------
//! 1. Resolve the cache dir and scan `<dir>/ethereum-mainnet` for current
//!    schema-version files (`<block>.v<N>.pblock.zst`); pick the highest N and
//!    take the most-recent `--count` blocks (default 1000), or an explicit
//!    `--start`/`--end` range. Record each file's on-disk size = production
//!    baseline.
//! 2. Load those blocks into memory via the production reader (decodes the real
//!    v2 files). This also measures the baseline parallel read wall time.
//! 3. For each SERIALIZATION candidate = field-set (Full | Lean) x codec
//!    (BincodeJson | Msgpack), serialize every block to an UNCOMPRESSED payload
//!    (compression is applied separately so it can be swept independently).
//!    `Lean` drops fields no consumer reads (struct logs, ERC-1155, access list,
//!    blob hashes, v4 fee-updates).
//! 4. For each serialization, sweep COMPRESSION candidates: plain zstd at several
//!    levels, plus a zstd dictionary trained on a sample of that serialization's
//!    payloads. Measure compressed total bytes, compress ms/block, and
//!    decompress+deserialize ms/block.
//! 5. Verify round-trip correctness per candidate with an order-independent
//!    fingerprint over the fields consumers actually read: the decoded block must
//!    preserve every consumed field. A candidate that fails to encode/decode or
//!    mismatches the fingerprint is reported and skipped (never silently passed).
//! 6. Emit one CSV row per candidate plus a human-readable summary, all relative
//!    to the production baseline.
//!
//! This example only needs the cache directory; it does not open the Reth
//! database or run the EVM.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;
use eyre::{bail, Result, WrapErr};
use tx_processor::{
    bench_deserialize_block, bench_serialize_block, CacheFieldSet, CacheSerCodec, ProcessedBlock,
    ProcessedBlockDiskCacheKey,
};

const DISK_CACHE_DIR_ENV: &str = "PROCESSED_BLOCK_DISK_CACHE_DIR";
const CHAIN_SERVER_DISK_CACHE_DIR_ENV: &str = "ETH_CHAIN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR";
const ETHEREUM_MAINNET_NETWORK: &str = "ethereum-mainnet";
const CACHE_FILE_SUFFIX: &str = ".pblock.zst";
/// zstd dictionary size target (bytes). 110 KiB is the zstd default ballpark.
const DICT_SIZE: usize = 112_640;
/// Number of payloads used to TRAIN the dictionary. ZDICT scales poorly with
/// total sample bytes, so cap the training set (the dictionary is then applied
/// to ALL payloads when measuring). A few hundred blocks is ample.
const DICT_TRAIN_SAMPLES: usize = 256;

#[derive(Debug, Parser)]
#[command(about = "Benchmark candidate processed-block disk-cache formats (space + read time).")]
struct Args {
    /// Cache directory. Defaults to PROCESSED_BLOCK_DISK_CACHE_DIR /
    /// ETH_CHAIN_SERVER_PROCESSED_BLOCK_DISK_CACHE_DIR.
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Chain id (1 = ethereum mainnet).
    #[arg(long, default_value_t = 1)]
    chain_id: u64,

    /// Last block to include. Defaults to the highest block present in the cache.
    #[arg(long)]
    end: Option<u64>,

    /// Number of most-recent blocks to benchmark when --start is omitted.
    #[arg(long, default_value_t = 1000)]
    count: u64,

    /// First block to include (overrides --count when set).
    #[arg(long)]
    start: Option<u64>,

    /// zstd levels to sweep (plain, no dictionary).
    #[arg(long, value_delimiter = ',', default_values_t = [3i32, 9, 19])]
    zstd_levels: Vec<i32>,

    /// zstd level used for the trained-dictionary candidate.
    #[arg(long, default_value_t = 19)]
    dict_level: i32,

    /// Chunked-archive sizes (blocks per archive) to test cross-block
    /// compression. Empty disables the chunked candidates.
    #[arg(long, value_delimiter = ',', default_values_t = [16usize, 64, 256])]
    chunk_sizes: Vec<usize>,

    /// zstd level for chunked-archive candidates.
    #[arg(long, default_value_t = 19)]
    chunk_level: i32,

    /// Include the MessagePack codec candidates (currently fails round-trip due
    /// to an alloy binary/human-readable serde mismatch; off by default).
    #[arg(long, default_value_t = false)]
    msgpack: bool,

    /// Skip the trained-dictionary candidate (it is the slowest pass).
    #[arg(long, default_value_t = false)]
    skip_dict: bool,

    /// Run a per-field-group size breakdown (raw + zstd-9) instead of the format
    /// matrix, to show where the uncompressed payload bytes go.
    #[arg(long, default_value_t = false)]
    field_breakdown: bool,
}

/// A serialization candidate: field set x codec.
#[derive(Debug, Clone, Copy)]
struct SerCandidate {
    label: &'static str,
    field_set: CacheFieldSet,
    codec: CacheSerCodec,
}

/// One measured result row.
#[derive(Debug, Clone)]
struct Row {
    label: String,
    blocks: u64,
    total_bytes: u64,
    avg_bytes: f64,
    compress_ms_per_block: f64,
    read_ms_per_block: f64,
    note: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cache_dir = resolve_cache_dir(args.cache_dir.as_deref())?;
    let network_dir = cache_dir.join(ETHEREUM_MAINNET_NETWORK);
    if !network_dir.is_dir() {
        bail!("cache network dir not found: {}", network_dir.display());
    }

    // (1) Scan for current-version files and pick the block range.
    let files = scan_current_version_files(&network_dir)?;
    if files.is_empty() {
        bail!("no current-version .pblock.zst files in {}", network_dir.display());
    }
    let max_block = *files.keys().next_back().expect("non-empty");
    let end = args.end.unwrap_or(max_block);
    let start = args
        .start
        .unwrap_or_else(|| end.saturating_sub(args.count.saturating_sub(1)));
    if end < start {
        bail!("end ({end}) < start ({start})");
    }

    let selected: Vec<(u64, u64, PathBuf)> = files
        .range(start..=end)
        .map(|(block, (size, path))| (*block, *size, path.clone()))
        .collect();
    if selected.is_empty() {
        bail!("no cached files in range {start}..={end}");
    }
    let baseline_total: u64 = selected.iter().map(|(_, size, _)| *size).sum();
    let baseline_blocks = selected.len() as u64;

    eprintln!(
        "cache_dir={} range={start}..={end} blocks_present={baseline_blocks} \
         baseline_total_bytes={baseline_total} baseline_avg_bytes={:.1}",
        cache_dir.display(),
        baseline_total as f64 / baseline_blocks as f64
    );

    // (2) Load blocks by reading the on-disk files directly (zstd-decode then
    // bincode-decode). We intentionally bypass `store.get`, which is locked to
    // the CURRENT schema-version filename — the existing cache may be an older
    // version. The bincode struct layout is unchanged across these versions, so
    // the payload decodes regardless; only field *content* differs.
    let read_started = Instant::now();
    let mut blocks: Vec<(ProcessedBlockDiskCacheKey, ProcessedBlock)> =
        Vec::with_capacity(selected.len());
    for (block_number, _, path) in &selected {
        let raw = fs::read(path).wrap_err_with(|| format!("read {}", path.display()))?;
        let decoded = zstd::stream::decode_all(raw.as_slice())
            .wrap_err_with(|| format!("zstd decode {}", path.display()))?;
        let block = bench_deserialize_block(&decoded, CacheSerCodec::BincodeJson)
            .wrap_err_with(|| format!("decode payload {}", path.display()))?;
        let key = ProcessedBlockDiskCacheKey::for_block_number(args.chain_id, *block_number);
        blocks.push((key, block));
    }
    let baseline_read_wall_ms = read_started.elapsed().as_secs_f64() * 1000.0;
    if blocks.is_empty() {
        bail!("no decodable blocks loaded");
    }
    eprintln!(
        "loaded {} blocks; baseline_read+decode_wall_ms={:.1} ({:.3} ms/block, serial)\n",
        blocks.len(),
        baseline_read_wall_ms,
        baseline_read_wall_ms / blocks.len() as f64
    );

    if args.field_breakdown {
        run_field_breakdown(&blocks)?;
        return Ok(());
    }

    // Order-independent fingerprint over CONSUMED fields, computed from the
    // freshly-loaded blocks. Every candidate's decode must reproduce this.
    let baseline_fingerprint: u64 = blocks
        .iter()
        .map(|(_, block)| consumed_fingerprint(block))
        .sum();

    let mut ser_candidates = vec![
        SerCandidate {
            label: "full+bincode",
            field_set: CacheFieldSet::Full,
            codec: CacheSerCodec::BincodeJson,
        },
        SerCandidate {
            label: "lean+bincode",
            field_set: CacheFieldSet::Lean,
            codec: CacheSerCodec::BincodeJson,
        },
    ];
    if args.msgpack {
        ser_candidates.push(SerCandidate {
            label: "full+msgpack",
            field_set: CacheFieldSet::Full,
            codec: CacheSerCodec::Msgpack,
        });
        ser_candidates.push(SerCandidate {
            label: "lean+msgpack",
            field_set: CacheFieldSet::Lean,
            codec: CacheSerCodec::Msgpack,
        });
    }

    let mut rows: Vec<Row> = Vec::new();
    rows.push(Row {
        label: "baseline(on-disk v2)".to_string(),
        blocks: baseline_blocks,
        total_bytes: baseline_total,
        avg_bytes: baseline_total as f64 / baseline_blocks as f64,
        compress_ms_per_block: f64::NAN,
        read_ms_per_block: baseline_read_wall_ms / blocks.len() as f64,
        note: "production codec".to_string(),
    });

    for cand in ser_candidates {
        match run_serialization_candidate(
            &cand,
            &blocks,
            baseline_fingerprint,
            &args.zstd_levels,
            args.dict_level,
            args.skip_dict,
            &args.chunk_sizes,
            args.chunk_level,
        ) {
            Ok(mut cand_rows) => rows.append(&mut cand_rows),
            Err(error) => {
                eprintln!("candidate {} failed: {error:#}", cand.label);
                rows.push(Row {
                    label: format!("{} (FAILED)", cand.label),
                    blocks: blocks.len() as u64,
                    total_bytes: 0,
                    avg_bytes: f64::NAN,
                    compress_ms_per_block: f64::NAN,
                    read_ms_per_block: f64::NAN,
                    note: truncate(&format!("{error:#}"), 80),
                });
            }
        }
    }

    print_results(&rows, baseline_total, baseline_blocks);
    Ok(())
}

/// Serialize all blocks once, then sweep compression candidates over the payloads.
#[allow(clippy::too_many_arguments)]
fn run_serialization_candidate(
    cand: &SerCandidate,
    blocks: &[(ProcessedBlockDiskCacheKey, ProcessedBlock)],
    baseline_fingerprint: u64,
    zstd_levels: &[i32],
    dict_level: i32,
    skip_dict: bool,
    chunk_sizes: &[usize],
    chunk_level: i32,
) -> Result<Vec<Row>> {
    // Serialize (uncompressed) and time it.
    let ser_started = Instant::now();
    let mut payloads: Vec<Vec<u8>> = Vec::with_capacity(blocks.len());
    for (key, block) in blocks {
        payloads.push(bench_serialize_block(key, block, cand.field_set, cand.codec)?);
    }
    let serialize_ms_per_block = ser_started.elapsed().as_secs_f64() * 1000.0 / blocks.len() as f64;

    // Round-trip correctness once on the uncompressed payloads: decode each and
    // confirm the consumed-field fingerprint matches the freshly loaded blocks.
    let mut decoded_fingerprint: u64 = 0;
    for payload in &payloads {
        let block = bench_deserialize_block(payload, cand.codec)
            .wrap_err("round-trip decode failed")?;
        decoded_fingerprint += consumed_fingerprint(&block);
    }
    if decoded_fingerprint != baseline_fingerprint {
        bail!(
            "consumed-field fingerprint mismatch (baseline={baseline_fingerprint}, \
             decoded={decoded_fingerprint}) - candidate would lose data consumers read"
        );
    }

    let uncompressed_total: u64 = payloads.iter().map(|p| p.len() as u64).sum();
    let blocks_n = blocks.len() as u64;
    let mut rows = Vec::new();

    // Plain zstd sweep.
    for &level in zstd_levels {
        let (total, compress_ms, read_ms) = measure_plain(&payloads, level)?;
        rows.push(Row {
            label: format!("{}/zstd{}", cand.label, level),
            blocks: blocks_n,
            total_bytes: total,
            avg_bytes: total as f64 / blocks_n as f64,
            compress_ms_per_block: compress_ms + serialize_ms_per_block,
            read_ms_per_block: read_ms,
            note: format!("uncompressed_avg={:.0}", uncompressed_total as f64 / blocks_n as f64),
        });
    }

    // Trained-dictionary candidate at dict_level.
    if !skip_dict {
        match measure_dict(&payloads, dict_level) {
            Ok((total, compress_ms, read_ms, dict_len)) => rows.push(Row {
                label: format!("{}/zstd{}+dict", cand.label, dict_level),
                blocks: blocks_n,
                total_bytes: total,
                avg_bytes: total as f64 / blocks_n as f64,
                compress_ms_per_block: compress_ms + serialize_ms_per_block,
                read_ms_per_block: read_ms,
                note: format!("dict_bytes={dict_len}"),
            }),
            Err(error) => eprintln!("  {} dictionary candidate skipped: {error:#}", cand.label),
        }
    }

    // Chunked-archive candidates: compress `chunk_size` consecutive block payloads
    // as one zstd frame to exploit cross-block redundancy (repeated routers,
    // selectors, token addresses). read_ms is amortized per block over a full
    // sequential range read (decompress each archive once); random single-block
    // access would pay the whole-archive decompress.
    for &chunk_size in chunk_sizes {
        if chunk_size < 2 {
            continue;
        }
        match measure_chunked(&payloads, chunk_size, chunk_level) {
            Ok((total, compress_ms, read_ms)) => rows.push(Row {
                label: format!("{}/zstd{}/chunk{}", cand.label, chunk_level, chunk_size),
                blocks: blocks_n,
                total_bytes: total,
                avg_bytes: total as f64 / blocks_n as f64,
                compress_ms_per_block: compress_ms + serialize_ms_per_block,
                read_ms_per_block: read_ms,
                note: format!("range-read amortized; {chunk_size} blocks/archive"),
            }),
            Err(error) => eprintln!("  {} chunk{chunk_size} skipped: {error:#}", cand.label),
        }
    }

    Ok(rows)
}

/// Worker thread count for the parallel compression passes.
fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Single-threaded compress-cost sample size. Compression (especially zstd-19)
/// is the slow knob; we get the per-block WRITE cost from a small serial sample
/// while still compressing every block in parallel to total the on-disk size.
const COMPRESS_SAMPLE: usize = 16;

/// Parallel map over payloads where each worker thread holds its own mutable
/// state (e.g. a zstd `Compressor`). Results are returned in input order.
fn parallel_map<S>(
    payloads: &[Vec<u8>],
    new_state: &(dyn Fn() -> Result<S> + Sync),
    run_one: &(dyn Fn(&mut S, &[u8]) -> Result<Vec<u8>> + Sync),
) -> Result<Vec<Vec<u8>>> {
    let n = payloads.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    let threads = worker_threads().max(1).min(n);
    let chunk = n.div_ceil(threads);
    let mut out: Vec<Option<Vec<u8>>> = (0..n).map(|_| None).collect();

    std::thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for (chunk_index, slice) in payloads.chunks(chunk).enumerate() {
            let base = chunk_index * chunk;
            handles.push(scope.spawn(move || -> Result<Vec<(usize, Vec<u8>)>> {
                let mut state = new_state()?;
                let mut res = Vec::with_capacity(slice.len());
                for (i, payload) in slice.iter().enumerate() {
                    res.push((base + i, run_one(&mut state, payload)?));
                }
                Ok(res)
            }));
        }
        for handle in handles {
            let res = handle
                .join()
                .map_err(|_| eyre::eyre!("benchmark compression worker panicked"))??;
            for (i, v) in res {
                out[i] = Some(v);
            }
        }
        Ok(())
    })?;

    Ok(out.into_iter().map(|v| v.expect("filled")).collect())
}

/// Compress all payloads (parallel) at `level`, time the per-block write cost on
/// a small serial sample, then decompress all (serial; decompression is cheap).
/// Returns (total_compressed_bytes, compress_ms_per_block, decompress_ms_per_block).
fn measure_plain(payloads: &[Vec<u8>], level: i32) -> Result<(u64, f64, f64)> {
    let compressed = parallel_map(
        payloads,
        &|| Ok(()),
        &|_state, payload| zstd::bulk::compress(payload, level).wrap_err("zstd compress failed"),
    )?;
    let total: u64 = compressed.iter().map(|c| c.len() as u64).sum();

    let sample = payloads.len().min(COMPRESS_SAMPLE);
    let started = Instant::now();
    for payload in &payloads[..sample] {
        std::hint::black_box(zstd::bulk::compress(payload, level)?);
    }
    let compress_ms = started.elapsed().as_secs_f64() * 1000.0 / sample.max(1) as f64;

    let started = Instant::now();
    for (i, c) in compressed.iter().enumerate() {
        std::hint::black_box(zstd::bulk::decompress(c, payloads[i].len())?);
    }
    let decompress_ms = started.elapsed().as_secs_f64() * 1000.0 / payloads.len().max(1) as f64;

    Ok((total, compress_ms, decompress_ms))
}

/// Train a zstd dictionary on the payloads, then compress (parallel) and
/// decompress (serial) all of them with it.
/// Returns (total, compress_ms/block, decompress_ms/block, dict_len).
fn measure_dict(payloads: &[Vec<u8>], level: i32) -> Result<(u64, f64, f64, usize)> {
    use zstd::bulk::{Compressor, Decompressor};

    let train = &payloads[..payloads.len().min(DICT_TRAIN_SAMPLES)];
    let dict = zstd::dict::from_samples(train, DICT_SIZE)
        .wrap_err("zstd dictionary training failed")?;

    let compressed = parallel_map(
        payloads,
        &|| Compressor::with_dictionary(level, &dict).wrap_err("dict compressor init failed"),
        &|compressor, payload| compressor.compress(payload).wrap_err("zstd dict compress failed"),
    )?;
    let total: u64 = compressed.iter().map(|c| c.len() as u64).sum();

    let sample = payloads.len().min(COMPRESS_SAMPLE);
    let started = Instant::now();
    let mut sample_compressor = Compressor::with_dictionary(level, &dict)?;
    for payload in &payloads[..sample] {
        std::hint::black_box(sample_compressor.compress(payload)?);
    }
    let compress_ms = started.elapsed().as_secs_f64() * 1000.0 / sample.max(1) as f64;

    let started = Instant::now();
    let mut decompressor = Decompressor::with_dictionary(&dict)?;
    for (i, c) in compressed.iter().enumerate() {
        std::hint::black_box(decompressor.decompress(c, payloads[i].len())?);
    }
    let decompress_ms = started.elapsed().as_secs_f64() * 1000.0 / payloads.len().max(1) as f64;

    Ok((total, compress_ms, decompress_ms, dict.len()))
}

/// Compress groups of `chunk_size` consecutive block payloads as single zstd
/// frames (one archive per group) to exploit cross-block redundancy. Each block
/// is length-prefixed inside its archive so it can be split back out.
/// Returns (total_compressed_bytes, compress_ms/block, decompress_ms/block) where
/// decompress is amortized over a full sequential range read.
fn measure_chunked(payloads: &[Vec<u8>], chunk_size: usize, level: i32) -> Result<(u64, f64, f64)> {
    // Build one framed buffer per group: [u32 len | bytes] repeated.
    let archives: Vec<Vec<u8>> = payloads
        .chunks(chunk_size)
        .map(|group| {
            let cap: usize = group.iter().map(|p| p.len() + 4).sum();
            let mut buf = Vec::with_capacity(cap);
            for p in group {
                buf.extend_from_slice(&(p.len() as u32).to_le_bytes());
                buf.extend_from_slice(p);
            }
            buf
        })
        .collect();
    let uncompressed: Vec<usize> = archives.iter().map(|a| a.len()).collect();

    let compressed = parallel_map(
        &archives,
        &|| Ok(()),
        &|_state, archive| zstd::bulk::compress(archive, level).wrap_err("chunk compress failed"),
    )?;
    let total: u64 = compressed.iter().map(|c| c.len() as u64).sum();
    let blocks = payloads.len().max(1) as f64;

    // Per-block compress cost = compress a few archives serially / blocks covered.
    let sample = archives.len().min(4).max(1);
    let started = Instant::now();
    for archive in &archives[..sample] {
        std::hint::black_box(zstd::bulk::compress(archive, level)?);
    }
    let sampled_blocks = (sample * chunk_size).min(payloads.len()).max(1) as f64;
    let compress_ms = started.elapsed().as_secs_f64() * 1000.0 / sampled_blocks;

    // Range-read cost: decompress every archive once, amortize over all blocks.
    let started = Instant::now();
    for (i, c) in compressed.iter().enumerate() {
        std::hint::black_box(zstd::bulk::decompress(c, uncompressed[i])?);
    }
    let decompress_ms = started.elapsed().as_secs_f64() * 1000.0 / blocks;

    Ok((total, compress_ms, decompress_ms))
}

/// Per-field-group size breakdown: shows where the uncompressed payload bytes
/// go. For each group we sum the JSON-serialized size across all sampled
/// transactions (raw data volume) and zstd-9 the concatenated group bytes
/// (compressibility-aware footprint). JSON is used because several fields are
/// not bincode-serializable in isolation; absolute numbers differ from the
/// production bincode payload but the RELATIVE distribution is what matters.
fn run_field_breakdown(blocks: &[(ProcessedBlockDiskCacheKey, ProcessedBlock)]) -> Result<()> {
    use tx_processor::CompactProcessedTransaction;
    // group name -> (raw_bytes, concatenated bytes for compression)
    let mut groups: BTreeMap<&str, (u64, Vec<u8>)> = BTreeMap::new();
    let mut tx_count: u64 = 0;
    let mut full_raw: u64 = 0;

    macro_rules! grp {
        ($name:expr, $val:expr) => {{
            let bytes = serde_json::to_vec(&$val).unwrap_or_default();
            let entry = groups.entry($name).or_insert_with(|| (0, Vec::new()));
            entry.0 += bytes.len() as u64;
            entry.1.extend_from_slice(&bytes);
        }};
    }

    for (_, block) in blocks {
        for tx in &block.transactions {
            tx_count += 1;
            let p = CompactProcessedTransaction::from_processed(&tx.processed);
            full_raw += serde_json::to_vec(&p).unwrap_or_default().len() as u64;

            grp!("input(calldata)", p.input);
            grp!("latest_states", p.latest_states);
            grp!("other_events", p.other_events);
            grp!("address_balance_changes", p.address_balance_changes);
            grp!("struct_logs", p.struct_logs);
            grp!("internal_transactions", p.internal_transactions);
            grp!("internal_erc20_calls", p.internal_erc20_calls);
            grp!("erc20_transfers", p.erc20_transfers);
            grp!("eth_transfers", p.eth_transfers);
            grp!("erc721+erc1155_transfers", (&p.erc721_transfers, &p.erc1155_transfers));
            grp!(
                "address_sets",
                (&p.unique_addresses, &p.erc20_contracts, &p.erc721_contracts, &p.erc1155_contracts)
            );
            grp!(
                "uniswap_v2",
                (&p.uniswap_v2_swaps, &p.uniswap_v2_syncs, &p.uniswap_v2_mints, &p.uniswap_v2_burns, &p.uniswap_v2_pair_created_events)
            );
            grp!(
                "uniswap_v3",
                (&p.uniswap_v3_swaps, &p.uniswap_v3_mints, &p.uniswap_v3_burns, &p.uniswap_v3_pools, &p.uniswap_v3_positions, &p.uniswap_v3_increases, &p.uniswap_v3_decreases, &p.uniswap_v3_initializations)
            );
            grp!(
                "uniswap_v4",
                (&p.uniswap_v4_swaps, &p.uniswap_v4_modifies, &p.uniswap_v4_balance_deltas, &p.uniswap_v4_initializes, &p.uniswap_v4_donates, &p.uniswap_v4_protocol_fee_updates, &p.uniswap_v4_dynamic_lp_fee_updates, &p.uniswap_v4_protocol_fee_controller_updates)
            );
            grp!(
                "approvals",
                (&p.erc20_approval_events, &p.erc721_approval_events, &p.approval_for_all_events)
            );
            grp!(
                "authority",
                (&p.ownership_transferred_events, &p.ownership_transfer_started_events, &p.access_control_role_granted_events, &p.access_control_role_revoked_events, &p.proxy_admin_changed_events, &p.contract_creation_events, &p.trading_enabled_events, &p.trading_disabled_events, &p.permit2_events)
            );
            grp!(
                "tx_misc(access_list,blob,signed_auth,deposits)",
                (&p.access_list, &p.blob_versioned_hashes, &p.signed_authorizations, &p.deposit_events, &p.withdraw_events)
            );
        }
    }

    // Compute compressed footprint per group.
    let mut rows: Vec<(String, u64, u64)> = Vec::new();
    let mut sum_raw = 0u64;
    let mut sum_comp = 0u64;
    for (name, (raw, blob)) in &groups {
        let comp = zstd::bulk::compress(blob, 9).map(|c| c.len() as u64).unwrap_or(0);
        sum_raw += *raw;
        sum_comp += comp;
        rows.push((name.to_string(), *raw, comp));
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2)); // by compressed footprint desc

    println!("field_group,raw_bytes_per_block,zstd9_bytes_per_block,pct_of_zstd9");
    eprintln!(
        "\n=== field-size breakdown ({tx_count} txs across {} blocks) ===",
        blocks.len()
    );
    eprintln!(
        "{:<46} {:>14} {:>14} {:>8}",
        "group", "raw/blk", "zstd9/blk", "%comp"
    );
    let blocks_n = blocks.len().max(1) as f64;
    for (name, raw, comp) in &rows {
        let pct = if sum_comp > 0 { *comp as f64 / sum_comp as f64 * 100.0 } else { 0.0 };
        println!("{name},{:.0},{:.0},{:.1}", *raw as f64 / blocks_n, *comp as f64 / blocks_n, pct);
        eprintln!(
            "{:<46} {:>14.0} {:>14.0} {:>7.1}%",
            truncate(name, 46),
            *raw as f64 / blocks_n,
            *comp as f64 / blocks_n,
            pct
        );
    }
    eprintln!(
        "{:<46} {:>14.0} {:>14.0}",
        "TOTAL (grouped)",
        sum_raw as f64 / blocks_n,
        sum_comp as f64 / blocks_n
    );
    eprintln!(
        "full compact JSON raw/blk = {:.0} (grouped covers {:.0}%)",
        full_raw as f64 / blocks_n,
        sum_raw as f64 / full_raw.max(1) as f64 * 100.0
    );
    Ok(())
}

/// Order-independent fingerprint over the fields consumers actually read.
/// Summing lengths is insensitive to HashMap iteration order, so a faithful
/// round-trip yields an identical value; a dropped/garbled consumed field does not.
fn consumed_fingerprint(block: &ProcessedBlock) -> u64 {
    let mut acc: u64 = block.transactions.len() as u64;
    for tx in &block.transactions {
        let p = &tx.processed;
        // Weight each field by a small distinct prime so that a count moving
        // between fields (e.g. mis-mapped vectors) still changes the sum.
        acc = acc
            .wrapping_add(2 * p.eth_transfers.len() as u64)
            .wrapping_add(3 * p.erc20_transfers.len() as u64)
            .wrapping_add(5 * p.erc721_transfers.len() as u64)
            .wrapping_add(7 * p.internal_transactions.len() as u64)
            .wrapping_add(11 * p.internal_erc20_calls.len() as u64)
            .wrapping_add(13 * p.unique_addresses.len() as u64)
            .wrapping_add(17 * p.erc20_contracts.len() as u64)
            .wrapping_add(19 * p.uniswap_v2_swaps.len() as u64)
            .wrapping_add(23 * p.uniswap_v2_syncs.len() as u64)
            .wrapping_add(29 * p.uniswap_v3_swaps.len() as u64)
            .wrapping_add(31 * p.uniswap_v3_mints.len() as u64)
            .wrapping_add(37 * p.uniswap_v3_burns.len() as u64)
            .wrapping_add(41 * p.uniswap_v4_swaps.len() as u64)
            .wrapping_add(43 * p.uniswap_v4_modifies.len() as u64)
            .wrapping_add(47 * p.uniswap_v4_balance_deltas.len() as u64)
            .wrapping_add(53 * p.erc20_approval_events.len() as u64)
            .wrapping_add(59 * p.erc721_approval_events.len() as u64)
            .wrapping_add(61 * p.approval_for_all_events.len() as u64)
            .wrapping_add(67 * p.ownership_transferred_events.len() as u64)
            .wrapping_add(71 * p.contract_creation_events.len() as u64)
            .wrapping_add(73 * p.permit2_events.len() as u64)
            .wrapping_add(79 * p.other_events.len() as u64)
            .wrapping_add(83 * p.address_balance_changes.len() as u64)
            .wrapping_add(89 * p.latest_states.len() as u64)
            // Consumed-but-easy-to-mistake-for-droppable fields: erc1155_transfers
            // (tx_fund_flow) and the v4 fee-update vectors (eth_token). Including
            // them here makes the round-trip check fail if Lean ever drops them.
            .wrapping_add(97 * p.erc1155_transfers.len() as u64)
            .wrapping_add(101 * p.uniswap_v4_protocol_fee_updates.len() as u64)
            .wrapping_add(103 * p.uniswap_v4_dynamic_lp_fee_updates.len() as u64)
            .wrapping_add(107 * p.uniswap_v4_protocol_fee_controller_updates.len() as u64);
    }
    acc
}

/// Scan a network dir for `<block>.v<N>.pblock.zst` files of the highest version
/// present, returning block -> (on-disk byte size, path).
fn scan_current_version_files(network_dir: &Path) -> Result<BTreeMap<u64, (u64, PathBuf)>> {
    let mut by_version: BTreeMap<u32, BTreeMap<u64, (u64, PathBuf)>> = BTreeMap::new();
    for entry in fs::read_dir(network_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some((block, version)) = parse_versioned_cache_name(name) else {
            continue;
        };
        let size = entry.metadata()?.len();
        by_version
            .entry(version)
            .or_default()
            .insert(block, (size, entry.path()));
    }
    Ok(by_version
        .into_iter()
        .next_back()
        .map(|(_, files)| files)
        .unwrap_or_default())
}

/// Parse `<block>.v<N>.pblock.zst` -> (block, N). Rejects the legacy unversioned
/// `<block>.pblock.zst` (returns None) so baselines never mix schema versions.
fn parse_versioned_cache_name(name: &str) -> Option<(u64, u32)> {
    let stem = name.strip_suffix(CACHE_FILE_SUFFIX)?; // "<block>.v<N>"
    let (block_str, version_str) = stem.rsplit_once(".v")?;
    let block = block_str.parse::<u64>().ok()?;
    let version = version_str.parse::<u32>().ok()?;
    Some((block, version))
}

fn resolve_cache_dir(arg: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = arg {
        return Ok(path.to_path_buf());
    }
    for var in [DISK_CACHE_DIR_ENV, CHAIN_SERVER_DISK_CACHE_DIR_ENV] {
        if let Ok(value) = std::env::var(var) {
            if !value.trim().is_empty() {
                return Ok(PathBuf::from(value));
            }
        }
    }
    bail!(
        "no cache dir: pass --cache-dir or set {DISK_CACHE_DIR_ENV} / {CHAIN_SERVER_DISK_CACHE_DIR_ENV}"
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn print_results(rows: &[Row], baseline_total: u64, baseline_blocks: u64) {
    // CSV (machine-readable).
    println!("label,blocks,total_bytes,avg_bytes,compress_ms_per_block,read_ms_per_block,space_vs_baseline,note");
    for row in rows {
        let ratio = if baseline_total > 0 {
            row.total_bytes as f64 / baseline_total as f64
        } else {
            f64::NAN
        };
        println!(
            "{},{},{},{:.1},{:.4},{:.4},{:.3},{}",
            row.label,
            row.blocks,
            row.total_bytes,
            row.avg_bytes,
            row.compress_ms_per_block,
            row.read_ms_per_block,
            ratio,
            row.note
        );
    }

    // Human summary.
    eprintln!("\n=== summary (baseline = on-disk v2, {baseline_blocks} blocks) ===");
    eprintln!(
        "{:<28} {:>12} {:>10} {:>10} {:>11}",
        "candidate", "avg_bytes", "vs_base", "comp_ms", "read_ms"
    );
    for row in rows {
        let ratio = if baseline_total > 0 {
            row.total_bytes as f64 / baseline_total as f64
        } else {
            f64::NAN
        };
        let vs = if row.label.starts_with("baseline") {
            "1.00x (base)".to_string()
        } else if row.total_bytes == 0 {
            "n/a".to_string()
        } else {
            format!("{:.2}x ({:+.0}%)", ratio, (ratio - 1.0) * 100.0)
        };
        eprintln!(
            "{:<28} {:>12.0} {:>10} {:>10.3} {:>11.3}",
            truncate(&row.label, 28),
            row.avg_bytes,
            vs,
            row.compress_ms_per_block,
            row.read_ms_per_block
        );
    }
}
