use crate::liquidity_approval_call::{
    decode_liquidity_approval_call, is_liquidity_approval_selector,
};
use crate::mempool_fetcher::MempoolTransaction;
use crate::position_approval_call::{decode_position_approval_call, is_position_approval_selector};
use crate::token_tracking::TokenTrackingCache;
use crate::unresolved_intents::UnresolvedIntentKind;
use alloy_primitives::Address as AlloyAddress;
use reth_chain_query::to_checksum_address;
use std::sync::Arc;

use super::liquidity::{
    is_known_position_manager_candidate, is_protocol_liquidity_removal_candidate,
    is_v4_modify_liquidity_candidate, liquidity_removal_token_candidates,
};
use super::metrics::{LpApprovalRouterStats, RouteOrigin, RouterMetrics};
use super::types::{
    priority_for_creator_function, should_route_tracked_token_call, ClassificationResult,
    SimulationPriority, TransactionCategory,
};
use super::{ContractCreationRouter, CreatorTransactionRouter};

pub use crate::function_detector::CreatorFunctionType;

pub struct TransactionRouter {
    contract_router: ContractCreationRouter,
    creator_router: CreatorTransactionRouter,
    token_cache: Option<Arc<TokenTrackingCache>>,
    metrics: RouterMetrics,
}

