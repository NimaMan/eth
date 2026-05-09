use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::erc20::{ERC20Token, ERC20TokenMetadata};

use super::{normalize_address, normalize_address_string};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenRegistry {
    pub tokens: HashMap<String, ERC20Token>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }

    pub fn add_token(&mut self, metadata: ERC20TokenMetadata) -> Option<ERC20Token> {
        self.add_token_with_live_mode(metadata, false)
    }

    pub fn add_token_with_live_mode(
        &mut self,
        metadata: ERC20TokenMetadata,
        is_live_mode: bool,
    ) -> Option<ERC20Token> {
        let address = normalize_address_string(metadata.address.clone());
        self.tokens
            .insert(address, ERC20Token::with_live_mode(metadata, is_live_mode))
    }

    pub fn set_live_mode(&mut self, is_live_mode: bool) {
        for token in self.tokens.values_mut() {
            token.set_live_mode(is_live_mode);
        }
    }

    pub fn token(&self, address: impl AsRef<str>) -> Option<&ERC20Token> {
        self.tokens.get(&normalize_address(address))
    }

    pub fn token_mut(&mut self, address: impl AsRef<str>) -> Option<&mut ERC20Token> {
        self.tokens.get_mut(&normalize_address(address))
    }

    pub fn token_addresses(&self) -> Vec<String> {
        self.tokens.keys().cloned().collect()
    }
}
