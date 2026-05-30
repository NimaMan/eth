use crate::erc20::ERC20Token;
use crate::pools::{BasePoolConfig, UniswapV3Pool};
use alloy_primitives::Address;
use reth_chain_query::common_addresses::KnownV3Protocol;
use tx_processor::ProcessedTransaction;

use super::super::known_token_metadata::known_decimals_for_address;
use super::super::ProcessedTokenUpdateRouter;
use crate::tracking::{address_string, hash_string, same_address_str};

impl ProcessedTokenUpdateRouter {
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
            if let Some(pool) = token.uniswap_v3_pool_mut(&pool_address) {
                apply_v3_protocol_metadata(pool, event.factory_address);
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let config = BasePoolConfig {
                token_decimals: token.decimals,
                denom_decimals: known_decimals_for_address(denom_address),
                token1_is_denom: Some(token_is_token0),
                history_limit: self.history_limit,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
            };
            let token0 = address_string(&event.token0);
            let token1 = address_string(&event.token1);
            let denom_address = address_string(&denom_address);
            let pool = if let Some(protocol) = KnownV3Protocol::from_factory(event.factory_address)
            {
                token.create_known_v3_pool(
                    protocol,
                    pool_address.clone(),
                    denom_address,
                    token0,
                    token1,
                    event.fee,
                    event.tick_spacing,
                    config,
                )
            } else {
                token.create_uniswap_v3_pool(
                    pool_address.clone(),
                    denom_address,
                    token0,
                    token1,
                    event.fee,
                    event.tick_spacing,
                    config,
                )
            };
            apply_v3_protocol_metadata(pool, event.factory_address);
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            pool.base.creator_address = Some(address_string(&tx.from_address));
            discovered.push(pool_address);
        }

        discovered
    }
}

fn apply_v3_protocol_metadata(pool: &mut UniswapV3Pool, factory_address: Address) {
    match KnownV3Protocol::from_factory(factory_address) {
        Some(protocol) => {
            pool.base.identity.protocol = protocol.label().to_string();
            pool.factory_address = Some(address_string(&protocol.factory()));
            pool.router_address = Some(address_string(&protocol.router()));
        }
        None if !factory_address.is_zero() => {
            pool.base.identity.protocol = "UNKNOWN-V3".to_string();
            pool.factory_address = Some(address_string(&factory_address));
            pool.router_address = None;
        }
        None => {
            let protocol = KnownV3Protocol::UniswapV3;
            pool.factory_address = Some(address_string(&protocol.factory()));
            pool.router_address = Some(address_string(&protocol.router()));
        }
    }
}
