/// Main Transaction Router
/// 
/// Routes transactions to appropriate simulation strategies based on their
/// characteristics and assigns processing priorities

use std::sync::Arc;
use crate::mempool_fetcher::MempoolTransaction;
use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::Address as AlloyAddress;
use reth_chain_query::to_checksum_address;

use super::{
    ContractCreationRouter,
    CreatorTransactionRouter,
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
    Critical = 0,  // Must simulate immediately
    High = 1,      // Should simulate soon
    Normal = 2,    // Can batch
    Low = 3,       // Optional simulation
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
        let start = std::time::Instant::now();
        
        // First check if it's a contract creation
        let to_str = tx.to.as_ref()
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
        // If not a creator or contract creation, classify as regular transaction
        let result = self.classify_regular_transaction(tx).await;
        result
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
            priority: if is_token { SimulationPriority::High } else { SimulationPriority::Low },
            requires_simulation: false, // Skip simulation for contract creations
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
        let is_eth_transfer = matches!(&function_type, CreatorFunctionType::Other(s) if s == "eth_transfer");
        
        // ALL creator transactions get high priority except ETH transfers
        let priority = match &function_type {
            CreatorFunctionType::TaxModification => SimulationPriority::Critical,
            CreatorFunctionType::TradingControl => SimulationPriority::Critical,
            CreatorFunctionType::OwnershipChange => SimulationPriority::High,
            CreatorFunctionType::LiquidityAddition => SimulationPriority::High,    // Less critical
            CreatorFunctionType::LiquidityRemoval => SimulationPriority::Critical,  // Potential rug pull!
            CreatorFunctionType::LiquidityPoolApproval => SimulationPriority::Critical,
            CreatorFunctionType::MaxWalletLimit => SimulationPriority::High,
            CreatorFunctionType::Other(_) => SimulationPriority::High, // Changed from Low to High
        };

        // LP approvals do NOT require simulation or buy/sell
        let is_lp_approval = matches!(&function_type, CreatorFunctionType::LiquidityPoolApproval);
        // ALL creator transactions get buy/sell test except ETH transfers and LP approvals
        let requires_buy_sell = !is_eth_transfer && !is_lp_approval;

        ClassificationResult {
            category: TransactionCategory::CreatorTransaction {
                creator: to_checksum_address(&AlloyAddress::from_slice(&tx.from)),
                target_address: tx.to.as_ref()
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
