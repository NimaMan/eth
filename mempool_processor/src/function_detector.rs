// signal_engine/function_detector.rs
//
// Simple function signature detector that categorizes transactions
// based on their function selectors (4-byte signatures)

use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::{address, Address as AlloyAddress};
use reth_chain_query::common_addresses::{POOL_FACTORIES, ROUTERS};
use reth_chain_query::to_checksum_address;
use std::collections::HashMap;
use std::sync::Arc;

/// Types of functions called by creators
#[derive(Debug, Clone, PartialEq)]
pub enum CreatorFunctionType {
    TaxModification,
    TradingControl,
    OwnershipChange,
    LiquidityAddition,     // Adding liquidity to pool
    LiquidityRemoval,      // Removing liquidity from pool
    LiquidityPoolApproval, // LP token approval to router (rug pull setup)
    MaxWalletLimit,
    Other(String),
}

/// Function detection result with category
#[derive(Debug, Clone)]
pub struct FunctionDetectionResult {
    pub function_name: String,
    pub function_type: CreatorFunctionType,
    pub selector: String,
}

/// Function detector that categorizes transactions by their function signatures
pub struct FunctionDetector {
    liquidity_removal: LiquidityRemovalDetector,
    trading_enabled: TradingEnabledDetector,
    swap: SwapDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl FunctionDetector {
    pub fn new() -> Self {
        Self::new_with_cache(None)
    }

    pub fn new_with_cache(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self {
            liquidity_removal: LiquidityRemovalDetector::new(),
            trading_enabled: TradingEnabledDetector::new(),
            swap: SwapDetector::new(),
            token_cache,
        }
    }

    /// Check if transaction is a liquidity removal (simplified)
    pub fn is_liquidity_removal(&self, input_data: &[u8]) -> Option<&'static str> {
        if input_data.len() >= 4 {
            self.liquidity_removal.detect(&input_data[0..4])
        } else {
            None
        }
    }

    /// Detect function for a transaction and return the function name if interesting
    pub fn detect_function(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
    ) -> Option<String> {
        if tx.input.len() < 4 {
            return None;
        }

        let selector = &tx.input[0..4];

        // Check liquidity removal
        if let Some(function_name) = self.liquidity_removal.detect(selector) {
            return Some(function_name.to_string());
        }

        // Check trading enabled
        if let Some(function_name) = self.trading_enabled.detect(selector) {
            return Some(function_name.to_string());
        }

        // Check swaps
        if let Some(function_name) = self.swap.detect(selector) {
            return Some(function_name.to_string());
        }

        None
    }

    /// Process batch of transactions and return with function information and categories
    pub fn detect_batch(
        &self,
        mut transactions: Vec<crate::mempool_fetcher::MempoolTransaction>,
    ) -> Vec<crate::mempool_fetcher::MempoolTransaction> {
        for tx in transactions.iter_mut() {
            let mut functions = Vec::new();

            // Skip if no input data
            if tx.input.len() < 4 {
                tx.functions = functions;
                continue;
            }

            let selector = &tx.input[0..4];
            let selector_hex = hex::encode(selector);

            // Detect function and categorize it
            let detection_result = self.detect_and_categorize(tx, selector, &selector_hex);

            if let Some(result) = detection_result {
                functions.push(result.function_name.clone());
                // Store the function type for router to use
                tx.function_category = Some(result.function_type);
            }

            tx.functions = functions;
        }

        transactions
    }

