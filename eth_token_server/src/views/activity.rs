use std::fmt;
use std::str::FromStr;

use alloy_primitives::Address;
use reth_chain_query::RethQueryProvider;
use serde::{Deserialize, Serialize};

const DEFAULT_BLOCK_LIMIT: usize = 1_000;
const MAX_BLOCK_LIMIT: usize = 10_000;
const DEFAULT_PAD_BLOCKS: u64 = 20;
const MAX_RANGE_COUNT: usize = 500;
const SOURCE_RETH_INDEX_ADDRESS_PARTICIPATION: &str = "reth_index_address_participation";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TokenActivityBlocksQuery {
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub pad_blocks: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ActivityBlockRange {
    pub start_block: u64,
    pub end_block: u64,
    pub block_count: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenActivityBlocksResponse {
    pub token_address: String,
    pub source: &'static str,
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
    pub active_block_count: usize,
    pub first_block: Option<u64>,
    pub last_block: Option<u64>,
    pub activity_span_blocks: Option<u64>,
    pub suggested_start_block: Option<u64>,
    pub suggested_end_block: Option<u64>,
    pub suggested_block_count: Option<u64>,
    pub returned_block_count: usize,
    pub blocks_truncated: bool,
    pub blocks: Vec<u64>,
    pub range_count: usize,
    pub ranges_truncated: bool,
    pub ranges: Vec<ActivityBlockRange>,
    pub caveats: Vec<&'static str>,
}

#[derive(Debug)]
pub enum TokenActivityError {
    InvalidAddress(String),
    IndexUnavailable(String),
    Lookup(String),
}

impl fmt::Display for TokenActivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(message) => write!(formatter, "{message}"),
            Self::IndexUnavailable(message) => write!(formatter, "{message}"),
            Self::Lookup(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for TokenActivityError {}

pub fn token_activity_blocks(
    provider: &RethQueryProvider,
    token_address: &str,
    query: TokenActivityBlocksQuery,
) -> Result<TokenActivityBlocksResponse, TokenActivityError> {
    if let (Some(start_block), Some(end_block)) = (query.start_block, query.end_block) {
        if end_block < start_block {
            return Err(TokenActivityError::InvalidAddress(
                "end_block must be greater than or equal to start_block".to_string(),
            ));
        }
    }

    let address = parse_token_address(token_address)?;
    let token_address = format!("{address:#x}");
    let mut blocks = provider
        .get_address_participation_blocks(address)
        .map_err(classify_index_error)?;
    blocks.sort_unstable();
    blocks.dedup();

    let blocks = blocks
        .into_iter()
        .filter(|block| query.start_block.map_or(true, |start| *block >= start))
        .filter(|block| query.end_block.map_or(true, |end| *block <= end))
        .collect::<Vec<_>>();

    let active_block_count = blocks.len();
    let first_block = blocks.first().copied();
    let last_block = blocks.last().copied();
    let activity_span_blocks = first_block
        .zip(last_block)
        .map(|(first, last)| last.saturating_sub(first) + 1);

    let pad_blocks = query.pad_blocks.unwrap_or(DEFAULT_PAD_BLOCKS);
    let latest_block = provider.get_latest_block().ok();
    let suggested_start_block = first_block.map(|block| block.saturating_sub(pad_blocks));
    let suggested_end_block = last_block.map(|block| {
        let padded = block.saturating_add(pad_blocks);
        latest_block.map_or(padded, |latest| padded.min(latest))
    });
    let suggested_block_count = suggested_start_block
        .zip(suggested_end_block)
        .map(|(start, end)| end.saturating_sub(start) + 1);

    let limit = query
        .limit
        .unwrap_or(DEFAULT_BLOCK_LIMIT)
        .clamp(1, MAX_BLOCK_LIMIT);
    let blocks_truncated = active_block_count > limit;
    let returned_blocks = blocks.iter().take(limit).copied().collect::<Vec<_>>();

    let ranges = compact_ranges(&blocks);
    let range_count = ranges.len();
    let ranges_truncated = range_count > MAX_RANGE_COUNT;
    let ranges = ranges.into_iter().take(MAX_RANGE_COUNT).collect();

    Ok(TokenActivityBlocksResponse {
        token_address,
        source: SOURCE_RETH_INDEX_ADDRESS_PARTICIPATION,
        start_block: query.start_block,
        end_block: query.end_block,
        active_block_count,
        first_block,
        last_block,
        activity_span_blocks,
        suggested_start_block,
        suggested_end_block,
        suggested_block_count,
        returned_block_count: returned_blocks.len(),
        blocks_truncated,
        blocks: returned_blocks,
        range_count,
        ranges_truncated,
        ranges,
        caveats: vec![
            "Uses the processed-block address participation index, not Reth account history.",
            "Uniswap v4 pool ids are not EVM addresses; query the ERC-20 token/currency address.",
        ],
    })
}

fn parse_token_address(value: &str) -> Result<Address, TokenActivityError> {
    let value = value.trim();
    if looks_like_pool_id(value) {
        return Err(TokenActivityError::InvalidAddress(
            "value looks like a 32-byte Uniswap v4 pool id, not an ERC-20 token address"
                .to_string(),
        ));
    }

    Address::from_str(value)
        .map_err(|_| TokenActivityError::InvalidAddress("invalid EVM token address".to_string()))
}

fn looks_like_pool_id(value: &str) -> bool {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn classify_index_error(error: eyre::Report) -> TokenActivityError {
    let message = error.to_string();
    if message.contains("RethIndex not available") {
        TokenActivityError::IndexUnavailable(
            "RethIndex address participation database is not configured".to_string(),
        )
    } else {
        TokenActivityError::Lookup(format!("failed to load token activity blocks: {message}"))
    }
}

fn compact_ranges(blocks: &[u64]) -> Vec<ActivityBlockRange> {
    let Some(&first) = blocks.first() else {
        return Vec::new();
    };

    let mut ranges = Vec::new();
    let mut start = first;
    let mut previous = first;

    for &block in blocks.iter().skip(1) {
        if block == previous.saturating_add(1) {
            previous = block;
            continue;
        }
        ranges.push(ActivityBlockRange {
            start_block: start,
            end_block: previous,
            block_count: previous.saturating_sub(start) + 1,
        });
        start = block;
        previous = block;
    }

    ranges.push(ActivityBlockRange {
        start_block: start,
        end_block: previous,
        block_count: previous.saturating_sub(start) + 1,
    });

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_id_shape_is_not_accepted_as_token_address() {
        let pool_id = "0xc5d22335fcb80282d32143be982064cf1c61eab4b95069f0d397d3f605c3c8f0";
        assert!(matches!(
            parse_token_address(pool_id),
            Err(TokenActivityError::InvalidAddress(_))
        ));
    }

    #[test]
    fn compact_ranges_groups_contiguous_blocks() {
        let ranges = compact_ranges(&[10, 11, 12, 15, 20, 21]);

        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].start_block, 10);
        assert_eq!(ranges[0].end_block, 12);
        assert_eq!(ranges[0].block_count, 3);
        assert_eq!(ranges[1].start_block, 15);
        assert_eq!(ranges[2].start_block, 20);
        assert_eq!(ranges[2].end_block, 21);
    }
}
