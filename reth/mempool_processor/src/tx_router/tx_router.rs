use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::Address as AlloyAddress;
use reth_chain_query::to_checksum_address;
/// Main Transaction Router
///
/// Routes transactions to appropriate simulation strategies based on their
/// characteristics and assigns processing priorities
use std::sync::Arc;

use super::{ContractCreationRouter, CreatorTransactionRouter};

/// Categories of transactions for processing
#[derive(Debug, Clone)]
pub enum TransactionCategory {
    /// Contract creation transaction
    ContractCreation {
        deployer: String,
        contract_address: String,
        is_token: bool,
        has_liquidity_in_calldata: bool,
    },
    /// Transaction from a known token creator
    CreatorTransaction {
        creator: String,
        target_address: String,
        target_token: Option<String>,
        function_type: CreatorFunctionType,
    },
    /// Regular transaction (transfer, approval, etc)
    Regular {
        is_transfer: bool,
        is_approval: bool,
    },
}

// CreatorFunctionType moved to function_detector module
pub use crate::function_detector::CreatorFunctionType;

/// Classification result with priority
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub requires_simulation: bool,
    pub requires_buy_sell_test: bool,
}

/// Simulation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulationPriority {
    Critical = 0, // Must simulate immediately
    High = 1,     // Should simulate soon
    Normal = 2,   // Can batch
    Low = 3,      // Optional simulation
}

/// Main transaction router
pub struct TransactionRouter {
    contract_router: ContractCreationRouter,
    creator_router: CreatorTransactionRouter,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl TransactionRouter {
    /// Create a new transaction classifier
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self {
            contract_router: ContractCreationRouter::new(),
            creator_router: CreatorTransactionRouter::new(token_cache.clone()),
            token_cache,
        }
    }

    /// Classify a transaction
    pub async fn classify(&self, tx: &MempoolTransaction) -> ClassificationResult {
        // First check if it's a contract creation
        let to_str = tx
            .to
            .as_ref()
            .map(|t| hex::encode(t))
            .unwrap_or_else(|| "contract_creation".to_string());

        if tx.to.is_none() || to_str == "0x0" || to_str.is_empty() {
            let result = self.classify_contract_creation(tx).await;
            return result;
        }

        // Check if from a known creator
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
            if let Some(target_token) = self.tracked_liquidity_removal_token(tx).await {
                return ClassificationResult {
                    category: TransactionCategory::CreatorTransaction {
                        creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                        target_address: tx
                            .to
                            .as_ref()
                            .map(|t| to_checksum_address(&AlloyAddress::from_slice(t)))
                            .unwrap_or_else(|| "none".to_string()),
                        target_token: Some(target_token),
                        function_type: CreatorFunctionType::LiquidityRemoval,
                    },
                    priority: SimulationPriority::Critical,
                    requires_simulation: true,
                    requires_buy_sell_test: true,
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

        // If not a creator or contract creation, classify as regular transaction
        let result = self.classify_regular_transaction(tx).await;
        result
    }

    async fn tracked_liquidity_removal_token(&self, tx: &MempoolTransaction) -> Option<String> {
        let cache = self.token_cache.as_ref()?;
        for candidate in liquidity_removal_token_candidates(&tx.input) {
            if cache.get_token(&candidate).await.is_some() {
                return Some(candidate);
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
        cache
            .get_token(&target)
            .await
            .map(|token| token.address.clone())
    }

    /// Classify contract creation
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

    /// Classify creator transaction
    async fn classify_creator_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let function_type = self.creator_router.get_function_type(tx);

        // Get the token created by this creator
        let target_token = if let Some(ref cache) = self.token_cache {
            let from_addr = to_checksum_address(&AlloyAddress::from_slice(&tx.from));

            // Get token info for this creator
            if let Some(token_info) = cache.get_token_for_creator(&from_addr).await {
                // Return the token address for context
                Some(token_info.address.clone())
            } else {
                None
            }
        } else {
            None
        };

        // Check if this is just an ETH transfer from a creator
        let is_eth_transfer =
            matches!(&function_type, CreatorFunctionType::Other(s) if s == "eth_transfer");

        // ALL creator transactions get high priority except ETH transfers
        let priority = priority_for_creator_function(&function_type);

        // LP approvals do NOT require simulation or buy/sell
        let is_lp_approval = matches!(&function_type, CreatorFunctionType::LiquidityPoolApproval);
        // ALL creator transactions get buy/sell test except ETH transfers and LP approvals
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
            // Simulate everything except ETH transfers and LP approvals
            requires_simulation: !is_eth_transfer && !is_lp_approval,
            requires_buy_sell_test: requires_buy_sell,
        }
    }

    /// Classify regular transaction
    async fn classify_regular_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let input_data = &tx.input;

        // Check for transfer (0xa9059cbb) or transferFrom (0x23b872dd)
        let is_transfer = input_data.len() >= 4
            && (&input_data[0..4] == &[0xa9, 0x05, 0x9c, 0xbb]
                || &input_data[0..4] == &[0x23, 0xb8, 0x72, 0xdd]);

        // Check for approve (0x095ea7b3)
        let is_approval = input_data.len() >= 4 && &input_data[0..4] == &[0x09, 0x5e, 0xa7, 0xb3];

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

fn should_route_tracked_token_call(function_type: &CreatorFunctionType) -> bool {
    matches!(
        function_type,
        CreatorFunctionType::TradingControl
            | CreatorFunctionType::TaxModification
            | CreatorFunctionType::MaxWalletLimit
            | CreatorFunctionType::OwnershipChange
    )
}

fn priority_for_creator_function(function_type: &CreatorFunctionType) -> SimulationPriority {
    match function_type {
        CreatorFunctionType::TaxModification => SimulationPriority::Critical,
        CreatorFunctionType::TradingControl => SimulationPriority::Critical,
        CreatorFunctionType::OwnershipChange => SimulationPriority::High,
        CreatorFunctionType::LiquidityAddition => SimulationPriority::High,
        CreatorFunctionType::LiquidityRemoval => SimulationPriority::Critical,
        CreatorFunctionType::LiquidityPoolApproval => SimulationPriority::Critical,
        CreatorFunctionType::MaxWalletLimit => SimulationPriority::High,
        CreatorFunctionType::Other(_) => SimulationPriority::High,
    }
}

fn liquidity_removal_token_candidates(input: &[u8]) -> Vec<String> {
    // Uniswap V2 removeLiquidityETH* uses token as param 0. removeLiquidity
    // uses tokenA/tokenB as params 0 and 1, so check both against tracked tokens.
    (0..2)
        .filter_map(|param_idx| calldata_address_param(input, param_idx))
        .collect()
}

fn calldata_address_param(input: &[u8], param_idx: usize) -> Option<String> {
    let start = 4 + param_idx * 32 + 12;
    let end = start + 20;
    if input.len() < end {
        return None;
    }
    Some(format!("0x{}", hex::encode(&input[start..end])))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    use alloy_primitives::U256;
    use serde_json::json;

    use super::{CreatorFunctionType, TransactionCategory, TransactionRouter};
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

    async fn hydrate_token(cache: &TokenTrackingCache, token_address: &str) {
        let creator_address = "0x3333333333333333333333333333333333333333".to_string();
        let pool_address = "0x4444444444444444444444444444444444444444".to_string();
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
            address: pool_address.clone(),
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
        pools.insert(pool_address, pool);
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
}
