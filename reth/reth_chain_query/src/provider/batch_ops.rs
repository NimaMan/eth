use super::RethQueryProvider;
use crate::contracts::erc20::TokenMetadata;
/// Batch operations - Optimized queries for multiple items
///
/// These methods provide efficient batch processing for multiple queries,
/// reducing overhead and improving performance for bulk operations.
use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use futures::future::try_join_all;

impl RethQueryProvider {
    /// Get multiple token balances in parallel
    ///
    /// # Arguments
    /// * `requests` - Vec of (token_address, holder_address) pairs
    /// * `block` - Optional block number for historical queries
    ///
    /// # Returns
    /// Vector of balances in the same order as requests
    pub async fn batch_get_balances_for_token_holder_pairs(
        &self,
        requests: Vec<(Address, Address)>,
        block: Option<u64>,
    ) -> Result<Vec<U256>> {
        let futures = requests
            .into_iter()
            .map(|(token, holder)| self.get_token_balance(token, holder, block));

        try_join_all(futures).await
    }

    /// Get balances for one address across multiple tokens
    ///
    /// # Arguments
    /// * `holder` - The address to check balances for
    /// * `tokens` - List of token addresses
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// HashMap of token_address -> balance (only non-zero balances)
    pub async fn get_multiple_token_balances_for_address(
        &self,
        holder: Address,
        tokens: Vec<Address>,
        block: Option<u64>,
    ) -> Result<std::collections::HashMap<Address, U256>> {
        let futures = tokens
            .iter()
            .map(|token| self.get_token_balance(*token, holder, block));

        let balances = try_join_all(futures).await?;

        let mut result = std::collections::HashMap::new();
        for (token, balance) in tokens.into_iter().zip(balances) {
            if balance > U256::ZERO {
                result.insert(token, balance);
            }
        }

        Ok(result)
    }

    /// Get multiple storage values from the same contract
    ///
    /// # Arguments
    /// * `contract` - The contract address
    /// * `slots` - List of storage slots to read
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// Vector of storage values in the same order as slots
    pub async fn batch_get_storage(
        &self,
        contract: Address,
        slots: Vec<B256>,
        block: Option<u64>,
    ) -> Result<Vec<U256>> {
        let futures = slots
            .into_iter()
            .map(|slot| self.get_storage(contract, slot, block));

        try_join_all(futures).await
    }

    /// Get metadata for multiple tokens
    ///
    /// # Arguments
    /// * `tokens` - List of token addresses
    ///
    /// # Returns
    /// Vector of TokenMetadata in the same order as tokens
    pub async fn batch_get_token_metadata(
        &self,
        tokens: Vec<Address>,
        block: Option<u64>,
    ) -> Result<Vec<Option<TokenMetadata>>> {
        let futures = tokens
            .into_iter()
            .map(|token| self.get_token_metadata(token, block, None, None));

        try_join_all(futures).await
    }

    /// Get ETH and token balances for multiple addresses
    ///
    /// # Arguments
    /// * `addresses` - List of addresses to check
    /// * `tokens` - List of tokens to check for each address
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// HashMap of address -> Portfolio
    pub async fn batch_get_portfolios(
        &self,
        addresses: Vec<Address>,
        tokens: Vec<Address>,
        block: Option<u64>,
    ) -> Result<std::collections::HashMap<Address, super::Portfolio>> {
        let futures = addresses
            .iter()
            .map(|address| self.get_portfolio(*address, tokens.clone(), block));

        let portfolios = try_join_all(futures).await?;

        let mut result = std::collections::HashMap::new();
        for (address, portfolio) in addresses.into_iter().zip(portfolios) {
            result.insert(address, portfolio);
        }

        Ok(result)
    }

    /// Check which addresses are contracts
    ///
    /// # Arguments
    /// * `addresses` - List of addresses to check
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// HashMap of address -> is_contract
    pub async fn batch_check_contracts(
        &self,
        addresses: Vec<Address>,
        block: Option<u64>,
    ) -> Result<std::collections::HashMap<Address, bool>> {
        let futures = addresses
            .iter()
            .map(|address| self.has_code(*address, block));

        let results = try_join_all(futures).await?;

        let mut map = std::collections::HashMap::new();
        for (address, is_contract) in addresses.into_iter().zip(results) {
            map.insert(address, is_contract);
        }

        Ok(map)
    }

    /// Get transaction counts (nonces) for multiple addresses
    ///
    /// # Arguments
    /// * `addresses` - List of addresses
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// HashMap of address -> nonce
    pub async fn get_transaction_counts_for_multiple_addresses(
        &self,
        addresses: Vec<Address>,
        block: Option<u64>,
    ) -> Result<std::collections::HashMap<Address, u64>> {
        let futures = addresses
            .iter()
            .map(|address| self.get_account(*address, block));

        let accounts = try_join_all(futures).await?;

        let mut result = std::collections::HashMap::new();
        for (address, account) in addresses.into_iter().zip(accounts) {
            result.insert(address, account.nonce);
        }

        Ok(result)
    }

    /// Get ETH balances for multiple addresses in parallel
    ///
    /// # Arguments
    /// * `addresses` - List of addresses to check
    /// * `block` - Optional block number
    ///
    /// # Returns
    /// Vector of ETH balances in the same order as addresses
    pub async fn get_eth_balances_for_multiple_addresses(
        &self,
        addresses: Vec<Address>,
        block: Option<u64>,
    ) -> Result<Vec<U256>> {
        let futures = addresses
            .into_iter()
            .map(|address| self.get_eth_balance(address, block));

        try_join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_operations_compile() {
        // This just verifies the batch operations compile correctly
        // Real tests would need a valid database
    }
}
