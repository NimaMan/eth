use crate::erc20::ERC20Token;
use crate::pools::BasePoolConfig;
use tx_processor::ProcessedTransaction;

use super::super::known_tokens::known_decimals_for_address;
use super::super::TokenTransactionApplier;
use crate::tracking::{address_string, hash_string, same_address_str};

impl TokenTransactionApplier {
    pub(crate) fn discover_uniswap_v3_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v3_pools {
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pool);
            if token.uniswap_v3_pool(&pool_address).is_some() {
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let pool = token.create_uniswap_v3_pool(
                pool_address.clone(),
                address_string(&denom_address),
                address_string(&event.token0),
                address_string(&event.token1),
                event.fee,
                event.tick_spacing,
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: known_decimals_for_address(denom_address),
                    token1_is_denom: Some(token_is_token0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        discovered
    }
}
