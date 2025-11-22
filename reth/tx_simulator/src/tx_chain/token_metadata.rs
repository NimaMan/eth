use crate::tx_chain::unsigned::UnsignedTxChainSimulation;
use crate::{RethTxSimulator, UnsignedTransaction};
use alloy_primitives::{Address, Bytes};
use eyre::Result;
use std::sync::Arc;

/// Simulator specifically designed for extracting token metadata (ERC20/ERC721)
/// in a stateful context that may include pending transactions (e.g. deployments).
pub struct TokenMetadataSimulator {
    simulator: Arc<RethTxSimulator>,
}

impl TokenMetadataSimulator {
    pub fn new(simulator: Arc<RethTxSimulator>) -> Self {
        Self { simulator }
    }

    /// Fetch metadata for a token address, optionally replaying a sequence of
    /// pending transactions to establish the correct state (e.g. for newly deployed tokens).
    ///
    /// Flow:
    /// 1. Initialize simulation chain at `block_number`.
    /// 2. Replay `pending_transactions` to update state (balances, code).
    /// 3. Check if `token_address` has code.
    /// 4. Execute view calls (name, symbol, decimals, totalSupply) against the modified state.
    pub async fn fetch_metadata(
        &self,
        token_address: Address,
        block_number: u64,
        gas_block_number: Option<u64>,
        pending_transactions: Vec<UnsignedTransaction>,
    ) -> Result<Option<TokenMetadataResult>> {
        // 1. Initialize Chain
        let mut chain = self
            .simulator
            .start_simulation_chain_with_gas_block(Some(block_number), gas_block_number)
            .await?;

        // 2. Hydrate Context (Pending State)
        for tx in pending_transactions {
            // We step through transactions. If a tx fails, we log it but continue,
            // as subsequent txs might not depend on it, or we want to see the final state anyway.
            // For a funding tx -> deployment flow, if funding fails, deployment fails.
            // We primarily care about the final state for the view calls.
            let _ = chain.step(tx).await?;
        }

        // 3. Code Verification
        // If the contract doesn't exist in the final state, no point querying.
        if !chain.account_has_code(token_address)? {
            return Ok(None);
        }

        // 4. Execute View Calls
        // We define helper for view calls to keep it clean
        let name = self.query_string(&mut chain, token_address, "name()").await;
        let symbol = self
            .query_string(&mut chain, token_address, "symbol()")
            .await;
        let decimals = self.query_u8(&mut chain, token_address, "decimals()").await;
        let total_supply = self
            .query_u256(&mut chain, token_address, "totalSupply()")
            .await;

        // If we got nothing, it's probably not a token
        if name.is_none() && symbol.is_none() && decimals.is_none() && total_supply.is_none() {
            return Ok(None);
        }

        Ok(Some(TokenMetadataResult {
            name,
            symbol,
            decimals,
            total_supply: total_supply.map(|v| v.to_string()),
        }))
    }

    async fn query_string(
        &self,
        chain: &mut UnsignedTxChainSimulation,
        contract: Address,
        signature: &str,
    ) -> Option<String> {
        let data = self.encode_selector(signature);
        let res = chain.simulate_view_call(contract, data).ok()?;
        if res.success {
            crate::contract_method_simulator::decode_string_from_contract_output(&res.output).into()
        } else {
            None
        }
    }

    async fn query_u8(
        &self,
        chain: &mut UnsignedTxChainSimulation,
        contract: Address,
        signature: &str,
    ) -> Option<u8> {
        let data = self.encode_selector(signature);
        let res = chain.simulate_view_call(contract, data).ok()?;
        if res.success && res.output.len() >= 32 {
            Some(
                crate::contract_method_simulator::decode_uint256_from_contract_output(&res.output)
                    .to::<u8>(),
            )
        } else {
            None
        }
    }

    async fn query_u256(
        &self,
        chain: &mut UnsignedTxChainSimulation,
        contract: Address,
        signature: &str,
    ) -> Option<alloy_primitives::U256> {
        let data = self.encode_selector(signature);
        let res = chain.simulate_view_call(contract, data).ok()?;
        if res.success {
            Some(crate::contract_method_simulator::decode_uint256_from_contract_output(&res.output))
        } else {
            None
        }
    }

    fn encode_selector(&self, signature: &str) -> Bytes {
        use alloy_primitives::keccak256;
        Bytes::from(keccak256(signature)[..4].to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct TokenMetadataResult {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<String>,
}
