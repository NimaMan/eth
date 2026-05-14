use std::sync::atomic::{AtomicU64, Ordering};

use alloy_primitives::U256;

use crate::function_detector::CreatorFunctionType;
use crate::liquidity_approval_call::decode_liquidity_approval_call;
use crate::mempool_fetcher::MempoolTransaction;
use crate::position_approval_call::decode_position_approval_call;

use super::liquidity::is_known_position_manager_candidate;
use super::types::{ClassificationResult, TransactionCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteOrigin {
    MempoolIngress,
    UnresolvedRetry,
}

#[derive(Debug, Clone, Default)]
pub struct LpApprovalRouterStats {
    pub ingress_erc20_approval_txs: u64,
    pub ingress_ownership_token_pool_hits: u64,
    pub ingress_position_approval_txs: u64,
    pub ingress_position_manager_hits: u64,
    pub ingress_pool_cache_miss_txs: u64,
    pub retry_erc20_approval_attempts: u64,
    pub retry_ownership_token_pool_hits: u64,
    pub retry_position_approval_attempts: u64,
    pub retry_position_manager_hits: u64,
    pub retry_pool_cache_miss_attempts: u64,
}

#[derive(Default)]
pub(crate) struct RouterMetrics {
    ingress_erc20_approval_txs: AtomicU64,
    ingress_ownership_token_pool_hits: AtomicU64,
    ingress_position_approval_txs: AtomicU64,
    ingress_position_manager_hits: AtomicU64,
    ingress_pool_cache_miss_txs: AtomicU64,
    retry_erc20_approval_attempts: AtomicU64,
    retry_ownership_token_pool_hits: AtomicU64,
    retry_position_approval_attempts: AtomicU64,
    retry_position_manager_hits: AtomicU64,
    retry_pool_cache_miss_attempts: AtomicU64,
}

impl RouterMetrics {
    pub(crate) fn observe_classification(
        &self,
        tx: &MempoolTransaction,
        classification: &ClassificationResult,
        origin: RouteOrigin,
    ) {
        if self.observe_position_approval(tx, classification, origin) {
            return;
        }
        self.observe_erc20_liquidity_approval(tx, classification, origin);
    }

    pub(crate) fn lp_approval_stats(&self) -> LpApprovalRouterStats {
        LpApprovalRouterStats {
            ingress_erc20_approval_txs: self.ingress_erc20_approval_txs.load(Ordering::Relaxed),
            ingress_ownership_token_pool_hits: self
                .ingress_ownership_token_pool_hits
                .load(Ordering::Relaxed),
            ingress_position_approval_txs: self
                .ingress_position_approval_txs
                .load(Ordering::Relaxed),
            ingress_position_manager_hits: self
                .ingress_position_manager_hits
                .load(Ordering::Relaxed),
            ingress_pool_cache_miss_txs: self.ingress_pool_cache_miss_txs.load(Ordering::Relaxed),
            retry_erc20_approval_attempts: self
                .retry_erc20_approval_attempts
                .load(Ordering::Relaxed),
            retry_ownership_token_pool_hits: self
                .retry_ownership_token_pool_hits
                .load(Ordering::Relaxed),
            retry_position_approval_attempts: self
                .retry_position_approval_attempts
                .load(Ordering::Relaxed),
            retry_position_manager_hits: self.retry_position_manager_hits.load(Ordering::Relaxed),
            retry_pool_cache_miss_attempts: self
                .retry_pool_cache_miss_attempts
                .load(Ordering::Relaxed),
        }
    }

    fn observe_position_approval(
        &self,
        tx: &MempoolTransaction,
        classification: &ClassificationResult,
        origin: RouteOrigin,
    ) -> bool {
        let Some(approval) = decode_position_approval_call(tx) else {
            return false;
        };

        let is_position_route = is_position_liquidity_approval_route(&classification.category);
        if !is_position_route && !is_known_position_manager_candidate(&approval.position_manager())
        {
            return false;
        }

        self.position_approval_counter(origin)
            .fetch_add(1, Ordering::Relaxed);
        if is_position_route {
            self.position_manager_hits(origin)
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.pool_cache_miss_counter(origin)
                .fetch_add(1, Ordering::Relaxed);
        }
        true
    }

    fn observe_erc20_liquidity_approval(
        &self,
        tx: &MempoolTransaction,
        classification: &ClassificationResult,
        origin: RouteOrigin,
    ) {
        let Some(approval) = decode_liquidity_approval_call(tx) else {
            return;
        };
        if approval.amount == U256::ZERO {
            return;
        }

        self.erc20_approval_counter(origin)
            .fetch_add(1, Ordering::Relaxed);
        if is_erc20_liquidity_approval_route(&classification.category) {
            self.ownership_token_pool_hits(origin)
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.pool_cache_miss_counter(origin)
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    fn erc20_approval_counter(&self, origin: RouteOrigin) -> &AtomicU64 {
        match origin {
            RouteOrigin::MempoolIngress => &self.ingress_erc20_approval_txs,
            RouteOrigin::UnresolvedRetry => &self.retry_erc20_approval_attempts,
        }
    }

    fn ownership_token_pool_hits(&self, origin: RouteOrigin) -> &AtomicU64 {
        match origin {
            RouteOrigin::MempoolIngress => &self.ingress_ownership_token_pool_hits,
            RouteOrigin::UnresolvedRetry => &self.retry_ownership_token_pool_hits,
        }
    }

    fn position_approval_counter(&self, origin: RouteOrigin) -> &AtomicU64 {
        match origin {
            RouteOrigin::MempoolIngress => &self.ingress_position_approval_txs,
            RouteOrigin::UnresolvedRetry => &self.retry_position_approval_attempts,
        }
    }

    fn position_manager_hits(&self, origin: RouteOrigin) -> &AtomicU64 {
        match origin {
            RouteOrigin::MempoolIngress => &self.ingress_position_manager_hits,
            RouteOrigin::UnresolvedRetry => &self.retry_position_manager_hits,
        }
    }

    fn pool_cache_miss_counter(&self, origin: RouteOrigin) -> &AtomicU64 {
        match origin {
            RouteOrigin::MempoolIngress => &self.ingress_pool_cache_miss_txs,
            RouteOrigin::UnresolvedRetry => &self.retry_pool_cache_miss_attempts,
        }
    }
}

fn is_erc20_liquidity_approval_route(category: &TransactionCategory) -> bool {
    matches!(
        category,
        TransactionCategory::CreatorTransaction {
            target_token: Some(_),
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    )
}

fn is_position_liquidity_approval_route(category: &TransactionCategory) -> bool {
    matches!(
        category,
        TransactionCategory::CreatorTransaction {
            target_token: None,
            function_type: CreatorFunctionType::LiquidityPoolApproval,
            ..
        }
    )
}