    /// Detect and categorize a function call
    fn detect_and_categorize(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
        selector: &[u8],
        selector_hex: &str,
    ) -> Option<FunctionDetectionResult> {
        // Check if this is a simple ETH transfer (no input data or empty input)
        if tx.input.is_empty() || tx.input.len() < 4 {
            return Some(FunctionDetectionResult {
                function_name: "eth_transfer".to_string(),
                function_type: CreatorFunctionType::Other("eth_transfer".to_string()),
                selector: selector_hex.to_string(),
            });
        }

        // Check for approve function first - needs special handling for LP tokens
        if selector == &hex_to_bytes("095ea7b3") {
            let approve_type = self.classify_approve(tx);
            return Some(FunctionDetectionResult {
                function_name: "approve".to_string(),
                function_type: approve_type,
                selector: selector_hex.to_string(),
            });
        }

        if selector == &hex_to_bytes("ac9650d8") && is_uniswap_v3_position_manager(tx) {
            return Some(FunctionDetectionResult {
                function_name: "multicall".to_string(),
                function_type: CreatorFunctionType::LiquidityRemoval,
                selector: selector_hex.to_string(),
            });
        }

        // Check liquidity removal functions
        if let Some(function_name) = self.liquidity_removal.detect(selector) {
            return Some(FunctionDetectionResult {
                function_name: function_name.to_string(),
                function_type: CreatorFunctionType::LiquidityRemoval,
                selector: selector_hex.to_string(),
            });
        }

        // Check trading enabled functions
        if let Some(function_name) = self.trading_enabled.detect(selector) {
            return Some(FunctionDetectionResult {
                function_name: function_name.to_string(),
                function_type: CreatorFunctionType::TradingControl,
                selector: selector_hex.to_string(),
            });
        }

        // Check swap functions
        if let Some(function_name) = self.swap.detect(selector) {
            return Some(FunctionDetectionResult {
                function_name: function_name.to_string(),
                function_type: CreatorFunctionType::Other(function_name.to_string()),
                selector: selector_hex.to_string(),
            });
        }

        // Check for other known functions by name mapping
        if let Some(function_type) = self.map_function_name_to_type(selector_hex) {
            return Some(FunctionDetectionResult {
                function_name: format!("unknown_{}", selector_hex),
                function_type,
                selector: selector_hex.to_string(),
            });
        }

        // Unknown function
        Some(FunctionDetectionResult {
            function_name: format!("unknown_{}", selector_hex),
            function_type: CreatorFunctionType::Other(selector_hex.to_string()),
            selector: selector_hex.to_string(),
        })
    }

    /// Map function selectors to types based on known signatures
    fn map_function_name_to_type(&self, selector_hex: &str) -> Option<CreatorFunctionType> {
        match selector_hex {
            // Tax modification selectors belong here only when they are known
            // mutating setters. View/transfer selectors must not be promoted
            // into creator-control.

            // Trading control functions
            "8a8c523c" | "c9567bf9" => Some(CreatorFunctionType::TradingControl),

            // Ownership functions
            "f2fde38b" | "715018a6" => Some(CreatorFunctionType::OwnershipChange),

            // Liquidity additions
            "e8e33700" | "f305d719" => Some(CreatorFunctionType::LiquidityAddition),

            _ => None,
        }
    }

    /// Classify approve() calls - determine if it's LP token approval for rug pull
    fn classify_approve(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
    ) -> CreatorFunctionType {
        // Check if we have enough data for approve(address,uint256)
        if tx.input.len() < 68 {
            return CreatorFunctionType::Other("approve".to_string());
        }

        let Some(spender) = approval_spender(&tx.input) else {
            return CreatorFunctionType::Other("approve".to_string());
        };
        let spender_is_known = is_known_lp_approval_spender(&spender);
        let spender_addr = to_checksum_address(&spender);

        // Check if the approve is being called on an LP token contract
        if let Some(to_bytes) = &tx.to {
            let to_addr = to_checksum_address(&AlloyAddress::from_slice(to_bytes));

            // Check if the 'to' address is a pool (LP token)
            if let Some(ref cache) = self.token_cache {
                let is_pool = futures::executor::block_on(cache.is_pool(&to_addr));
                let spender_is_tracked_pool =
                    futures::executor::block_on(cache.is_pool(&spender_addr));

                if is_pool && (spender_is_known || spender_is_tracked_pool) {
                    return CreatorFunctionType::LiquidityPoolApproval;
                }
            }
        }

        // Regular approval (not LP token or not to router)
        CreatorFunctionType::Other("approve".to_string())
    }
}

