use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::erc20::ERC20Token;
use crate::pools::BasePool;

pub const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
pub const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
pub const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
pub const DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS: u64 = 15_000;
const SIGNIFICANT_LIQUIDITY_DROP_RATIO: f64 = 0.80;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenRetentionPolicy {
    pub min_weth_denom_reserve: f64,
    pub min_stable_denom_reserve: f64,
    pub min_other_denom_reserve: f64,
    pub weth_denoms: BTreeSet<String>,
    pub stablecoin_denoms: BTreeSet<String>,
    pub drop_tokens_without_pools_after_blocks: Option<u64>,
    pub drop_tokens_without_retained_pools_after_blocks: Option<u64>,
    #[serde(default = "default_liquidity_removal_retention_blocks")]
    pub retain_liquidity_removal_pools_for_blocks: Option<u64>,
}

impl Default for LiveTokenRetentionPolicy {
    fn default() -> Self {
        Self {
            min_weth_denom_reserve: 0.1,
            min_stable_denom_reserve: 1_000.0,
            min_other_denom_reserve: 0.0,
            weth_denoms: address_set([WETH_ADDRESS]),
            stablecoin_denoms: address_set([USDC_ADDRESS, USDT_ADDRESS, DAI_ADDRESS]),
            drop_tokens_without_pools_after_blocks: None,
            drop_tokens_without_retained_pools_after_blocks: None,
            retain_liquidity_removal_pools_for_blocks: default_liquidity_removal_retention_blocks(),
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

    pub fn evaluate_pool(&self, pool: &BasePool, current_block: u64) -> LivePoolRetentionDecision {
        let pool_address = pool.identity.pool_address.clone();
        let denom_address = pool.identity.denom_address.clone();
        let denom_class = self.denom_class(&denom_address);
        let denom_reserve = pool.denom_reserve();
        let threshold = self.denom_threshold(&denom_address);
        let reason = if let Some(reason) = self.liquidity_removal_expiry_reason(pool, current_block)
        {
            Some(reason)
        } else if pool.has_liquidity_removal() || threshold <= 0.0 || denom_reserve >= threshold {
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
            retain: reason.is_none(),
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
            .all_pool_bases()
            .into_iter()
            .map(|pool| self.evaluate_pool(pool, current_block))
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
        } else if let Some(reason) = self.liquidity_removal_token_expiry_reason(&dropped_v2_pools) {
            Some(reason)
        } else if !token.has_pool() {
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
        self.mark_liquidity_removal_evidence(token);
        let decision = self.evaluate_token(token, current_block);
        for pool in &decision.dropped_v2_pools {
            token.v2_pools.remove(&pool.pool_address);
            token.v3_pools.remove(&pool.pool_address);
            token.v4_pools.remove(&pool.pool_address);
            token.curve_pools.remove(&pool.pool_address);
            token.balancer_pools.remove(&pool.pool_address);
        }
        token.refresh_lifecycle_status();
        decision
    }

    fn liquidity_removal_token_expiry_reason(
        &self,
        dropped_pools: &[LivePoolRetentionDecision],
    ) -> Option<TokenDropReason> {
        dropped_pools
            .iter()
            .find_map(|pool| match pool.reason.as_ref() {
                Some(PoolDropReason::LiquidityRemovalRetentionExpired {
                    removal_block,
                    current_block,
                    retention_blocks,
                }) => Some(TokenDropReason::LiquidityRemovalRetentionExpired {
                    removal_block: *removal_block,
                    current_block: *current_block,
                    retention_blocks: *retention_blocks,
                }),
                _ => None,
            })
    }

    fn mark_liquidity_removal_evidence(&self, token: &mut ERC20Token) {
        for pool in token.v2_pools.values_mut() {
            self.mark_pool_liquidity_removal_evidence(&mut pool.base);
        }
        for pool in token.v3_pools.values_mut() {
            self.mark_pool_liquidity_removal_evidence(&mut pool.base);
        }
        for pool in token.v4_pools.values_mut() {
            self.mark_pool_liquidity_removal_evidence(&mut pool.base);
        }
        for pool in token.curve_pools.values_mut() {
            self.mark_pool_liquidity_removal_evidence(&mut pool.base);
        }
        for pool in token.balancer_pools.values_mut() {
            self.mark_pool_liquidity_removal_evidence(&mut pool.base);
        }
    }

    fn mark_pool_liquidity_removal_evidence(&self, pool: &mut BasePool) {
        if pool.has_liquidity_removal() {
            return;
        }

        let threshold = self.denom_threshold(&pool.identity.denom_address);
        let Some(evidence) = reserve_drop_evidence(pool, threshold) else {
            return;
        };
        let label = format!(
            "liquidity_removal (retention threshold {}, drop {:.2}%)",
            display_threshold(evidence.threshold),
            evidence.drop_ratio * 100.0
        );
        pool.mark_liquidity_removal(label, Some(evidence.block_number), Some(evidence.tx_hash));
    }

    fn liquidity_removal_expiry_reason(
        &self,
        pool: &BasePool,
        current_block: u64,
    ) -> Option<PoolDropReason> {
        if !pool.has_liquidity_removal() {
            return None;
        }
        let retention_blocks = self.retain_liquidity_removal_pools_for_blocks?;
        let removal_block = pool.scam_block?;
        if current_block.saturating_sub(removal_block) < retention_blocks {
            return None;
        }
        Some(PoolDropReason::LiquidityRemovalRetentionExpired {
            removal_block,
            current_block,
            retention_blocks,
        })
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

#[derive(Clone, Debug, PartialEq)]
struct ReserveDropEvidence {
    threshold: f64,
    drop_ratio: f64,
    block_number: u64,
    tx_hash: String,
}

fn reserve_drop_evidence(pool: &BasePool, threshold: f64) -> Option<ReserveDropEvidence> {
    if threshold <= 0.0 || !threshold.is_finite() {
        return None;
    }

    let latest = pool.reserve_tracker.latest_snapshot.as_ref()?;
    let current = latest.denom_reserve.max(0.0);
    if !current.is_finite() || current >= threshold {
        return None;
    }

    let max_seen = pool
        .reserve_tracker
        .reserve_history
        .iter()
        .map(|snapshot| snapshot.denom_reserve)
        .filter(|reserve| reserve.is_finite())
        .fold(current, f64::max);

    if max_seen < threshold || max_seen <= 0.0 {
        return None;
    }

    let drop_ratio = ((max_seen - current) / max_seen).max(0.0);
    if drop_ratio < SIGNIFICANT_LIQUIDITY_DROP_RATIO {
        return None;
    }

    Some(ReserveDropEvidence {
        threshold,
        drop_ratio,
        block_number: latest.block_number,
        tx_hash: latest.tx_hash.clone(),
    })
}

fn display_threshold(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn default_liquidity_removal_retention_blocks() -> Option<u64> {
    Some(DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS)
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
    LiquidityRemovalRetentionExpired {
        removal_block: u64,
        current_block: u64,
        retention_blocks: u64,
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
    LiquidityRemovalRetentionExpired {
        removal_block: u64,
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
        .all_pool_bases()
        .into_iter()
        .filter_map(pool_reference_block)
        .max()
        .or_else(|| token_reference_block(token))
}

fn pool_reference_block(pool: &BasePool) -> Option<u64> {
    [
        pool.latest_block_number,
        pool.can_buy_block,
        pool.creation_block,
        pool.price_history.last().map(|(block, _)| *block),
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

    fn drained_v2_pool(pool_address: &str, denom_address: &str) -> UniswapV2Pool {
        let mut pool = v2_pool(pool_address, denom_address, 1.2661295);
        pool.base
            .update_reserves(1_000.0, 0.0196108, 101, 1_712, "0xDRAIN");
        pool
    }

    #[test]
    fn weth_pool_below_threshold_is_dropped() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.099);

        let decision = policy.evaluate_pool(&pool.base, 110);

        assert!(!decision.retain);
        assert_eq!(decision.denom_class, LivePoolDenomClass::Weth);
        assert_eq!(decision.threshold, 0.1);
    }

    #[test]
    fn weth_pool_at_threshold_is_retained() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.1);

        let decision = policy.evaluate_pool(&pool.base, 110);

        assert!(decision.retain);
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn stablecoin_pool_uses_stable_threshold() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(POOL_ADDRESS, USDC_ADDRESS, 999.9);

        let decision = policy.evaluate_pool(&pool.base, 110);

        assert!(!decision.retain);
        assert_eq!(decision.denom_class, LivePoolDenomClass::Stablecoin);
        assert_eq!(decision.threshold, 1_000.0);
    }

    #[test]
    fn unknown_denom_is_retained_by_default() {
        let policy = LiveTokenRetentionPolicy::default();
        let pool = v2_pool(
            POOL_ADDRESS,
            "0x5555555555555555555555555555555555555555",
            0.0,
        );

        let decision = policy.evaluate_pool(&pool.base, 110);

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
    fn drained_pool_is_marked_and_retained_as_liquidity_removal_evidence() {
        let policy = LiveTokenRetentionPolicy::default();
        let mut token = token();
        token.add_uniswap_v2_pool(drained_v2_pool(POOL_ADDRESS, WETH_ADDRESS));

        let decision = policy.apply_to_token(&mut token, 101);
        let pool = token.uniswap_v2_pool(POOL_ADDRESS).unwrap();

        assert!(decision.retain);
        assert_eq!(decision.retained_v2_pools, vec![POOL_ADDRESS]);
        assert!(decision.dropped_v2_pools.is_empty());
        assert!(pool.base.has_liquidity_removal());
        assert_eq!(pool.base.scam_block, Some(101));
        assert_eq!(pool.base.scam_tx_hash.as_deref(), Some("0xDRAIN"));
        assert_eq!(token.liquidity_removal_pool_count(), 1);
        assert!(token.is_scam());
    }

    #[test]
    fn liquidity_removal_pool_is_dropped_after_retention_window() {
        let policy = LiveTokenRetentionPolicy::default();
        let mut token = token();
        token.add_uniswap_v2_pool(drained_v2_pool(POOL_ADDRESS, WETH_ADDRESS));
        policy.apply_to_token(&mut token, 101);

        let decision =
            policy.apply_to_token(&mut token, 101 + DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS);

        assert!(!decision.retain);
        assert!(decision.retained_v2_pools.is_empty());
        assert_eq!(decision.dropped_v2_pools.len(), 1);
        assert!(matches!(
            decision.dropped_v2_pools[0].reason,
            Some(PoolDropReason::LiquidityRemovalRetentionExpired {
                removal_block: 101,
                current_block,
                retention_blocks: DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS,
            }) if current_block == 101 + DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS
        ));
        assert!(matches!(
            decision.reason,
            Some(TokenDropReason::LiquidityRemovalRetentionExpired {
                removal_block: 101,
                current_block,
                retention_blocks: DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS,
            }) if current_block == 101 + DEFAULT_LIQUIDITY_REMOVAL_RETENTION_BLOCKS
        ));
        assert!(token.uniswap_v2_pool(POOL_ADDRESS).is_none());
        assert_eq!(token.liquidity_removal_pool_count(), 0);
    }

    #[test]
    fn dropping_last_pool_refreshes_token_lifecycle() {
        let policy = LiveTokenRetentionPolicy::default();
        let mut token = token();
        token.handle_contract_creation(90, 1_600, "0xCREATE", TOKEN_ADDRESS, 0);
        token.add_uniswap_v2_pool(v2_pool(POOL_ADDRESS, WETH_ADDRESS, 0.01));
        assert!(token.has_pool());

        let decision = policy.apply_to_token(&mut token, 110);

        assert!(decision.retain);
        assert!(!token.has_pool());
        assert_eq!(
            token.token_life_cycle_status,
            Some(crate::erc20::TokenLifecycleState::ContractCreation)
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
