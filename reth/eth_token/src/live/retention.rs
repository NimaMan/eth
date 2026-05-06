use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::erc20::ERC20Token;
use crate::manager::{TokenRegistry, TrackedTokenIndex};
use crate::pools::UniswapV2Pool;

pub const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
pub const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
pub const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRetentionPolicy {
    pub min_weth_denom_reserve: f64,
    pub min_stable_denom_reserve: f64,
    pub min_other_denom_reserve: f64,
    pub weth_denoms: BTreeSet<String>,
    pub stablecoin_denoms: BTreeSet<String>,
    pub drop_tokens_without_pools_after_blocks: Option<u64>,
    pub drop_tokens_without_retained_pools_after_blocks: Option<u64>,
}

impl Default for LiveTokenRetentionPolicy {
    fn default() -> Self {
        Self {
            min_weth_denom_reserve: 0.1,
            min_stable_denom_reserve: 500.0,
            min_other_denom_reserve: 0.0,
            weth_denoms: address_set([WETH_ADDRESS]),
            stablecoin_denoms: address_set([USDC_ADDRESS, USDT_ADDRESS, DAI_ADDRESS]),
            drop_tokens_without_pools_after_blocks: None,
            drop_tokens_without_retained_pools_after_blocks: None,
        }
    }
}

impl LiveTokenRetentionPolicy {
    pub fn denom_class(&self, denom_address: impl AsRef<str>) -> LivePoolDenomClass {
        let denom_address = normalize_address(denom_address);
        if self.weth_denoms.contains(&denom_address) {
            LivePoolDenomClass::Weth
        } else if self.stablecoin_denoms.contains(&denom_address) {
            LivePoolDenomClass::Stablecoin
        } else {
            LivePoolDenomClass::Other
        }
    }

    pub fn denom_threshold(&self, denom_address: impl AsRef<str>) -> f64 {
        match self.denom_class(denom_address) {
            LivePoolDenomClass::Weth => self.min_weth_denom_reserve,
            LivePoolDenomClass::Stablecoin => self.min_stable_denom_reserve,
            LivePoolDenomClass::Other => self.min_other_denom_reserve,
        }
    }

    pub fn evaluate_pool(&self, pool: &UniswapV2Pool) -> LivePoolRetentionDecision {
        let pool_address = pool.base.identity.pool_address.clone();
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_class = self.denom_class(&denom_address);
        let denom_reserve = pool.base.denom_reserve();
        let threshold = self.denom_threshold(&denom_address);
        let retain = threshold <= 0.0 || denom_reserve >= threshold;
        let reason = if retain {
            None
        } else {
            Some(PoolDropReason::BelowDenomThreshold {
                denom_address: normalize_address(&denom_address),
                denom_reserve,
                threshold,
            })
        };

        LivePoolRetentionDecision {
            pool_address,
            retain,
            reason,
            denom_address: normalize_address(denom_address),
            denom_class,
            denom_reserve,
            threshold,
        }
    }

    pub fn evaluate_token(
        &self,
        token: &ERC20Token,
        current_block: u64,
    ) -> LiveTokenRetentionDecision {
        let mut pool_decisions: Vec<_> = token
            .v2_pools
            .values()
            .map(|pool| self.evaluate_pool(pool))
            .collect();
        pool_decisions.sort_by(|left, right| left.pool_address.cmp(&right.pool_address));

        let retained_v2_pools = pool_decisions
            .iter()
            .filter(|decision| decision.retain)
            .map(|decision| decision.pool_address.clone())
            .collect::<Vec<_>>();
        let dropped_v2_pools = pool_decisions
            .into_iter()
            .filter(|decision| !decision.retain)
            .collect::<Vec<_>>();

        let reason = if !retained_v2_pools.is_empty() {
            None
        } else if token.v2_pools.is_empty() {
            self.pending_token_drop_reason(
                token_reference_block(token),
                current_block,
                self.drop_tokens_without_pools_after_blocks,
                TokenDropReasonKind::NoPools,
            )
        } else {
            self.pending_token_drop_reason(
                latest_pool_reference_block(token),
                current_block,
                self.drop_tokens_without_retained_pools_after_blocks,
                TokenDropReasonKind::NoRetainedPools,
            )
        };

        LiveTokenRetentionDecision {
            token_address: token.contract_address.clone(),
            retain: reason.is_none(),
            reason,
            retained_v2_pools,
            dropped_v2_pools,
        }
    }

    pub fn apply_to_token(
        &self,
        token: &mut ERC20Token,
        current_block: u64,
    ) -> LiveTokenRetentionDecision {
        let decision = self.evaluate_token(token, current_block);
        for pool in &decision.dropped_v2_pools {
            token.v2_pools.remove(&pool.pool_address);
        }
        decision
    }