impl TransactionRouter {
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self {
            contract_router: ContractCreationRouter::new(),
            creator_router: CreatorTransactionRouter::new(token_cache.clone()),
            token_cache,
            metrics: RouterMetrics::default(),
        }
    }

    pub fn lp_approval_stats(&self) -> LpApprovalRouterStats {
        self.metrics.lp_approval_stats()
    }

    pub fn observe_route(
        &self,
        tx: &MempoolTransaction,
        classification: &ClassificationResult,
        origin: RouteOrigin,
    ) {
        self.metrics
            .observe_classification(tx, classification, origin);
    }

    pub fn unresolved_intent_for(
        &self,
        tx: &MempoolTransaction,
        classification: &ClassificationResult,
    ) -> Option<(UnresolvedIntentKind, &'static str)> {
        if !matches!(classification.category, TransactionCategory::Regular { .. }) {
            return None;
        }

        if decode_liquidity_approval_call(tx)
            .filter(|approval| approval.amount != alloy_primitives::U256::ZERO)
            .is_some()
            || decode_position_approval_call(tx)
                .map(|approval| {
                    tx.to
                        .as_ref()
                        .map(|_| is_known_position_manager_candidate(&approval.position_manager()))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        {
            return Some((
                UnresolvedIntentKind::LpApproval,
                "LP approval target is not in token cache yet",
            ));
        }

        if matches!(
            tx.function_category.as_ref(),
            Some(CreatorFunctionType::LiquidityRemoval)
        ) {
            if is_v4_modify_liquidity_candidate(tx) {
                return Some((
                    UnresolvedIntentKind::V4ModifyLiquidity,
                    "V4 liquidity tx is waiting for token/pool cache mapping",
                ));
            }
            return Some((
                UnresolvedIntentKind::LiquidityRemoval,
                "liquidity removal tx is waiting for token/pool cache mapping",
            ));
        }

        if let Some(function_type) = tx
            .function_category
            .as_ref()
            .filter(|function_type| should_route_tracked_token_call(function_type))
        {
            return Some((
                UnresolvedIntentKind::CreatorControl,
                match function_type {
                    CreatorFunctionType::TradingControl => {
                        "trading-control tx target is not in token cache yet"
                    }
                    CreatorFunctionType::TaxModification => {
                        "tax-control tx target is not in token cache yet"
                    }
                    CreatorFunctionType::MaxWalletLimit => {
                        "wallet-limit tx target is not in token cache yet"
                    }
                    CreatorFunctionType::OwnershipChange => {
                        "ownership-control tx target is not in token cache yet"
                    }
                    CreatorFunctionType::LiquidityPoolApproval => {
                        "LP approval target is not in token cache yet"
                    }
                    CreatorFunctionType::TokenSupplyModification => {
                        "supply-control tx target is not in token cache yet"
                    }
                    _ => "creator-control tx target is not in token cache yet",
                },
            ));
        }

        None
    }

    pub async fn classify(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let to_str = tx
            .to
            .as_ref()
            .map(|t| hex::encode(t))
            .unwrap_or_else(|| "contract_creation".to_string());

        if tx.to.is_none() || to_str == "0x0" || to_str.is_empty() {
            let result = self.classify_contract_creation(tx).await;
            return result;
        }

        if let Some(result) = self.classify_tracked_lp_approval(tx).await {
            return result;
        }

        if matches!(
            tx.function_category.as_ref(),
            Some(CreatorFunctionType::LiquidityPoolApproval)
        ) {
            return self.classify_regular_transaction(tx).await;
        }

        if let Some(ref cache) = self.token_cache {
            let from_addr = to_checksum_address(&AlloyAddress::from_slice(&tx.from));
            if cache.is_creator(&from_addr).await {
                let result = self.classify_creator_transaction(tx).await;
                return result;
            }
        }

        if matches!(
            tx.function_category.as_ref(),
            Some(CreatorFunctionType::LiquidityRemoval)
        ) {
            if let Some((target_token, target_address)) =
                self.tracked_liquidity_removal_target(tx).await
            {
                return ClassificationResult {
                    category: TransactionCategory::CreatorTransaction {
                        creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                        target_address,
                        target_token: Some(target_token),
                        function_type: CreatorFunctionType::LiquidityRemoval,
                    },
                    priority: SimulationPriority::Critical,
                    requires_simulation: true,
                    requires_buy_sell_test: true,
                };
            }

            if is_protocol_liquidity_removal_candidate(tx) {
                return ClassificationResult {
                    category: TransactionCategory::CreatorTransaction {
                        creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                        target_address: tx
                            .to
                            .as_ref()
                            .map(|t| to_checksum_address(&AlloyAddress::from_slice(t)))
                            .unwrap_or_else(|| "none".to_string()),
                        target_token: None,
                        function_type: CreatorFunctionType::LiquidityRemoval,
                    },
                    priority: SimulationPriority::Critical,
                    requires_simulation: true,
                    requires_buy_sell_test: false,
                };
            }
        }

        if let Some(function_type) = tx
            .function_category
            .as_ref()
            .filter(|function_type| should_route_tracked_token_call(function_type))
            .cloned()
        {
            if let Some(target_token) = self.tracked_target_token(tx).await {
                return ClassificationResult {
                    category: TransactionCategory::CreatorTransaction {
                        creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                        target_address: target_token.clone(),
                        target_token: Some(target_token),
                        function_type: function_type.clone(),
                    },
                    priority: priority_for_creator_function(&function_type),
                    requires_simulation: true,
                    requires_buy_sell_test: true,
                };
            }
        }

        let result = self.classify_regular_transaction(tx).await;
        result
    }

    async fn classify_tracked_lp_approval(
        &self,
        tx: &MempoolTransaction,
    ) -> Option<ClassificationResult> {
        if let Some(classification) = self.classify_tracked_position_approval(tx).await {
            return Some(classification);
        }

        let approval = decode_liquidity_approval_call(tx)?;
        if approval.amount == alloy_primitives::U256::ZERO {
            return None;
        }

        let Some(ref cache) = self.token_cache else {
            return None;
        };
        let target_address = to_checksum_address(&approval.ownership_token);

        let Some(pool) = cache
            .get_pool_by_liquidity_ownership_token(&target_address)
            .await
        else {
            return None;
        };

        Some(ClassificationResult {
            category: TransactionCategory::CreatorTransaction {
                creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                target_address: pool.address.clone(),
                target_token: Some(pool.token_address.clone()),
                function_type: CreatorFunctionType::LiquidityPoolApproval,
            },
            priority: SimulationPriority::Critical,
            requires_simulation: false,
            requires_buy_sell_test: false,
        })
    }

    async fn classify_tracked_position_approval(
        &self,
        tx: &MempoolTransaction,
    ) -> Option<ClassificationResult> {
        let approval = decode_position_approval_call(tx)?;
        let Some(ref cache) = self.token_cache else {
            return None;
        };
        let position_manager = to_checksum_address(&approval.position_manager());
        if !is_known_position_manager_candidate(&approval.position_manager())
            && !cache.is_position_manager(&position_manager).await
        {
            return None;
        }
        if !cache.is_position_manager(&position_manager).await {
            return None;
        }

        Some(ClassificationResult {
            category: TransactionCategory::CreatorTransaction {
                creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                target_address: position_manager,
                target_token: None,
                function_type: CreatorFunctionType::LiquidityPoolApproval,
            },
            priority: SimulationPriority::Critical,
            requires_simulation: false,
            requires_buy_sell_test: false,
        })
    }

    async fn tracked_liquidity_removal_target(
        &self,
        tx: &MempoolTransaction,
    ) -> Option<(String, String)> {
        let cache = self.token_cache.as_ref()?;
        for candidate in liquidity_removal_token_candidates(&tx.input) {
            if cache.get_token(&candidate).await.is_some() {
                let target_address = tx
                    .to
                    .as_ref()
                    .map(|t| to_checksum_address(&AlloyAddress::from_slice(t)))
                    .unwrap_or_else(|| "none".to_string());
                return Some((candidate, target_address));
            }
        }

        if let Some(target_address) = tx
            .to
            .as_ref()
            .map(|address| to_checksum_address(&AlloyAddress::from_slice(address)))
        {
            if let Some(pool) = cache.get_pool_by_address(&target_address).await {
                return Some((pool.token_address.clone(), pool.address.clone()));
            }

            if tx.input.get(0..4) == Some([0x8b, 0xdb, 0x39, 0x13].as_slice())
                && tx.input.len() >= 36
            {
                let pool_identifier =
                    format!("{}#0x{}", target_address, hex::encode(&tx.input[4..36]));
                if let Some(pool) = cache.get_pool_by_address(&pool_identifier).await {
                    return Some((pool.token_address.clone(), pool.address.clone()));
                }
            }
        }

        None
    }

    async fn tracked_target_token(&self, tx: &MempoolTransaction) -> Option<String> {
        let cache = self.token_cache.as_ref()?;
        let target = tx
            .to
            .as_ref()
            .map(|address| format!("0x{}", hex::encode(address)))?;
        if let Some(pool) = cache.get_pool_by_address(&target).await {
            return Some(pool.token_address.clone());
        }
        cache
            .get_token(&target)
            .await
            .map(|token| token.address.clone())
    }

    async fn classify_contract_creation(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let (is_token, has_liquidity) = self.contract_router.analyze_creation(tx);

        ClassificationResult {
            category: TransactionCategory::ContractCreation {
                deployer: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                contract_address: "pending".to_string(), // Will be determined after execution
                is_token,
                has_liquidity_in_calldata: has_liquidity,
            },
            priority: if is_token {
                SimulationPriority::High
            } else {
                SimulationPriority::Low
            },
            requires_simulation: true, // Track creations so we can stitch them into pending sequences
            requires_buy_sell_test: false, // Can't test until deployed
        }
    }

    async fn classify_creator_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let function_type = self.creator_router.get_function_type(tx);

        let target_token = if let Some(ref cache) = self.token_cache {
            let from_addr = to_checksum_address(&AlloyAddress::from_slice(&tx.from));

            if let Some(token_info) = cache.get_token_for_creator(&from_addr).await {
                Some(token_info.address.clone())
            } else {
                None
            }
        } else {
            None
        };

        let is_eth_transfer =
            matches!(&function_type, CreatorFunctionType::Other(s) if s == "eth_transfer");

        let priority = priority_for_creator_function(&function_type);

        let is_lp_approval = matches!(&function_type, CreatorFunctionType::LiquidityPoolApproval);
        let requires_buy_sell = !is_eth_transfer && !is_lp_approval;

        ClassificationResult {
            category: TransactionCategory::CreatorTransaction {
                creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                target_address: tx
                    .to
                    .as_ref()
                    .map(|t| to_checksum_address(&AlloyAddress::from_slice(t)))
                    .unwrap_or_else(|| "none".to_string()),
                target_token,
                function_type,
            },
            priority,
            requires_simulation: !is_eth_transfer && !is_lp_approval,
            requires_buy_sell_test: requires_buy_sell,
        }
    }

    async fn classify_regular_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let input_data = &tx.input;

        let is_transfer = input_data.len() >= 4
            && (&input_data[0..4] == &[0xa9, 0x05, 0x9c, 0xbb]
                || &input_data[0..4] == &[0x23, 0xb8, 0x72, 0xdd]);

        let is_approval = input_data
            .get(0..4)
            .map(|selector| {
                is_liquidity_approval_selector(selector) || is_position_approval_selector(selector)
            })
            .unwrap_or(false);

        ClassificationResult {
            category: TransactionCategory::Regular {
                is_transfer,
                is_approval,
            },
            priority: SimulationPriority::Low,
            requires_simulation: false,
            requires_buy_sell_test: false,
        }
    }
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod protocol_tests;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    use alloy_primitives::U256;
    use serde_json::json;

    use super::{
        ClassificationResult, CreatorFunctionType, RouteOrigin, SimulationPriority,
        TransactionCategory, TransactionRouter,
    };
    use crate::mempool_fetcher::MempoolTransaction;
    use crate::token_tracking::types::PoolLifecycle;
    use crate::token_tracking::{
        Pool, PoolType, Token, TokenTrackingCache, TokenUpdate, TokenWithPools,
    };

    #[tokio::test]
    async fn routes_tracked_liquidity_removal_from_non_creator() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let token_address = "0x1111111111111111111111111111111111111111".to_string();
        hydrate_token(&cache, &token_address).await;

        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x7a250d5630b4cf539739df2c5dacb4c659f2488d")),
            input: remove_liquidity_eth_calldata(&token_address),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["removeLiquidityETH".to_string()],
            function_category: Some(CreatorFunctionType::LiquidityRemoval),
        };

        let classification = router.classify(&tx).await;
        assert!(classification.requires_simulation);
        match classification.category {
            TransactionCategory::CreatorTransaction {
                target_token,
                function_type,
                ..
            } => {
                assert_eq!(target_token.as_deref(), Some(token_address.as_str()));
                assert_eq!(function_type, CreatorFunctionType::LiquidityRemoval);
            }
            other => panic!("unexpected category: {other:?}"),
        }
    }

    #[tokio::test]
    async fn routes_uniswap_v3_decrease_liquidity_without_token_param() {
        let router = TransactionRouter::new(Some(Arc::new(TokenTrackingCache::with_defaults())));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")),
            input: hex::decode("0c49ccbe").unwrap(),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["decreaseLiquidity".to_string()],
            function_category: Some(CreatorFunctionType::LiquidityRemoval),
        };

        let classification = router.classify(&tx).await;
        assert_eq!(classification.priority, SimulationPriority::Critical);
        assert!(classification.requires_simulation);
        assert!(!classification.requires_buy_sell_test);
        match classification.category {
            TransactionCategory::CreatorTransaction {
                target_token,
                function_type,
                ..
            } => {
                assert_eq!(target_token, None);
                assert_eq!(function_type, CreatorFunctionType::LiquidityRemoval);
            }
            other => panic!("unexpected category: {other:?}"),
        }
    }

    #[tokio::test]
    async fn routes_uniswap_v3_position_manager_multicall_for_protocol_removal_scan() {
        let router = TransactionRouter::new(Some(Arc::new(TokenTrackingCache::with_defaults())));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")),
            input: hex::decode("ac9650d8").unwrap(),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["multicall".to_string()],
            function_category: Some(CreatorFunctionType::LiquidityRemoval),
        };

        let classification = router.classify(&tx).await;
        assert_eq!(classification.priority, SimulationPriority::Critical);
        assert!(classification.requires_simulation);
        assert!(!classification.requires_buy_sell_test);
    }

    #[tokio::test]
    async fn routes_trading_control_to_tracked_token_from_non_creator() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let token_address = "0x1111111111111111111111111111111111111111".to_string();
        hydrate_token(&cache, &token_address).await;

        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes(&token_address)),
            input: hex::decode("8a8c523c").unwrap(),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["enableTrading".to_string()],
            function_category: Some(CreatorFunctionType::TradingControl),
        };

        let classification = router.classify(&tx).await;
        assert!(classification.requires_simulation);
        assert!(classification.requires_buy_sell_test);
        assert_eq!(classification.priority, super::SimulationPriority::Critical);
        match classification.category {
            TransactionCategory::CreatorTransaction {
                creator,
                target_address,
                target_token,
                function_type,
            } => {
                assert_eq!(creator, "0x2222222222222222222222222222222222222222");
                assert_eq!(target_address, token_address);
                assert_eq!(target_token.as_deref(), Some(token_address.as_str()));
                assert_eq!(function_type, CreatorFunctionType::TradingControl);
            }
            other => panic!("unexpected category: {other:?}"),
        }
    }

    #[tokio::test]
    async fn routes_tracked_lp_approval_from_non_creator_without_simulation() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let token_address = "0x1111111111111111111111111111111111111111".to_string();
        let pool_address = "0x4444444444444444444444444444444444444444".to_string();
        hydrate_token_with_pool(&cache, &token_address, &pool_address).await;

        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes(&pool_address)),
            input: approve_calldata(
                "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
                U256::from(1_000_000u64),
            ),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["approve".to_string()],
            function_category: Some(CreatorFunctionType::Other("approve".to_string())),
        };

        let classification = router.classify(&tx).await;
        assert_eq!(classification.priority, SimulationPriority::Critical);
        assert!(!classification.requires_simulation);
        assert!(!classification.requires_buy_sell_test);
        router.observe_route(&tx, &classification, RouteOrigin::MempoolIngress);
        match classification.category {
            TransactionCategory::CreatorTransaction {
                target_address,
                target_token,
                function_type,
                ..
            } => {
                assert_eq!(target_address, pool_address);
                assert_eq!(target_token.as_deref(), Some(token_address.as_str()));
                assert_eq!(function_type, CreatorFunctionType::LiquidityPoolApproval);
            }
            other => panic!("unexpected category: {other:?}"),
        }

        let stats = router.lp_approval_stats();
        assert_eq!(stats.ingress_erc20_approval_txs, 1);
        assert_eq!(stats.ingress_ownership_token_pool_hits, 1);
        assert_eq!(stats.ingress_pool_cache_miss_txs, 0);
    }

    #[tokio::test]
    async fn counts_router_approval_pool_cache_miss() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x5555555555555555555555555555555555555555")),
            input: approve_calldata(
                "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
                U256::from(1u64),
            ),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["approve".to_string()],
            function_category: Some(CreatorFunctionType::Other("approve".to_string())),
        };

        let classification = router.classify(&tx).await;
        assert!(matches!(
            classification.category,
            TransactionCategory::Regular {
                is_approval: true,
                ..
            }
        ));

        router.observe_route(&tx, &classification, RouteOrigin::MempoolIngress);
        let stats = router.lp_approval_stats();
        assert_eq!(stats.ingress_erc20_approval_txs, 1);
        assert_eq!(stats.ingress_ownership_token_pool_hits, 0);
        assert_eq!(stats.ingress_pool_cache_miss_txs, 1);
    }

    #[tokio::test]
    async fn separates_ingress_tx_counts_from_unresolved_retry_attempts() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x5555555555555555555555555555555555555555")),
            input: approve_calldata(
                "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
                U256::from(1u64),
            ),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["approve".to_string()],
            function_category: Some(CreatorFunctionType::Other("approve".to_string())),
        };

        let classification = router.classify(&tx).await;
        router.observe_route(&tx, &classification, RouteOrigin::MempoolIngress);
        router.observe_route(&tx, &classification, RouteOrigin::UnresolvedRetry);

        let stats = router.lp_approval_stats();
        assert_eq!(stats.ingress_erc20_approval_txs, 1);
        assert_eq!(stats.ingress_pool_cache_miss_txs, 1);
        assert_eq!(stats.retry_erc20_approval_attempts, 1);
        assert_eq!(stats.retry_pool_cache_miss_attempts, 1);
    }

    #[tokio::test]
    async fn exposes_unresolved_lp_approval_intent_on_cache_miss() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x5555555555555555555555555555555555555555")),
            input: approve_calldata(
                "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
                U256::from(1u64),
            ),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["approve".to_string()],
            function_category: Some(CreatorFunctionType::Other("approve".to_string())),
        };

        let classification = router.classify(&tx).await;
        let (kind, _) = router
            .unresolved_intent_for(&tx, &classification)
            .expect("LP approval should become unresolved intent");
        assert_eq!(
            kind,
            crate::unresolved_intents::UnresolvedIntentKind::LpApproval
        );
    }

    #[tokio::test]
    async fn exposes_unresolved_v4_modify_liquidity_intent() {
        let router = TransactionRouter::new(Some(Arc::new(TokenTrackingCache::with_defaults())));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x000000000004444c5dc75cb358380d2e3de08a90")),
            input: hex::decode("dd46508f").unwrap(),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["modifyLiquidities".to_string()],
            function_category: Some(CreatorFunctionType::LiquidityRemoval),
        };

        let classification = router.classify(&tx).await;
        assert!(matches!(
            classification.category,
            TransactionCategory::CreatorTransaction { .. }
        ));

        let regular = ClassificationResult {
            category: TransactionCategory::Regular {
                is_transfer: false,
                is_approval: false,
            },
            priority: SimulationPriority::Low,
            requires_simulation: false,
            requires_buy_sell_test: false,
        };
        let (kind, _) = router
            .unresolved_intent_for(&tx, &regular)
            .expect("V4 modify liquidity should be identifiable as unresolved");
        assert_eq!(
            kind,
            crate::unresolved_intents::UnresolvedIntentKind::V4ModifyLiquidity
        );
    }

    #[tokio::test]
    async fn does_not_promote_tracked_token_approval_as_lp_approval() {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let token_address = "0x1111111111111111111111111111111111111111".to_string();
        hydrate_token(&cache, &token_address).await;

        let router = TransactionRouter::new(Some(cache));
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes(&token_address)),
            input: approve_calldata(
                "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
                U256::from(1u64),
            ),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: vec!["approve".to_string()],
            function_category: Some(CreatorFunctionType::LiquidityPoolApproval),
        };

        let classification = router.classify(&tx).await;
        assert!(matches!(
            classification.category,
            TransactionCategory::Regular {
                is_approval: true,
                ..
            }
        ));
        assert!(!classification.requires_simulation);
        assert!(!classification.requires_buy_sell_test);

        router.observe_route(&tx, &classification, RouteOrigin::MempoolIngress);
        let stats = router.lp_approval_stats();
        assert_eq!(stats.ingress_erc20_approval_txs, 1);
        assert_eq!(stats.ingress_ownership_token_pool_hits, 0);
        assert_eq!(stats.ingress_pool_cache_miss_txs, 1);
    }

    async fn hydrate_token(cache: &TokenTrackingCache, token_address: &str) {
        hydrate_token_with_pool(
            cache,
            token_address,
            "0x4444444444444444444444444444444444444444",
        )
        .await;
    }

    async fn hydrate_token_with_pool(
        cache: &TokenTrackingCache,
        token_address: &str,
        pool_address: &str,
    ) {
        let creator_address = "0x3333333333333333333333333333333333333333".to_string();
        let token = Token {
            address: token_address.to_string(),
            symbol: "TEST".to_string(),
            name: "Test".to_string(),
            decimals: 18,
            total_supply: Some("1000".to_string()),
            creator_address: creator_address.clone(),
            current_owner: creator_address,
            tax_setter_addresses: Vec::new(),
            ownership_renounced: false,
            renouncement_block: None,
            buy_tax: None,
            sell_tax: None,
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: 1,
            creation_tx: "0xcreation".to_string(),
            creation_timestamp: None,
            latest_activity_block: 1,
            is_scam: false,
            scam_label: None,
            total_liquidity: 0.0,
        };
        let pool = Pool {
            address: pool_address.to_string(),
            token_address: token_address.to_string(),
            pool_type: PoolType::UniswapV2,
            token_reserve: 1_000.0,
            eth_reserve: 1.0,
            denom_currency: "ETH".to_string(),
            denom_address: "0x0000000000000000000000000000000000000000".to_string(),
            trading_enabled: true,
            trading_enabled_block: Some(1),
            trading_enabled_tx: None,
            fee_tier: None,
            pool_id: None,
            lp_token_address: None,
            position_manager_address: None,
            lp_total_supply: None,
            liquidity_positions: Vec::new(),
            last_updated_block: 1,
            last_updated_time: 0.0,
            is_scam: false,
            scam_label: None,
            lp_tokens_approved_percentage: None,
            lifecycle: PoolLifecycle::Active,
            control_addresses: Vec::new(),
            can_buy: true,
            can_sell: true,
            received_at: Instant::now(),
        };

        let mut pools = HashMap::new();
        pools.insert(pool_address.to_string(), pool);
        let mut data = HashMap::new();
        data.insert(token_address.to_string(), TokenWithPools { token, pools });
        cache
            .batch_update(TokenUpdate {
                message_type: "test".to_string(),
                token_count: 1,
                block_number: 1,
                timestamp: 0.0,
                data,
            })
            .await;
    }

    fn remove_liquidity_eth_calldata(token_address: &str) -> Vec<u8> {
        let mut input = hex::decode("02751cec").unwrap();
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(&address_bytes(token_address));
        input
    }

    fn address_bytes(address: &str) -> Vec<u8> {
        hex::decode(address.trim_start_matches("0x")).unwrap()
    }

    fn approve_calldata(spender: &str, amount: U256) -> Vec<u8> {
        let mut input = hex::decode("095ea7b3").unwrap();
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(&address_bytes(spender));
        let amount_bytes = amount.to_be_bytes::<32>();
        input.extend_from_slice(&amount_bytes);
        input
    }
}
