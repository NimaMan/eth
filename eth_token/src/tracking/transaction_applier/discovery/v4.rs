use crate::erc20::ERC20Token;
use crate::pools::uniswap::{display_denom_for_v4_currency, v4_event_display_key};
use crate::pools::{BasePoolConfig, UniswapV4Pool};
use tx_processor::ProcessedTransaction;

use super::super::known_tokens::known_decimals_for_address_or_native;
use super::super::TokenTransactionApplier;
use crate::tracking::{hash_string, same_address_str};

impl TokenTransactionApplier {
    pub(crate) fn discover_uniswap_v4_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v4_initializes {
            let token_is_currency0 = same_address_str(event.currency0, &token_address);
            let token_is_currency1 = same_address_str(event.currency1, &token_address);
            if !token_is_currency0 && !token_is_currency1 {
                continue;
            }

            let pool_key = v4_event_display_key(event.pool_manager_address, event.event_id);
            if token.uniswap_v4_pool(&pool_key).is_some() {
                continue;
            }

            let denom_currency = if token_is_currency0 {
                event.currency1
            } else {
                event.currency0
            };
            let mut pool = UniswapV4Pool::from_initialize_event(
                event,
                token_address.clone(),
                display_denom_for_v4_currency(denom_currency),
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: known_decimals_for_address_or_native(denom_currency),
                    token1_is_denom: Some(token_is_currency0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            token.add_uniswap_v4_pool(pool);
            discovered.push(pool_key);
        }

        discovered
    }
}