    pub fn apply_to_registry(
        &self,
        registry: &mut TokenRegistry,
        token_index: &mut TrackedTokenIndex,
        current_block: u64,
    ) -> LiveTokenRetentionReport {
        let mut token_addresses = registry.token_addresses();
        token_addresses.sort();

        let mut token_decisions = Vec::new();
        let mut retained_tokens = 0;
        let mut dropped_tokens = 0;
        let mut dropped_v2_pool_count = 0;

        for token_address in token_addresses {
            let Some(token) = registry.tokens.get_mut(&token_address) else {
                continue;
            };

            let decision = self.apply_to_token(token, current_block);
            dropped_v2_pool_count += decision.dropped_v2_pools.len();

            if decision.retain {
                retained_tokens += 1;
                token_index.update_pool_mapping(token);
            } else {
                dropped_tokens += 1;
                registry.tokens.remove(&token_address);
                token_index.remove_token(&token_address);
            }

            token_decisions.push(decision);
        }

        LiveTokenRetentionReport {
            current_block,
            evaluated_tokens: token_decisions.len(),
            retained_tokens,
            dropped_tokens,
            dropped_v2_pool_count,
            token_decisions,
        }
    }

    fn pending_token_drop_reason(
        &self,
        reference_block: Option<u64>,
        current_block: u64,
        retention_blocks: Option<u64>,
        kind: TokenDropReasonKind,
    ) -> Option<TokenDropReason> {
        let retention_blocks = retention_blocks?;
        let reference_block = reference_block?;
        if current_block.saturating_sub(reference_block) < retention_blocks {
            return None;
        }

        match kind {
            TokenDropReasonKind::NoPools => Some(TokenDropReason::NoPoolsPastRetentionBlocks {
                reference_block,
                current_block,
                retention_blocks,
            }),
            TokenDropReasonKind::NoRetainedPools => {
                Some(TokenDropReason::NoRetainedPoolsPastRetentionBlocks {
                    reference_block,
                    current_block,
                    retention_blocks,
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivePoolDenomClass {
    Weth,
    Stablecoin,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivePoolRetentionDecision {
    pub pool_address: String,
    pub retain: bool,
    pub reason: Option<PoolDropReason>,
    pub denom_address: String,
    pub denom_class: LivePoolDenomClass,
    pub denom_reserve: f64,
    pub threshold: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolDropReason {
    BelowDenomThreshold {
        denom_address: String,
        denom_reserve: f64,
        threshold: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRetentionDecision {
    pub token_address: String,
    pub retain: bool,
    pub reason: Option<TokenDropReason>,
    pub retained_v2_pools: Vec<String>,
    pub dropped_v2_pools: Vec<LivePoolRetentionDecision>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenDropReason {
    NoPoolsPastRetentionBlocks {
        reference_block: u64,
        current_block: u64,
        retention_blocks: u64,
    },
    NoRetainedPoolsPastRetentionBlocks {
        reference_block: u64,
        current_block: u64,
        retention_blocks: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRetentionReport {
    pub current_block: u64,
    pub evaluated_tokens: usize,
    pub retained_tokens: usize,
    pub dropped_tokens: usize,
    pub dropped_v2_pool_count: usize,
    pub token_decisions: Vec<LiveTokenRetentionDecision>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenDropReasonKind {
    NoPools,
    NoRetainedPools,
}

fn token_reference_block(token: &ERC20Token) -> Option<u64> {
    token.latest_block_number.or(token.creation_block)
}

fn latest_pool_reference_block(token: &ERC20Token) -> Option<u64> {
    token
        .v2_pools
        .values()
        .filter_map(pool_reference_block)
        .max()
        .or_else(|| token_reference_block(token))
}

fn pool_reference_block(pool: &UniswapV2Pool) -> Option<u64> {
    [
        pool.base.latest_block_number,
        pool.base.can_buy_block,
        pool.base.creation_block,
        pool.base.price_history.last().map(|(block, _)| *block),
    ]
    .into_iter()
    .flatten()
    .max()
}

fn address_set<const N: usize>(addresses: [&str; N]) -> BTreeSet<String> {
    addresses.into_iter().map(normalize_address).collect()
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::erc20::ERC20TokenMetadata;
    use crate::manager::{TokenRegistry, TrackedTokenStatus};
    use crate::pools::{BasePoolConfig, UniswapV2Pool};

    const TOKEN_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
    const POOL_ADDRESS: &str = "0x2222222222222222222222222222222222222222";
    const SECOND_POOL_ADDRESS: &str = "0x3333333333333333333333333333333333333333";

    fn token() -> ERC20Token {
        ERC20Token::with_live_mode(
            ERC20TokenMetadata::new(TOKEN_ADDRESS, "Token", "TKN", 18, "1000"),
            true,
        )
    }

    fn v2_pool(pool_address: &str, denom_address: &str, denom_reserve: f64) -> UniswapV2Pool {
        let mut pool = UniswapV2Pool::new(
            pool_address,
            TOKEN_ADDRESS,
            denom_address,
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: 0.01,
            },
            ["0x4444444444444444444444444444444444444444"],
        );
        pool.base
            .update_reserves(1_000.0, denom_reserve, 100, 1_700, "0xSYNC");
        pool
    }

    #[test]
    fn weth_pool_below_threshold_is_dropped() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.099);

        let decision = policy.evaluate_pool(&pool);

        assert!(!decision.retain);
        assert_eq!(decision.denom_class, LivePoolDenomClass::Weth);
        assert_eq!(decision.threshold, 0.1);
    }

    #[test]
    fn weth_pool_at_threshold_is_retained() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.1);

        let decision = policy.evaluate_pool(&pool);

        assert!(decision.retain);
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn stablecoin_pool_uses_stable_threshold() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, USDC_ADDRESS, 499.9);

        let decision = policy.evaluate_pool(&pool);

        assert!(!decision.retain);
        assert_eq!(decision.denom_class, LivePoolDenomClass::Stablecoin);
        assert_eq!(decision.threshold, 500.0);
    }

    #[test]
    fn unknown_denom_is_retained_by_default() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(
            POOL_ADDRESS,
            "0x5555555555555555555555555555555555555555",
            0.0,
        );

        let decision = policy.evaluate_pool(&pool);

        assert!(decision.retain);
        assert_eq!(decision.denom_class, LivePoolDenomClass::Other);
    }

    #[test]
    fn token_retains_when_any_pool_passes_threshold() {
        let policy = LiveTokenRetentionPolicy::default();
        let mut token = token();
        token.add_uniswap_v2_pool(v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.01));
        token.add_uniswap_v2_pool(v2_pool(SECOND_POOL_ADDRESS, WETH_ADDRESS, 0.2));

        let decision = policy.evaluate_token(&token, 110);

        assert!(decision.retain);
        assert_eq!(decision.retained_v2_pools, vec![SECOND_POOL_ADDRESS]);
        assert_eq!(decision.dropped_v2_pools.len(), 1);
        assert_eq!(decision.dropped_v2_pools[0].pool_address, POOL_ADDRESS);
    }

    #[test]
    fn registry_apply_prunes_pools_and_updates_index_mapping() {
        let policy = LiveTokenRetentionPolicy::default();
        let mut registry = TokenRegistry::new();
        registry.add_token_with_live_mode(
            ERC20TokenMetadata::new(TOKEN_ADDRESS, "Token", "TKN", 18, "1000"),
            true,
        );
        let token = registry.token_mut(TOKEN_ADDRESS).unwrap();
        token.add_uniswap_v2_pool(v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.01));
        token.add_uniswap_v2_pool(v2_pool(SECOND_POOL_ADDRESS, WETH_ADDRESS, 0.2));

        let mut index = TrackedTokenIndex::new(10);
        index.index_token(
            registry.token(TOKEN_ADDRESS).unwrap(),
            TrackedTokenStatus::Active,
        );
        assert_eq!(index.token_for_pool(POOL_ADDRESS), Some(TOKEN_ADDRESS));

        let report = policy.apply_to_registry(&mut registry, &mut index, 110);

        assert_eq!(report.evaluated_tokens, 1);
        assert_eq!(report.dropped_v2_pool_count, 1);
        let token = registry.token(TOKEN_ADDRESS).unwrap();
        assert!(token.uniswap_v2_pool(POOL_ADDRESS).is_none());
        assert!(token.uniswap_v2_pool(SECOND_POOL_ADDRESS).is_some());
        assert_eq!(index.token_for_pool(POOL_ADDRESS), None);
        assert_eq!(
            index.token_for_pool(SECOND_POOL_ADDRESS),
            Some(TOKEN_ADDRESS)
        );
    }

    #[test]
    fn token_without_retained_pools_can_be_dropped_after_configured_window() {
        let mut policy = LiveTokenRetentionPolicy {
            drop_tokens_without_retained_pools_after_blocks: Some(10),
            ..LiveTokenRetentionPolicy::default()
        };
        policy.min_other_denom_reserve = 1.0;

        let mut token = token();
        token.add_uniswap_v2_pool(v2_pool(
            POOL_ADDRESS,
            "0x5555555555555555555555555555555555555555",
            0.1,
        ));

        let retained = policy.evaluate_token(&token, 109);
        let dropped = policy.evaluate_token(&token, 110);

        assert!(retained.retain);
        assert!(!dropped.retain);
        assert!(matches!(
            dropped.reason,
            Some(TokenDropReason::NoRetainedPoolsPastRetentionBlocks { .. })
        ));
    }
}
