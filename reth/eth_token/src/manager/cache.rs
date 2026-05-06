use std::collections::{BTreeSet, HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::erc20::ERC20Token;

use super::TokenRegistry;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TokenCacheStatus {
    Creation,
    Active,
    InactiveScam,
    InactiveOther,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenCacheEntry {
    pub token_address: String,
    pub token_status: TokenCacheStatus,
    pub inserted_sequence: u64,
    pub updated_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenStateCache {
    pub max_size: usize,
    pub entries: HashMap<String, TokenCacheEntry>,
    pub pool_to_token: HashMap<String, String>,
    token_pool_addresses: HashMap<String, BTreeSet<String>>,
    lru_order: VecDeque<String>,
    sequence: u64,
}

impl TokenStateCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size: max_size.max(1),
            entries: HashMap::new(),
            pool_to_token: HashMap::new(),
            token_pool_addresses: HashMap::new(),
            lru_order: VecDeque::new(),
            sequence: 0,
        }
    }

    pub fn from_registry(registry: &TokenRegistry, max_size: usize) -> Self {
        let mut cache = Self::new(max_size);
        for token in registry.tokens.values() {
            cache.insert_token(token, TokenCacheStatus::Creation);
        }
        cache
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.pool_to_token.clear();
        self.token_pool_addresses.clear();
        self.lru_order.clear();
        self.sequence = 0;
    }

    pub fn contains_token(&self, token_address: impl AsRef<str>) -> bool {
        self.entries.contains_key(&normalize_address(token_address))
    }

    pub fn cached_token_addresses(&self) -> Vec<String> {
        self.lru_order.iter().cloned().collect()
    }

    pub fn token_for_pool(&self, pool_address: impl AsRef<str>) -> Option<&str> {
        self.pool_to_token
            .get(&normalize_address(pool_address))
            .map(String::as_str)
    }

    pub fn resolve_token_address(&self, token_or_pool: impl AsRef<str>) -> Option<&str> {
        let address = normalize_address(token_or_pool);
        if self.entries.contains_key(&address) {
            return Some(self.entries.get_key_value(&address)?.0.as_str());
        }
        self.pool_to_token.get(&address).map(String::as_str)
    }

    pub fn insert_token(
        &mut self,
        token: &ERC20Token,
        token_status: TokenCacheStatus,
    ) -> Option<String> {
        let address = normalize_address(&token.contract_address);
        self.sequence += 1;
        let entry = TokenCacheEntry {
            token_address: address.clone(),
            token_status,
            inserted_sequence: self
                .entries
                .get(&address)
                .map(|entry| entry.inserted_sequence)
                .unwrap_or(self.sequence),
            updated_sequence: self.sequence,
        };
        self.entries.insert(address.clone(), entry);
        self.touch(&address);
        self.update_pool_mapping(token);
        self.evict_if_needed()
    }

    pub fn mark_status(
        &mut self,
        token_address: impl AsRef<str>,
        token_status: TokenCacheStatus,
    ) -> bool {
        let address = normalize_address(token_address);
        self.sequence += 1;
        if let Some(entry) = self.entries.get_mut(&address) {
            entry.token_status = token_status;
            entry.updated_sequence = self.sequence;
            self.touch(&address);
            return true;
        }
        false
    }

    pub fn update_pool_mapping(&mut self, token: &ERC20Token) {
        let token_address = normalize_address(&token.contract_address);
        let current: BTreeSet<_> = token.pool_addresses().into_iter().collect();
        let previous = self
            .token_pool_addresses
            .get(&token_address)
            .cloned()
            .unwrap_or_default();

        for pool_address in previous.difference(&current) {
            self.pool_to_token.remove(pool_address);
        }

        for pool_address in &current {
            self.pool_to_token
                .insert(pool_address.clone(), token_address.clone());
        }

        self.token_pool_addresses.insert(token_address, current);
    }

    pub fn remove_token(&mut self, token_address: impl AsRef<str>) -> bool {
        let address = normalize_address(token_address);
        let existed = self.entries.remove(&address).is_some();
        self.lru_order.retain(|candidate| candidate != &address);
        if let Some(pool_addresses) = self.token_pool_addresses.remove(&address) {
            for pool_address in pool_addresses {
                self.pool_to_token.remove(&pool_address);
            }
        }
        existed
    }

    fn touch(&mut self, address: &str) {
        self.lru_order.retain(|candidate| candidate != address);
        self.lru_order.push_back(address.to_string());
    }

    fn evict_if_needed(&mut self) -> Option<String> {
        if self.entries.len() <= self.max_size {
            return None;
        }

        while let Some(candidate) = self.lru_order.pop_front() {
            if self.entries.contains_key(&candidate) {
                self.remove_token(&candidate);
                return Some(candidate);
            }
        }

        None
    }
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}
