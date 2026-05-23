use std::collections::{BTreeSet, HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::erc20::ERC20Token;

use super::retention::{
    LiveTokenRetentionDecision, LiveTokenRetentionPolicy, LiveTokenRetentionReport,
};
use super::TokenRegistry;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TrackedTokenStatus {
    Creation,
    Active,
    #[serde(alias = "InactiveScam")]
    InactiveHiddenMint,
    InactiveOther,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrackedTokenIndexEntry {
    pub token_address: String,
    pub token_status: TrackedTokenStatus,
    pub inserted_sequence: u64,
    pub updated_sequence: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackedTokenIndexUpdate {
    pub token_address: String,
    pub indexed: bool,
    pub removed_by_retention: bool,
    pub evicted_token_address: Option<String>,
    pub retention_decision: Option<LiveTokenRetentionDecision>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackedTokenIndex {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<usize>,
    pub entries: HashMap<String, TrackedTokenIndexEntry>,
    pub pool_to_token: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_retention_policy: Option<LiveTokenRetentionPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_retention_report: Option<LiveTokenRetentionReport>,
    token_pool_addresses: HashMap<String, BTreeSet<String>>,
    lru_order: VecDeque<String>,
    sequence: u64,
    #[serde(default)]
    membership_generation: u64,
}

impl TrackedTokenIndex {
    pub fn new(max_size: usize) -> Self {
        Self::with_max_size(Some(max_size.max(1)))
    }

    pub fn unbounded() -> Self {
        Self::with_max_size(None)
    }

    fn with_max_size(max_size: Option<usize>) -> Self {
        Self {
            max_size,
            entries: HashMap::new(),
            pool_to_token: HashMap::new(),
            live_retention_policy: None,
            last_retention_report: None,
            token_pool_addresses: HashMap::new(),
            lru_order: VecDeque::new(),
            sequence: 0,
            membership_generation: 0,
        }
    }

    pub fn with_live_retention_policy(max_size: usize, policy: LiveTokenRetentionPolicy) -> Self {
        let mut index = Self::new(max_size);
        index.live_retention_policy = Some(policy);
        index
    }

    pub fn from_registry(registry: &TokenRegistry, max_size: usize) -> Self {
        let mut index = Self::new(max_size);
        for token in registry.tokens.values() {
            index.index_token(token, TrackedTokenStatus::Creation);
        }
        index
    }

    pub fn set_live_retention_policy(&mut self, policy: Option<LiveTokenRetentionPolicy>) {
        self.live_retention_policy = policy;
        self.last_retention_report = None;
    }

    pub fn live_retention_policy(&self) -> Option<&LiveTokenRetentionPolicy> {
        self.live_retention_policy.as_ref()
    }

    pub fn last_retention_report(&self) -> Option<&LiveTokenRetentionReport> {
        self.last_retention_report.as_ref()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        if !self.entries.is_empty() || !self.pool_to_token.is_empty() {
            self.membership_generation += 1;
        }
        self.entries.clear();
        self.pool_to_token.clear();
        self.token_pool_addresses.clear();
        self.lru_order.clear();
        self.last_retention_report = None;
        self.sequence = 0;
    }

    pub fn contains_token(&self, token_address: impl AsRef<str>) -> bool {
        self.entries.contains_key(&normalize_address(token_address))
    }

    pub fn tracked_token_addresses(&self) -> Vec<String> {
        self.lru_order.iter().cloned().collect()
    }

    pub fn membership_generation(&self) -> u64 {
        self.membership_generation
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

    pub fn index_token(
        &mut self,
        token: &ERC20Token,
        token_status: TrackedTokenStatus,
    ) -> Option<String> {
        let address = normalize_address(&token.contract_address);
        let was_present = self.entries.contains_key(&address);
        self.sequence += 1;
        let entry = TrackedTokenIndexEntry {
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
        if !was_present {
            self.membership_generation += 1;
        }
        self.touch(&address);
        self.update_pool_mapping(token);
        self.evict_if_needed()
    }

    pub fn index_registry_token(
        &mut self,
        registry: &mut TokenRegistry,
        token_address: impl AsRef<str>,
        token_status: TrackedTokenStatus,
        _current_block: u64,
    ) -> TrackedTokenIndexUpdate {
        let token_address = normalize_address(token_address);

        let Some(token) = registry.tokens.get(&token_address) else {
            return TrackedTokenIndexUpdate {
                token_address,
                indexed: false,
                removed_by_retention: false,
                evicted_token_address: None,
                retention_decision: None,
            };
        };
        let evicted_token_address = self.index_token(token, token_status);

        if let Some(evicted_token_address) = &evicted_token_address {
            registry.tokens.remove(evicted_token_address);
        }

        TrackedTokenIndexUpdate {
            indexed: match &evicted_token_address {
                Some(evicted) => evicted != &token_address,
                None => true,
            },
            token_address,
            removed_by_retention: false,
            evicted_token_address,
            retention_decision: None,
        }
    }

    pub fn apply_live_retention_policy(
        &mut self,
        registry: &mut TokenRegistry,
        current_block: u64,
    ) -> Option<LiveTokenRetentionReport> {
        let policy = self.live_retention_policy.clone()?;
        Some(self.apply_retention_policy(registry, &policy, current_block))
    }

    pub fn apply_retention_policy(
        &mut self,
        registry: &mut TokenRegistry,
        policy: &LiveTokenRetentionPolicy,
        current_block: u64,
    ) -> LiveTokenRetentionReport {
        let mut token_addresses = registry.token_addresses();
        token_addresses.sort();

        let mut token_decisions = Vec::new();
        let mut retained_tokens = 0;
        let mut dropped_tokens = 0;
        let mut dropped_v2_pool_count = 0;

        for token_address in token_addresses {
            let Some(decision) = self.apply_retention_policy_to_token(
                registry,
                policy,
                &token_address,
                current_block,
            ) else {
                continue;
            };

            dropped_v2_pool_count += decision.dropped_v2_pools.len();
            if decision.retain {
                retained_tokens += 1;
                if let Some(token) = registry.token(&token_address) {
                    self.update_pool_mapping(token);
                }
            } else {
                dropped_tokens += 1;
                registry.tokens.remove(&token_address);
                self.remove_token(&token_address);
            }

            token_decisions.push(decision);
        }

        let report = LiveTokenRetentionReport {
            current_block,
            evaluated_tokens: token_decisions.len(),
            retained_tokens,
            dropped_tokens,
            dropped_v2_pool_count,
            token_decisions,
        };
        self.last_retention_report = Some(report.clone());
        report
    }

    pub fn mark_status(
        &mut self,
        token_address: impl AsRef<str>,
        token_status: TrackedTokenStatus,
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
        if existed {
            self.membership_generation += 1;
        }
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
        let Some(max_size) = self.max_size else {
            return None;
        };

        if self.entries.len() <= max_size {
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

    fn apply_retention_policy_to_token(
        &mut self,
        registry: &mut TokenRegistry,
        policy: &LiveTokenRetentionPolicy,
        token_address: &str,
        current_block: u64,
    ) -> Option<LiveTokenRetentionDecision> {
        registry
            .tokens
            .get_mut(token_address)
            .map(|token| policy.apply_to_token(token, current_block))
    }
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::erc20::ERC20TokenMetadata;
    use crate::pools::{BasePoolConfig, UniswapV2Pool};
    use crate::tracking::retention::{ephemeral_terminal_scam_retention_policy, WETH_ADDRESS};

    const TOKEN_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
    const SECOND_TOKEN_ADDRESS: &str = "0x2222222222222222222222222222222222222222";
    const LOW_POOL_ADDRESS: &str = "0x3333333333333333333333333333333333333333";
    const HIGH_POOL_ADDRESS: &str = "0x4444444444444444444444444444444444444444";

    fn metadata(address: &str) -> ERC20TokenMetadata {
        ERC20TokenMetadata::new(address, "Token", "TKN", 18, "1000")
    }

    fn v2_pool(pool_address: &str, token_address: &str, denom_reserve: f64) -> UniswapV2Pool {
        let mut pool = UniswapV2Pool::new(
            pool_address,
            token_address,
            WETH_ADDRESS,
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: 0.01,
            },
            ["0x5555555555555555555555555555555555555555"],
        );
        pool.base
            .update_reserves(1_000.0, denom_reserve, 100, 1_700, "0xSYNC");
        pool
    }

    #[test]
    fn retention_policy_prunes_pools_from_index_and_registry_token() {
        let mut registry = TokenRegistry::new();
        registry.add_token_with_live_mode(metadata(TOKEN_ADDRESS), true);
        let token = registry.token_mut(TOKEN_ADDRESS).unwrap();
        token.add_uniswap_v2_pool(v2_pool(LOW_POOL_ADDRESS, TOKEN_ADDRESS, 0.01));
        token.add_uniswap_v2_pool(v2_pool(HIGH_POOL_ADDRESS, TOKEN_ADDRESS, 0.2));

        let mut index =
            TrackedTokenIndex::with_live_retention_policy(10, LiveTokenRetentionPolicy::default());
        index.index_token(
            registry.token(TOKEN_ADDRESS).unwrap(),
            TrackedTokenStatus::Active,
        );
        assert_eq!(index.token_for_pool(LOW_POOL_ADDRESS), Some(TOKEN_ADDRESS));

        let report = index
            .apply_live_retention_policy(&mut registry, 110)
            .unwrap();

        assert_eq!(report.evaluated_tokens, 1);
        assert_eq!(report.dropped_v2_pool_count, 1);
        let token = registry.token(TOKEN_ADDRESS).unwrap();
        assert!(token.uniswap_v2_pool(LOW_POOL_ADDRESS).is_none());
        assert!(token.uniswap_v2_pool(HIGH_POOL_ADDRESS).is_some());
        assert_eq!(index.token_for_pool(LOW_POOL_ADDRESS), None);
        assert_eq!(index.token_for_pool(HIGH_POOL_ADDRESS), Some(TOKEN_ADDRESS));
        assert!(index.last_retention_report().is_some());
    }

    #[test]
    fn index_registry_token_defers_retention_to_reported_pass() {
        let mut policy = LiveTokenRetentionPolicy {
            drop_tokens_without_retained_pools_after_blocks: Some(10),
            ..LiveTokenRetentionPolicy::default()
        };
        policy.min_other_denom_reserve = 1.0;

        let mut registry = TokenRegistry::new();
        registry.add_token_with_live_mode(metadata(TOKEN_ADDRESS), true);
        let token = registry.token_mut(TOKEN_ADDRESS).unwrap();
        token.add_uniswap_v2_pool(v2_pool(LOW_POOL_ADDRESS, TOKEN_ADDRESS, 0.01));

        let mut index = TrackedTokenIndex::with_live_retention_policy(10, policy);
        let update = index.index_registry_token(
            &mut registry,
            TOKEN_ADDRESS,
            TrackedTokenStatus::Creation,
            110,
        );

        assert!(update.indexed);
        assert!(!update.removed_by_retention);
        assert!(registry.token(TOKEN_ADDRESS).is_some());
        assert!(index.contains_token(TOKEN_ADDRESS));

        let report = index
            .apply_live_retention_policy(&mut registry, 110)
            .unwrap();

        assert_eq!(report.dropped_tokens, 1);
        assert!(registry.token(TOKEN_ADDRESS).is_none());
        assert!(!index.contains_token(TOKEN_ADDRESS));
    }

    #[test]
    fn ephemeral_terminal_scam_policy_removes_scam_token_from_registry() {
        let mut registry = TokenRegistry::new();
        registry.add_token_with_live_mode(metadata(TOKEN_ADDRESS), true);
        let token = registry.token_mut(TOKEN_ADDRESS).unwrap();
        let mut pool = v2_pool(LOW_POOL_ADDRESS, TOKEN_ADDRESS, 0.5);
        pool.base.mark_liquidity_removal(
            "liquidity_removal",
            Some(110),
            Some("0xSCAM".to_string()),
        );
        token.add_uniswap_v2_pool(pool);

        let mut index = TrackedTokenIndex::with_live_retention_policy(
            10,
            ephemeral_terminal_scam_retention_policy(),
        );
        index.index_token(
            registry.token(TOKEN_ADDRESS).unwrap(),
            TrackedTokenStatus::Active,
        );

        let report = index
            .apply_live_retention_policy(&mut registry, 110)
            .unwrap();

        assert_eq!(report.dropped_tokens, 1);
        assert!(registry.token(TOKEN_ADDRESS).is_none());
        assert!(!index.contains_token(TOKEN_ADDRESS));
    }

    #[test]
    fn index_registry_token_eviction_removes_registry_token() {
        let mut registry = TokenRegistry::new();
        registry.add_token(metadata(TOKEN_ADDRESS));
        registry.add_token(metadata(SECOND_TOKEN_ADDRESS));

        let mut index = TrackedTokenIndex::new(1);
        let first = index.index_registry_token(
            &mut registry,
            TOKEN_ADDRESS,
            TrackedTokenStatus::Creation,
            100,
        );
        assert!(first.evicted_token_address.is_none());

        let second = index.index_registry_token(
            &mut registry,
            SECOND_TOKEN_ADDRESS,
            TrackedTokenStatus::Creation,
            101,
        );

        assert_eq!(second.evicted_token_address.as_deref(), Some(TOKEN_ADDRESS));
        assert!(registry.token(TOKEN_ADDRESS).is_none());
        assert!(registry.token(SECOND_TOKEN_ADDRESS).is_some());
        assert!(index.contains_token(SECOND_TOKEN_ADDRESS));
    }

    #[test]
    fn unbounded_index_registry_token_keeps_all_registry_tokens() {
        let mut registry = TokenRegistry::new();
        registry.add_token(metadata(TOKEN_ADDRESS));
        registry.add_token(metadata(SECOND_TOKEN_ADDRESS));

        let mut index = TrackedTokenIndex::unbounded();
        let first = index.index_registry_token(
            &mut registry,
            TOKEN_ADDRESS,
            TrackedTokenStatus::Creation,
            100,
        );
        let second = index.index_registry_token(
            &mut registry,
            SECOND_TOKEN_ADDRESS,
            TrackedTokenStatus::Creation,
            101,
        );

        assert!(first.evicted_token_address.is_none());
        assert!(second.evicted_token_address.is_none());
        assert!(registry.token(TOKEN_ADDRESS).is_some());
        assert!(registry.token(SECOND_TOKEN_ADDRESS).is_some());
        assert!(index.contains_token(TOKEN_ADDRESS));
        assert!(index.contains_token(SECOND_TOKEN_ADDRESS));
    }
}