fn approval_spender(input: &[u8]) -> Option<AlloyAddress> {
    if input.len() < 68 || input.get(0..4)? != [0x09, 0x5e, 0xa7, 0xb3].as_slice() {
        return None;
    }
    Some(AlloyAddress::from_slice(&input[16..36]))
}

fn is_known_lp_approval_spender(spender: &AlloyAddress) -> bool {
    *spender == address!("000000000022D473030F116dDEE9F6B43aC78BA3")
        || *spender == POOL_FACTORIES["balancer_vault"]
        || ROUTERS.values().any(|router| router == spender)
}

fn is_uniswap_v3_position_manager(tx: &crate::mempool_fetcher::MempoolTransaction) -> bool {
    tx.to
        .as_ref()
        .map(|to| {
            AlloyAddress::from_slice(to) == address!("C36442b4a4522E871399CD717aBDD847Ab11FE88")
        })
        .unwrap_or(false)
}

/// Detector for liquidity removal functions
struct LiquidityRemovalDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl LiquidityRemovalDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();

        // Uniswap V2 Router
        signatures.insert(hex_to_bytes("02751cec"), "removeLiquidityETH");
        signatures.insert(hex_to_bytes("baa2abde"), "removeLiquidity");
        signatures.insert(
            hex_to_bytes("af2979eb"),
            "removeLiquidityETHSupportingFeeOnTransferTokens",
        );
        signatures.insert(hex_to_bytes("5b0d5984"), "removeLiquidityETHWithPermit");
        signatures.insert(
            hex_to_bytes("ded9382a"),
            "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens",
        );

        // Uniswap V3 Position Manager
        signatures.insert(hex_to_bytes("0c49ccbe"), "decreaseLiquidity");

        // Uniswap V4 Position/Pool Manager
        signatures.insert(hex_to_bytes("dd46508f"), "modifyLiquidities");
        signatures.insert(hex_to_bytes("a355de88"), "modifyLiquiditiesWithoutUnlock");
        signatures.insert(hex_to_bytes("0d4f319d"), "modifyLiquidity");

        // Balancer
        signatures.insert(hex_to_bytes("8bdb3913"), "exitPool");

        // Curve
        signatures.insert(hex_to_bytes("1a4d01d2"), "remove_liquidity");
        signatures.insert(hex_to_bytes("517a55a3"), "remove_liquidity_one_coin");
        signatures.insert(hex_to_bytes("5b36389c"), "remove_liquidity_imbalance");

        // SushiSwap (same as Uniswap V2)
        // PancakeSwap (same as Uniswap V2)

        Self { signatures }
    }

    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}

/// Helper function to convert hex string to 4-byte array at compile time
const fn hex_to_bytes(hex: &'static str) -> [u8; 4] {
    let bytes = hex.as_bytes();
    let mut result = [0u8; 4];
    let mut i = 0;
    while i < 4 {
        let high = hex_char_to_byte(bytes[i * 2]);
        let low = hex_char_to_byte(bytes[i * 2 + 1]);
        result[i] = (high << 4) | low;
        i += 1;
    }
    result
}

const fn hex_char_to_byte(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

/// Detector for trading enabled functions
struct TradingEnabledDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl TradingEnabledDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();

        // Confirmed trading enabled functions
        signatures.insert(hex_to_bytes("8a8c523c"), "enableTrading"); // ✓ Confirmed
        signatures.insert(hex_to_bytes("c9567bf9"), "openTrading"); // ✓ Confirmed
        signatures.insert(hex_to_bytes("8ee88c53"), "enableTrading");
        signatures.insert(hex_to_bytes("fb201b1d"), "startTrading");

        // Trading disable/pause functions
        signatures.insert(hex_to_bytes("1c8387fa"), "pauseTrading");
        signatures.insert(hex_to_bytes("0fb5a6ec"), "disableTrading");

        Self { signatures }
    }

    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}

