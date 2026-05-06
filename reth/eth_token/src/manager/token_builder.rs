use eyre::{eyre, Result};
use tx_processor::{ProcessedBlock, ProcessedTransaction};

use crate::erc20::{ERC20Token, ERC20TokenMetadata};

use super::{address_string, hash_string, normalize_address, TokenStateManager};

#[derive(Clone, Debug, PartialEq)]
pub struct TokenStateBuilder {
    pub metadata: ERC20TokenMetadata,
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

impl TokenStateBuilder {
    pub fn new(metadata: ERC20TokenMetadata, history_limit: usize) -> Self {
        Self {
            metadata,
            history_limit,
            known_routers: Vec::new(),
        }
    }

    pub fn with_known_routers(
        metadata: ERC20TokenMetadata,
        history_limit: usize,
        routers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            metadata,
            history_limit,
            known_routers: routers
                .into_iter()
                .map(|router| normalize_address(router.into()))
                .collect(),
        }
    }

    pub fn build_from_processed_transactions(
        &self,
        transactions: impl IntoIterator<Item = ProcessedTransaction>,
    ) -> Result<ERC20Token> {
        let mut manager = if self.known_routers.is_empty() {
            TokenStateManager::new(self.history_limit)
        } else {
            TokenStateManager::with_known_routers(
                self.history_limit,
                self.known_routers.iter().cloned(),
            )
        };
        manager.add_token(self.metadata.clone());

        let token_address = normalize_address(&self.metadata.address);
        let mut transactions: Vec<_> = transactions.into_iter().collect();
        transactions.sort_by_key(|tx| (tx.block_number, tx.tx_index));

        for tx in transactions {
            if created_token_addresses(&tx)
                .into_iter()
                .any(|address| normalize_address(address_string(&address)) == token_address)
            {
                if let Some(token) = manager.token_mut(&token_address) {
                    if token.creation_block.is_none() {
                        token.handle_contract_creation(
                            tx.block_number,
                            tx.block_timestamp,
                            hash_string(&tx.hash),
                            address_string(&tx.from_address),
                            tx.nonce,
                        );
                    }
                }
            }
            manager.update_from_processed_transaction(&tx)?;
        }

        manager
            .tokens
            .remove(&token_address)
            .ok_or_else(|| eyre!("token {token_address} missing after build"))
    }

    pub fn build_from_processed_blocks(
        &self,
        blocks: impl IntoIterator<Item = ProcessedBlock>,
    ) -> Result<ERC20Token> {
        let transactions = blocks
            .into_iter()
            .flat_map(|block| {
                block
                    .transactions
                    .into_iter()
                    .map(|transaction| transaction.processed)
            })
            .collect::<Vec<_>>();
        self.build_from_processed_transactions(transactions)
    }
}

fn created_token_addresses(tx: &ProcessedTransaction) -> Vec<alloy_primitives::Address> {
    let mut addresses = Vec::new();
    if let Some(address) = tx.contract_address {
        addresses.push(address);
    }
    addresses.extend(
        tx.contract_creation_events
            .iter()
            .map(|event| event.contract_address),
    );
    addresses
}
