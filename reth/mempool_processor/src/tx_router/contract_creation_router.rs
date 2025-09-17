/// Contract Creation Classifier
///
/// Analyzes contract creation transactions to identify token deployments
use crate::mempool_fetcher::MempoolTransaction;
use tracing::debug;

pub struct ContractCreationRouter {
    // Common token creation patterns
    token_signatures: Vec<Vec<u8>>,
    liquidity_signatures: Vec<Vec<u8>>,
}

impl ContractCreationRouter {
    pub fn new() -> Self {
        Self {
            // Common ERC20 function signatures in bytecode
            token_signatures: vec![
                hex::decode("18160ddd").unwrap(), // totalSupply()
                hex::decode("70a08231").unwrap(), // balanceOf(address)
                hex::decode("dd62ed3e").unwrap(), // allowance(address,address)
                hex::decode("a9059cbb").unwrap(), // transfer(address,uint256)
                hex::decode("23b872dd").unwrap(), // transferFrom(address,address,uint256)
                hex::decode("095ea7b3").unwrap(), // approve(address,uint256)
            ],
            // Liquidity-related signatures
            liquidity_signatures: vec![
                hex::decode("e8e33700").unwrap(), // addLiquidity
                hex::decode("f305d719").unwrap(), // addLiquidityETH
                hex::decode("02751cec").unwrap(), // removeLiquidity
                hex::decode("af2979eb").unwrap(), // removeLiquidityETH
            ],
        }
    }

    /// Analyze contract creation to determine if it's a token
    pub fn analyze_creation(&self, tx: &MempoolTransaction) -> (bool, bool) {
        let bytecode = &tx.input;

        if bytecode.len() < 100 {
            return (false, false);
        }

        // Count token-related signatures
        let mut token_sig_count = 0;
        for sig in &self.token_signatures {
            if contains_bytes(&bytecode, sig) {
                token_sig_count += 1;
            }
        }

        // Check for liquidity functions
        let mut has_liquidity = false;
        for sig in &self.liquidity_signatures {
            if contains_bytes(&bytecode, sig) {
                has_liquidity = true;
                break;
            }
        }

        // Consider it a token if it has at least 4 standard ERC20 functions
        let is_token = token_sig_count >= 4;

        if is_token {
            debug!(
                "Detected token creation with {} ERC20 signatures",
                token_sig_count
            );
        }

        (is_token, has_liquidity)
    }
}

/// Check if haystack contains needle
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }

    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
