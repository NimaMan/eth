//! Block-level token orchestration over processed Rust transactions.

use std::collections::HashMap;

use alloy_primitives::Address;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::erc20::{ERC20Token, ERC20TokenMetadata};
use crate::pools::BasePoolConfig;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenStateUpdateReport {
    pub token_address: String,
    pub discovered_uniswap_v2_pools: Vec<String>,
    pub updated_uniswap_v2_pools: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenStateManager {
    pub tokens: HashMap<String, ERC20Token>,
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

impl TokenStateManager {
    pub fn new(history_limit: usize) -> Self {
        Self {
            tokens: HashMap::new(),
            history_limit,
            known_routers: Vec::new(),
        }
    }

    pub fn with_known_routers(
        history_limit: usize,
        routers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            tokens: HashMap::new(),
            history_limit,
            known_routers: routers
                .into_iter()
                .map(|router| normalize_address_string(router))
                .collect(),
        }
    }

    pub fn add_token(&mut self, metadata: ERC20TokenMetadata) -> Option<ERC20Token> {
        let address = normalize_address_string(metadata.address.clone());
        self.tokens.insert(address, ERC20Token::new(metadata))
    }

    pub fn token(&self, address: impl AsRef<str>) -> Option<&ERC20Token> {
        self.tokens.get(&normalize_address(address))
    }

    pub fn token_mut(&mut self, address: impl AsRef<str>) -> Option<&mut ERC20Token> {
        self.tokens.get_mut(&normalize_address(address))
    }

    pub fn update_from_processed_transaction(
        &mut self,
        tx: &ProcessedTransaction,
    ) -> Result<Vec<TokenStateUpdateReport>> {
        let token_addresses: Vec<_> = self.tokens.keys().cloned().collect();
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = self.tokens.get_mut(&token_address) else {
                continue;
            };

            token.update_token_state_from_processed_transaction(tx)?;
            let discovered = discover_uniswap_v2_pools_for_token(
                token,
                tx,
                self.history_limit,
                &self.known_routers,
            );

            let pool_addresses = token.pool_addresses();
            let mut updated = Vec::new();
            for pool_address in pool_addresses {
                if touches_v2_pool(tx, &pool_address) {
                    token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
                    updated.push(pool_address);
                }
            }

            if !discovered.is_empty() || !updated.is_empty() {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                });
            }
        }

        Ok(reports)
    }
}

fn discover_uniswap_v2_pools_for_token(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    history_limit: usize,
    known_routers: &[String],
) -> Vec<String> {
    let token_address = token.contract_address.clone();
    let mut discovered = Vec::new();

    for event in &tx.uniswap_v2_pair_created_events {
        let token_is_token0 = same_address_str(event.token0, &token_address);
        let token_is_token1 = same_address_str(event.token1, &token_address);
        if !token_is_token0 && !token_is_token1 {
            continue;
        }

        let pool_address = address_string(&event.pair_address);
        if token.uniswap_v2_pool(&pool_address).is_some() {
            continue;
        }

        let denom_address = if token_is_token0 {
            event.token1
        } else {
            event.token0
        };
        token.create_uniswap_v2_pool(
            pool_address.clone(),
            address_string(&denom_address),
            BasePoolConfig {
                token_decimals: token.decimals,
                denom_decimals: None,
                token1_is_denom: Some(token_is_token0),
                history_limit,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
            },
            known_routers,
        );
        discovered.push(pool_address);
    }

    discovered
}

fn touches_v2_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_syncs
        .iter()
        .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
}

fn same_address_str(address: Address, value: &str) -> bool {
    address_string(&address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, U256};
    use tx_processor::tx_processor::data_models::{UniswapV2PairCreatedEvent, UniswapV2SyncEvent};

    fn metadata() -> ERC20TokenMetadata {
        ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "100000000000000000000",
        )
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            1,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn discovers_and_updates_uniswap_v2_pool_for_tracked_token() {
        let mut manager = TokenStateManager::new(100);
        manager.add_token(metadata());
        let mut tx = tx();
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0: address!("1111111111111111111111111111111111111111"),
                token1: address!("2222222222222222222222222222222222222222"),
                log_index: 1,
            });
        tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: address!("3333333333333333333333333333333333333333"),
            reserve0: U256::from(100_000_000_000_000_000_000_u128),
            reserve1: U256::from(2_000_000_000_000_000_000_u128),
            log_index: 2,
        });

        let reports = manager.update_from_processed_transaction(&tx).unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].discovered_uniswap_v2_pools.len(), 1);
        assert_eq!(reports[0].updated_uniswap_v2_pools.len(), 1);
        let token = manager
            .token("0x1111111111111111111111111111111111111111")
            .unwrap();
        let pool = token
            .uniswap_v2_pool("0x3333333333333333333333333333333333333333")
            .unwrap();
        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
    }
}
