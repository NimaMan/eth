use alloy_primitives::Address;
use eth_token::network::flow_context::FlowContextConfig;
use eth_token::network::model::normalize_network_address;
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const DEFAULT_ANALYSIS_BLOCKS: u64 = 100_000;
pub const DEFAULT_HISTORY_LIMIT: usize = 1_000;
pub const DEFAULT_MAX_TOKEN_BLOCKS: usize = 256;
pub const DEFAULT_MAX_SEED_ADDRESSES: usize = 16;
pub const DEFAULT_MAX_BLOCKS_PER_ADDRESS: usize = 64;
pub const DEFAULT_LOOKBACK_BLOCKS: u64 = 300;
pub const DEFAULT_LOOKAHEAD_BLOCKS: u64 = 80;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkAnalysisRequest {
    pub token: String,
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub history_limit: Option<usize>,
    #[serde(default)]
    pub max_token_blocks: Option<usize>,
    #[serde(default)]
    pub max_seeds: Option<usize>,
    #[serde(default)]
    pub max_blocks_per_address: Option<usize>,
    #[serde(default)]
    pub lookback_blocks: Option<u64>,
    #[serde(default)]
    pub lookahead_blocks: Option<u64>,
    #[serde(default)]
    pub include_timeline: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedNetworkAnalysisRequest {
    pub token: String,
    pub start_block: u64,
    pub end_block: u64,
    pub history_limit: usize,
    pub max_token_blocks: usize,
    pub max_seeds: usize,
    pub max_blocks_per_address: usize,
    pub lookback_blocks: u64,
    pub lookahead_blocks: u64,
    pub include_timeline: bool,
}

impl NetworkAnalysisRequest {
    pub fn resolve(self, latest_block: u64) -> Result<ResolvedNetworkAnalysisRequest> {
        let token = normalize_token(&self.token)?;
        let end_block = self.end_block.unwrap_or(latest_block).min(latest_block);
        let default_start = end_block.saturating_sub(DEFAULT_ANALYSIS_BLOCKS.saturating_sub(1));
        let start_block = self.start_block.unwrap_or(default_start).min(end_block);
        if start_block > end_block {
            bail!("start_block must be <= end_block");
        }

        let history_limit = non_zero(
            self.history_limit.unwrap_or(DEFAULT_HISTORY_LIMIT),
            "history_limit",
        )?;
        let max_token_blocks = non_zero(
            self.max_token_blocks.unwrap_or(DEFAULT_MAX_TOKEN_BLOCKS),
            "max_token_blocks",
        )?;
        let max_seeds = non_zero(
            self.max_seeds.unwrap_or(DEFAULT_MAX_SEED_ADDRESSES),
            "max_seeds",
        )?;
        let max_blocks_per_address = non_zero(
            self.max_blocks_per_address
                .unwrap_or(DEFAULT_MAX_BLOCKS_PER_ADDRESS),
            "max_blocks_per_address",
        )?;

        Ok(ResolvedNetworkAnalysisRequest {
            token,
            start_block,
            end_block,
            history_limit,
            max_token_blocks,
            max_seeds,
            max_blocks_per_address,
            lookback_blocks: self.lookback_blocks.unwrap_or(DEFAULT_LOOKBACK_BLOCKS),
            lookahead_blocks: self.lookahead_blocks.unwrap_or(DEFAULT_LOOKAHEAD_BLOCKS),
            include_timeline: self.include_timeline.unwrap_or(false),
        })
    }
}

impl ResolvedNetworkAnalysisRequest {
    pub fn token_address(&self) -> Result<Address> {
        Address::from_str(&self.token).map_err(Into::into)
    }

    pub fn flow_context_config(&self) -> FlowContextConfig {
        FlowContextConfig {
            max_seed_addresses: self.max_seeds,
            lookback_blocks: self.lookback_blocks,
            lookahead_blocks: self.lookahead_blocks,
            max_blocks_per_address: self.max_blocks_per_address,
            ..FlowContextConfig::default()
        }
    }

    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

fn normalize_token(value: &str) -> Result<String> {
    let address = Address::from_str(value.trim())?;
    Ok(normalize_network_address(format!("{address:#x}")))
}

fn non_zero(value: usize, name: &str) -> Result<usize> {
    if value == 0 {
        bail!("{name} must be greater than zero");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_defaults_to_bounded_recent_range() {
        let request = NetworkAnalysisRequest {
            token: "0x133a79c66bc8789cf4d081159654aa378004541c".to_string(),
            start_block: None,
            end_block: None,
            history_limit: None,
            max_token_blocks: None,
            max_seeds: None,
            max_blocks_per_address: None,
            lookback_blocks: None,
            lookahead_blocks: None,
            include_timeline: None,
        };

        let resolved = request.resolve(1_000_000).expect("request resolves");

        assert_eq!(resolved.token, "0x133a79c66bc8789cf4d081159654aa378004541c");
        assert_eq!(resolved.end_block, 1_000_000);
        assert_eq!(resolved.block_count(), DEFAULT_ANALYSIS_BLOCKS);
        assert_eq!(resolved.max_token_blocks, DEFAULT_MAX_TOKEN_BLOCKS);
        assert!(!resolved.include_timeline);
    }

    #[test]
    fn resolve_rejects_zero_limits() {
        let request = NetworkAnalysisRequest {
            token: "0x133a79c66bc8789cf4d081159654aa378004541c".to_string(),
            start_block: Some(1),
            end_block: Some(10),
            history_limit: Some(0),
            max_token_blocks: None,
            max_seeds: None,
            max_blocks_per_address: None,
            lookback_blocks: None,
            lookahead_blocks: None,
            include_timeline: None,
        };

        let error = request.resolve(10).expect_err("zero history rejects");

        assert!(error.to_string().contains("history_limit"));
    }

    #[test]
    fn resolve_accepts_timeline_flag() {
        let request: NetworkAnalysisRequest = serde_json::from_str(
            r#"{
                "token": "0x133a79c66bc8789cf4d081159654aa378004541c",
                "include_timeline": true
            }"#,
        )
        .expect("request deserializes");

        let resolved = request.resolve(1_000).expect("request resolves");

        assert!(resolved.include_timeline);
    }
}