/// Detector for swap functions
struct SwapDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl SwapDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();

        // Uniswap V2/V3 and forks
        signatures.insert(hex_to_bytes("38ed1739"), "swapExactTokensForTokens");
        signatures.insert(hex_to_bytes("8803dbee"), "swapTokensForExactTokens");
        signatures.insert(hex_to_bytes("7ff36ab5"), "swapExactETHForTokens");
        signatures.insert(hex_to_bytes("4a25d94a"), "swapTokensForExactETH");
        signatures.insert(hex_to_bytes("18cbafe5"), "swapExactTokensForETH");
        signatures.insert(hex_to_bytes("fb3bdb41"), "swapETHForExactTokens");
        signatures.insert(
            hex_to_bytes("791ac947"),
            "swapExactTokensForETHSupportingFeeOnTransferTokens",
        );
        signatures.insert(
            hex_to_bytes("b6f9de95"),
            "swapExactETHForTokensSupportingFeeOnTransferTokens",
        );

        // Uniswap V3
        signatures.insert(hex_to_bytes("414bf389"), "exactInputSingle");
        signatures.insert(hex_to_bytes("db3e2198"), "exactOutputSingle");
        signatures.insert(hex_to_bytes("c04b8d59"), "exactInput");
        signatures.insert(hex_to_bytes("f28c0498"), "exactOutput");

        // 1inch
        signatures.insert(hex_to_bytes("2e95b6c8"), "swap");
        signatures.insert(hex_to_bytes("7c025200"), "swap_1inch_v2");
        signatures.insert(hex_to_bytes("e449022e"), "uniswapV3Swap");

        // 0x Protocol
        signatures.insert(hex_to_bytes("d9627aa4"), "sellToUniswap");
        signatures.insert(hex_to_bytes("3598d8ab"), "sellToLiquidityProvider");

        // Curve
        signatures.insert(hex_to_bytes("3df02124"), "exchange");
        signatures.insert(hex_to_bytes("5b41b908"), "exchange_underlying");

        // Balancer
        signatures.insert(hex_to_bytes("52bbbe29"), "swap_balancer");
        signatures.insert(hex_to_bytes("945bcec9"), "batchSwap");

        Self { signatures }
    }

    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CreatorFunctionType, FunctionDetector};
    use crate::mempool_fetcher::MempoolTransaction;
    use alloy_primitives::U256;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn erc20_transfer_is_not_wallet_limit_control() {
        let detector = FunctionDetector::new();
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x5555555555555555555555555555555555555555")),
            input: erc20_transfer_calldata("0x3333333333333333333333333333333333333333"),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        };

        let detected = detector.detect_batch(vec![tx]);
        assert!(matches!(
            detected[0].function_category,
            Some(CreatorFunctionType::Other(_))
        ));
        assert!(!matches!(
            detected[0].function_category,
            Some(CreatorFunctionType::MaxWalletLimit)
        ));
    }

    #[test]
    fn erc20_balance_of_is_not_tax_modification() {
        let detector = FunctionDetector::new();
        let tx = MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address_bytes("0x2222222222222222222222222222222222222222"),
            to: Some(address_bytes("0x5555555555555555555555555555555555555555")),
            input: erc20_balance_of_calldata("0x3333333333333333333333333333333333333333"),
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        };

        let detected = detector.detect_batch(vec![tx]);
        assert!(matches!(
            detected[0].function_category,
            Some(CreatorFunctionType::Other(_))
        ));
        assert!(!matches!(
            detected[0].function_category,
            Some(CreatorFunctionType::TaxModification)
        ));
    }

    fn erc20_transfer_calldata(to: &str) -> Vec<u8> {
        let mut input = hex::decode("a9059cbb").unwrap();
        let address = address_bytes(to);
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(&address);
        input.extend_from_slice(&[0u8; 31]);
        input.push(1);
        input
    }

    fn erc20_balance_of_calldata(account: &str) -> Vec<u8> {
        let mut input = hex::decode("70a08231").unwrap();
        let address = address_bytes(account);
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(&address);
        input
    }

    fn address_bytes(address: &str) -> Vec<u8> {
        hex::decode(address.trim_start_matches("0x")).unwrap()
    }
}
