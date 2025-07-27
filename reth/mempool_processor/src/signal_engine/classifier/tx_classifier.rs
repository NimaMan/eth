/// Main Transaction Classifier
/// 
/// Categorizes transactions based on their characteristics and routes them
/// for appropriate processing with assigned priorities

use alloy_primitives::Address;
use std::sync::Arc;
use tracing::{debug, trace};
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
use crate::common::address::checksum_address;

use super::{
    ContractCreationClassifier,
    CreatorTransactionClassifier, 
    DexClassifier,
};

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
    /// DEX interaction (swap, liquidity, etc)
    DexInteraction {
        dex_type: DexType,
        action: DexAction,
        token_address: Option<String>,
        pool_address: Option<String>,
    },
    /// Regular transaction (transfer, approval, etc)
    Regular {
        is_transfer: bool,
        is_approval: bool,
    },
}

/// Types of functions called by creators
#[derive(Debug, Clone, PartialEq)]
pub enum CreatorFunctionType {
    TaxModification,
    TradingControl,
    OwnershipChange,
    LiquidityManagement,
    MaxWalletLimit,
    Other(String),
}

/// DEX actions
#[derive(Debug, Clone, PartialEq)]
pub enum DexAction {
    AddLiquidity,
    RemoveLiquidity,
    Swap,
    CreatePool,
    Other,
}

/// DEX types
#[derive(Debug, Clone, PartialEq)]
pub enum DexType {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    Balancer,
    Curve,
    Other(String),
}

/// Classification result with priority
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub requires_simulation: bool,
    pub requires_buy_sell_test: bool,
}

/// Simulation priority levels
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum SimulationPriority {
    Critical = 0,  // Must simulate immediately
    High = 1,      // Should simulate soon
    Normal = 2,    // Can batch
    Low = 3,       // Optional simulation
}

/// Main transaction classifier
pub struct TransactionClassifier {
    contract_classifier: ContractCreationClassifier,
    creator_classifier: CreatorTransactionClassifier,
    dex_classifier: DexClassifier,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl TransactionClassifier {
    /// Create a new transaction classifier
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self {
            contract_classifier: ContractCreationClassifier::new(),
            creator_classifier: CreatorTransactionClassifier::new(token_cache.clone()),
            dex_classifier: DexClassifier::new(),
            token_cache,
        }
    }

    /// Classify a transaction
    pub async fn classify(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let start = std::time::Instant::now();
        
        // First check if it's a contract creation
        let to_str = tx.to.as_ref()
            .map(|t| hex::encode(t))
            .unwrap_or_else(|| "contract_creation".to_string());
            
        if tx.to.is_none() || to_str == "0x0" || to_str.is_empty() {
            let result = self.classify_contract_creation(tx).await;
            debug!("Classified contract creation in {:?}", start.elapsed());
            return result;
        }

        // Check if from a known creator
        if let Some(ref cache) = self.token_cache {
            let from_addr = checksum_address(&hex::encode(&tx.from));
            if cache.is_creator(&from_addr).await {
                let result = self.classify_creator_transaction(tx).await;
                debug!("Classified creator transaction in {:?}", start.elapsed());
                return result;
            }
        }

        // Check if it's a DEX interaction
        if let Some(result) = self.classify_dex_interaction(tx).await {
            debug!("Classified DEX interaction in {:?}", start.elapsed());
            return result;
        }

        // Default to regular transaction
        let result = self.classify_regular_transaction(tx).await;
        trace!("Classified regular transaction in {:?}", start.elapsed());
        result
    }

    /// Classify contract creation
    async fn classify_contract_creation(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let (is_token, has_liquidity) = self.contract_classifier.analyze_creation(tx);
        
        ClassificationResult {
            category: TransactionCategory::ContractCreation {
                deployer: format!("0x{}", hex::encode(&tx.from)),
                contract_address: "pending".to_string(), // Will be determined after execution
                is_token,
                has_liquidity_in_calldata: has_liquidity,
            },
            priority: if is_token { SimulationPriority::High } else { SimulationPriority::Low },
            requires_simulation: is_token,
            requires_buy_sell_test: false, // Can't test until deployed
        }
    }

    /// Classify creator transaction
    async fn classify_creator_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let function_type = self.creator_classifier.identify_function(tx);
        
        // Determine if target is a token
        let target_token = if let Some(ref cache) = self.token_cache {
            if let Some(to_bytes) = &tx.to {
                let to_addr = checksum_address(&hex::encode(to_bytes));
                if cache.get_token_info(&to_addr).await.is_some() {
                    Some(to_addr)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let priority = match &function_type {
            CreatorFunctionType::TaxModification => SimulationPriority::Critical,
            CreatorFunctionType::TradingControl => SimulationPriority::Critical,
            CreatorFunctionType::OwnershipChange => SimulationPriority::High,
            CreatorFunctionType::LiquidityManagement => SimulationPriority::High,
            CreatorFunctionType::MaxWalletLimit => SimulationPriority::High,
            CreatorFunctionType::Other(_) => SimulationPriority::Normal,
        };

        let requires_buy_sell = matches!(
            function_type,
            CreatorFunctionType::TaxModification | 
            CreatorFunctionType::TradingControl |
            CreatorFunctionType::MaxWalletLimit
        );

        ClassificationResult {
            category: TransactionCategory::CreatorTransaction {
                creator: format!("0x{}", hex::encode(&tx.from)),
                target_address: tx.to.as_ref()
                    .map(|t| format!("0x{}", hex::encode(t)))
                    .unwrap_or_else(|| "none".to_string()),
                target_token,
                function_type,
            },
            priority,
            requires_simulation: true,
            requires_buy_sell_test: requires_buy_sell,
        }
    }

    /// Classify DEX interaction
    async fn classify_dex_interaction(&self, tx: &MempoolTransaction) -> Option<ClassificationResult> {
        let (dex_type, action) = self.dex_classifier.identify_dex_action(tx)?;
        
        // Extract token/pool from calldata if possible
        let (token_address, pool_address) = self.dex_classifier.extract_addresses(tx);

        let priority = match action {
            DexAction::RemoveLiquidity => SimulationPriority::High,
            DexAction::AddLiquidity => SimulationPriority::Normal,
            DexAction::Swap => SimulationPriority::Low,
            _ => SimulationPriority::Low,
        };

        Some(ClassificationResult {
            category: TransactionCategory::DexInteraction {
                dex_type,
                action,
                token_address,
                pool_address,
            },
            priority,
            requires_simulation: matches!(action, DexAction::RemoveLiquidity | DexAction::AddLiquidity),
            requires_buy_sell_test: false,
        })
    }

    /// Classify regular transaction
    async fn classify_regular_transaction(&self, tx: &MempoolTransaction) -> ClassificationResult {
        let input_data = &tx.input;
        
        // Check for transfer (0xa9059cbb) or transferFrom (0x23b872dd)
        let is_transfer = input_data.len() >= 4 && (
            &input_data[0..4] == &[0xa9, 0x05, 0x9c, 0xbb] ||
            &input_data[0..4] == &[0x23, 0xb8, 0x72, 0xdd]
        );

        // Check for approve (0x095ea7b3)
        let is_approval = input_data.len() >= 4 && 
            &input_data[0..4] == &[0x09, 0x5e, 0xa7, 0xb3];

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