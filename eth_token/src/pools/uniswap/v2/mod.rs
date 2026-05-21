use std::collections::{HashMap, HashSet};

use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::utils::{append_with_history_limit, parse_raw_f64};

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};

pub const UNISWAP_V2_PROTOCOL: &str = "UNISWAP-V2";
pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2TxContext {
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_hash: String,
    pub from_address: Option<String>,
}

impl UniswapV2TxContext {
    pub fn new(block_number: u64, block_timestamp: u64, tx_hash: impl Into<String>) -> Self {
        Self {
            block_number,
            block_timestamp,
            tx_hash: tx_hash.into(),
            from_address: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2SyncEvent {
    pub pair_address: String,
    pub reserve0: String,
    pub reserve1: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2SwapEvent {
    pub pair_address: String,
    pub sender: Option<String>,
    pub to: Option<String>,
    pub amount0_in: String,
    pub amount1_in: String,
    pub amount0_out: String,
    pub amount1_out: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2MintEvent {
    pub pair_address: String,
    pub to: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2BurnEvent {
    pub pair_address: String,
    pub sender: Option<String>,
    #[serde(default)]
    pub amount0: String,
    #[serde(default)]
    pub amount1: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2TransactionEvents {
    pub syncs: Vec<UniswapV2SyncEvent>,
    pub swaps: Vec<UniswapV2SwapEvent>,
    pub mints: Vec<UniswapV2MintEvent>,
    pub burns: Vec<UniswapV2BurnEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LPTransferEvent {
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub log_index: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LPApprovalEvent {
    pub owner: String,
    pub spender: String,
    pub amount: String,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub block_timestamp: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalInfo {
    pub amount: f64,
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LPHolderInfo {
    pub balance: f64,
    pub approvals: HashMap<String, ApprovalInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LPHolderSnapshot {
    pub address: String,
    pub balance: f64,
    pub share: f64,
    pub approvals: HashMap<String, LPApprovalSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LPApprovalSnapshot {
    pub amount: f64,
    pub tx_hash: String,
    pub block_number: u64,
    pub is_router: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LPTokenTracker {
    pub lp_decimals: u8,
    pub known_routers: HashSet<String>,
    pub history_limit: usize,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub holders: HashMap<String, LPHolderInfo>,
    pub total_supply: f64,
    pub transfers: Vec<Value>,
    pub mint_events: Vec<Value>,
    pub burn_events: Vec<Value>,
    pub approval_events: Vec<Value>,
}

impl LPTokenTracker {
    pub fn new(
        lp_decimals: u8,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
        history_limit: usize,
    ) -> Self {
        Self {
            lp_decimals,
            known_routers: known_routers.into_iter().map(normalize_address).collect(),
            history_limit,
            token_address: None,
            pool_address: None,
            holders: HashMap::new(),
            total_supply: 0.0,
            transfers: Vec::new(),
            mint_events: Vec::new(),
            burn_events: Vec::new(),
            approval_events: Vec::new(),
        }
    }

    pub fn record_transfer(
        &mut self,
        from_address: impl AsRef<str>,
        to_address: impl AsRef<str>,
        amount: f64,
        block_number: u64,
        tx_hash: impl Into<String>,
        log_index: Option<u64>,
    ) {
        let from_address = normalize_address(from_address);
        let to_address = normalize_address(to_address);
        let tx_hash = tx_hash.into();
        let event = json!({
            "block_number": block_number,
            "tx_hash": tx_hash,
            "from_address": from_address,
            "to_address": to_address,
            "amount": amount,
            "log_index": log_index,
        });
        append_with_history_limit(&mut self.transfers, event, self.history_limit);

        if from_address != ZERO_ADDRESS {
            let remove_holder = {
                let holder = self.holder_mut(&from_address);
                holder.balance -= amount;
                holder.balance.abs() < 1e-18 && holder.approvals.is_empty()
            };
            if remove_holder {
                self.holders.remove(&from_address);
            }
        } else {
            self.total_supply += amount;
            append_with_history_limit(
                &mut self.mint_events,
                json!({
                    "block_number": block_number,
                    "tx_hash": tx_hash,
                    "to_address": to_address,
                    "amount": amount,
                    "log_index": log_index,
                }),
                self.history_limit,
            );
        }

        if to_address != ZERO_ADDRESS {
            self.holder_mut(&to_address).balance += amount;
        } else {
            self.total_supply = (self.total_supply - amount).max(0.0);
            append_with_history_limit(
                &mut self.burn_events,
                json!({
                    "block_number": block_number,
                    "tx_hash": tx_hash,
                    "from_address": from_address,
                    "amount": amount,
                    "log_index": log_index,
                }),
                self.history_limit,
            );
        }
    }

    pub fn record_transfer_event(&mut self, transfer: &LPTransferEvent) -> Result<()> {
        let amount = parse_raw_f64(&transfer.amount)? / decimal_scale(self.lp_decimals);
        self.record_transfer(
            &transfer.from_address,
            &transfer.to_address,
            amount,
            transfer.block_number.unwrap_or_default(),
            transfer.tx_hash.clone().unwrap_or_default(),
            transfer.log_index,
        );
        Ok(())
    }

    pub fn record_approval(
        &mut self,
        owner: impl AsRef<str>,
        spender: impl AsRef<str>,
        amount: f64,
        block_number: u64,
        tx_hash: impl Into<String>,
        timestamp: Option<u64>,
    ) -> ApprovalInfo {
        let owner = normalize_address(owner);
        let spender = normalize_address(spender);
        let tx_hash = tx_hash.into();
        let approval = ApprovalInfo {
            amount,
            tx_hash: tx_hash.clone(),
            block_number,
            timestamp,
        };
        self.holder_mut(&owner)
            .approvals
            .insert(spender.clone(), approval.clone());

        append_with_history_limit(
            &mut self.approval_events,
            json!({
                "block_number": block_number,
                "tx_hash": tx_hash,
                "owner": owner,
                "spender": spender,
                "amount": amount,
                "is_router": self.known_routers.contains(&spender),
                "timestamp": timestamp,
            }),
            self.history_limit,
        );

        approval
    }

    pub fn record_approval_event(&mut self, approval: &LPApprovalEvent) -> Result<ApprovalInfo> {
        let amount = parse_raw_f64(&approval.amount)? / decimal_scale(self.lp_decimals);
        Ok(self.record_approval(
            &approval.owner,
            &approval.spender,
            amount,
            approval.block_number.unwrap_or_default(),
            approval.tx_hash.clone().unwrap_or_default(),
            approval.block_timestamp,
        ))
    }

    pub fn balances(&self) -> HashMap<String, f64> {
        self.holders
            .iter()
            .map(|(address, holder)| (address.clone(), holder.balance))
            .collect()
    }

    pub fn share(&self, address: impl AsRef<str>) -> f64 {
        if self.total_supply == 0.0 {
            return 0.0;
        }
        self.holders
            .get(&normalize_address(address))
            .map(|holder| (holder.balance / self.total_supply) * 100.0)
            .unwrap_or(0.0)
    }

    pub fn holder_snapshots(&self) -> Vec<LPHolderSnapshot> {
        let mut snapshots: Vec<_> = self
            .holders
            .iter()
            .filter(|(_, holder)| holder.balance > 0.0 || !holder.approvals.is_empty())
            .map(|(address, holder)| {
                let approvals = holder
                    .approvals
                    .iter()
                    .map(|(spender, approval)| {
                        (
                            spender.clone(),
                            LPApprovalSnapshot {
                                amount: approval.amount,
                                tx_hash: approval.tx_hash.clone(),
                                block_number: approval.block_number,
                                is_router: self.known_routers.contains(spender),
                            },
                        )
                    })
                    .collect();
                LPHolderSnapshot {
                    address: address.clone(),
                    balance: holder.balance,
                    share: self.share(address),
                    approvals,
                }
            })
            .collect();

        snapshots.sort_by(|left, right| {
            right
                .balance
                .partial_cmp(&left.balance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        snapshots
    }

    pub fn total_approved_to_routers(&self) -> f64 {
        self.holders
            .values()
            .flat_map(|holder| {
                holder
                    .approvals
                    .iter()
                    .filter(|(spender, _)| self.known_routers.contains(*spender))
                    .map(|(_, approval)| approval.amount.min(holder.balance))
            })
            .sum()
    }

    pub fn approved_percentage(&self) -> f64 {
        if self.total_supply == 0.0 {
            0.0
        } else {
            (self.total_approved_to_routers() / self.total_supply) * 100.0
        }
    }

    pub fn last_approval_block(&self) -> Option<u64> {
        self.approval_events
            .last()
            .and_then(|event| event.get("block_number"))
            .and_then(Value::as_u64)
    }

    pub fn last_approval_event(&self) -> Option<Value> {
        self.approval_events.last().cloned()
    }

    pub fn holders_with_approvals(&self) -> Vec<String> {
        self.holders
            .iter()
            .filter(|(_, holder)| !holder.approvals.is_empty())
            .map(|(address, _)| address.clone())
            .collect()
    }

    fn holder_mut(&mut self, address: &str) -> &mut LPHolderInfo {
        self.holders.entry(address.to_string()).or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2Pool {
    pub base: BasePool,
    pub lp_tracker: LPTokenTracker,
}

impl UniswapV2Pool {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        Self::new_with_protocol(
            pool_address,
            token_address,
            denom_address,
            UNISWAP_V2_PROTOCOL,
            config,
            known_routers,
        )
    }

    pub fn new_with_protocol(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        protocol: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        config.token1_is_denom = Some(config.token1_is_denom.unwrap_or(true));
        if config.history_limit == 100 {
            config.history_limit = 1000;
        }

        let identity = PoolIdentity::new(pool_address, token_address, denom_address, protocol);
        let mut lp_tracker = LPTokenTracker::new(18, known_routers, config.history_limit);
        lp_tracker.token_address = Some(identity.token_address.clone());
        lp_tracker.pool_address = Some(identity.pool_address.clone());

        Self {
            base: BasePool::new(identity, config),
            lp_tracker,
        }
    }

    pub fn process_sync(
        &mut self,
        sync: &UniswapV2SyncEvent,
        tx: &UniswapV2TxContext,
    ) -> Result<bool> {
        if !same_address(&sync.pair_address, &self.base.identity.pool_address) {
            return Ok(false);
        }

        let token0_decimals = self.decimals_for_token_position(true);
        let token1_decimals = self.decimals_for_token_position(false);
        let reserve0 = parse_raw_f64(&sync.reserve0)? / decimal_scale(token0_decimals);
        let reserve1 = parse_raw_f64(&sync.reserve1)? / decimal_scale(token1_decimals);
        let (token_reserve, denom_reserve) = self.base.map_token_and_denom(reserve0, reserve1);

        self.base.update_reserves(
            token_reserve,
            denom_reserve,
            tx.block_number,
            tx.block_timestamp,
            tx.tx_hash.clone(),
        );

        append_with_history_limit(
            &mut self.base.sync_events,
            json!({
                "block": tx.block_number,
                "tx_hash": tx.tx_hash,
                "token_reserve": token_reserve,
                "denom_reserve": denom_reserve,
                "timestamp": tx.block_timestamp,
            }),
            self.base.config.history_limit,
        );

        Ok(true)
    }

    pub fn update_from_events(
        &mut self,
        events: &UniswapV2TransactionEvents,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        for sync in &events.syncs {
            self.process_sync(sync, tx)?;
        }
        for swap in &events.swaps {
            self.process_swap(swap, tx)?;
        }
        for mint in &events.mints {
            self.process_mint(mint, tx)?;
        }
        for burn in &events.burns {
            self.process_burn(burn, tx)?;
        }
        if self.base.has_liquidity_removal() {
            if let Some(mechanism) = self.base.inferred_scam_mechanism() {
                self.base.mark_scam_mechanism(
                    mechanism.mechanism,
                    mechanism.block_number,
                    mechanism.tx_hash,
                    mechanism.evidence,
                );
            }
        }
        Ok(())
    }

    pub fn process_swap(
        &mut self,
        swap: &UniswapV2SwapEvent,
        tx: &UniswapV2TxContext,
    ) -> Result<bool> {
        if !same_address(&swap.pair_address, &self.base.identity.pool_address) {
            return Ok(false);
        }

        let amount0_in = parse_raw_f64(&swap.amount0_in)?;
        let amount1_in = parse_raw_f64(&swap.amount1_in)?;
        let amount0_out = parse_raw_f64(&swap.amount0_out)?;
        let amount1_out = parse_raw_f64(&swap.amount1_out)?;

        let (token_in, denom_in) = self.base.map_token_and_denom(amount0_in, amount1_in);
        let (token_out, denom_out) = self.base.map_token_and_denom(amount0_out, amount1_out);
        self.base
            .state
            .record_swap(denom_in, token_in, denom_out, token_out);

        let token1_is_denom = self.base.config.token1_is_denom.unwrap_or(true);
        let is_buy = if token1_is_denom {
            amount0_out > 0.0 && amount1_in > 0.0
        } else {
            amount1_out > 0.0 && amount0_in > 0.0
        };
        let is_sell = if token1_is_denom {
            amount0_in > 0.0 && amount1_out > 0.0
        } else {
            amount1_in > 0.0 && amount0_out > 0.0
        };

        append_with_history_limit(
            &mut self.base.swap_events,
            json!({
                "block": tx.block_number,
                "tx_hash": tx.tx_hash,
                "sender": swap.sender,
                "to": swap.to,
                "amount0_in": amount0_in,
                "amount1_in": amount1_in,
                "amount0_out": amount0_out,
                "amount1_out": amount1_out,
                "is_buy": is_buy,
                "is_sell": is_sell,
                "timestamp": tx.block_timestamp,
            }),
            self.base.config.history_limit,
        );

        Ok(true)
    }

    pub fn process_mint(
        &mut self,
        mint: &UniswapV2MintEvent,
        tx: &UniswapV2TxContext,
    ) -> Result<bool> {
        if !same_address(&mint.pair_address, &self.base.identity.pool_address) {
            return Ok(false);
        }

        let amount = mint
            .amount
            .as_deref()
            .map(parse_raw_f64)
            .transpose()?
            .unwrap_or_default();
        self.base.state.total_mints += 1;
        append_with_history_limit(
            &mut self.base.mint_events,
            json!({
                "block": tx.block_number,
                "tx_hash": tx.tx_hash,
                "to": mint.to,
                "amount": amount,
                "timestamp": tx.block_timestamp,
            }),
            self.base.config.history_limit,
        );

        Ok(true)
    }

    pub fn process_burn(
        &mut self,
        burn: &UniswapV2BurnEvent,
        tx: &UniswapV2TxContext,
    ) -> Result<bool> {
        if !same_address(&burn.pair_address, &self.base.identity.pool_address) {
            return Ok(false);
        }

        let amount0 = parse_raw_f64(&burn.amount0)?;
        let amount1 = parse_raw_f64(&burn.amount1)?;
        self.base.state.total_burns += 1;
        append_with_history_limit(
            &mut self.base.burn_events,
            json!({
                "block": tx.block_number,
                "tx_hash": tx.tx_hash,
                "from": burn.sender,
                "amount0": amount0,
                "amount1": amount1,
                "amount": amount0,
                "timestamp": tx.block_timestamp,
            }),
            self.base.config.history_limit,
        );

        Ok(true)
    }

    pub fn latest_sync(&self) -> Option<&Value> {
        self.base.sync_events.last()
    }

    pub fn recent_swaps(&self, count: usize) -> Vec<&Value> {
        let len = self.base.swap_events.len();
        self.base.swap_events[len.saturating_sub(count)..]
            .iter()
            .collect()
    }

    pub fn process_lp_transfer(&mut self, transfer: &LPTransferEvent) -> Result<()> {
        self.lp_tracker.record_transfer_event(transfer)
    }

    pub fn process_lp_approval(&mut self, approval: &LPApprovalEvent) -> Result<ApprovalInfo> {
        self.lp_tracker.record_approval_event(approval)
    }

    pub fn lp_share(&self, address: impl AsRef<str>) -> f64 {
        self.lp_tracker.share(address)
    }

    pub fn lp_holders(&self) -> Vec<LPHolderSnapshot> {
        self.lp_tracker.holder_snapshots()
    }

    pub fn total_approved_to_routers(&self) -> f64 {
        self.lp_tracker.total_approved_to_routers()
    }

    pub fn lp_approved_percentage(&self) -> f64 {
        self.lp_tracker.approved_percentage()
    }

    pub fn last_lp_approval_block(&self) -> Option<u64> {
        self.lp_tracker.last_approval_block()
    }

    pub fn last_lp_approval_event(&self) -> Option<Value> {
        self.lp_tracker.last_approval_event()
    }

    pub fn holders_with_approvals(&self) -> Vec<String> {
        self.lp_tracker.holders_with_approvals()
    }

    pub fn pool_data_for_publishing(&self) -> Value {
        json!({
            "pool_address": self.base.identity.pool_address,
            "token_reserve": self.base.token_reserve(),
            "denom_reserve": self.base.denom_reserve(),
            "trading_enabled": self.base.trading_enabled(),
            "trading_enabled_block": self.base.can_buy_block,
            "trading_enabled_tx": self.base.can_buy_tx,
            "lp_tokens_approved_percentage": self.lp_tracker.approved_percentage(),
        })
    }

    fn decimals_for_token_position(&self, is_token0: bool) -> u8 {
        let denom_decimals = self
            .base
            .config
            .denom_decimals
            .unwrap_or(self.base.config.token_decimals);
        if self.base.config.token1_is_denom.unwrap_or(true) {
            if is_token0 {
                self.base.config.token_decimals
            } else {
                denom_decimals
            }
        } else if is_token0 {
            denom_decimals
        } else {
            self.base.config.token_decimals
        }
    }
}

fn decimal_scale(decimals: u8) -> f64 {
    10_f64.powi(i32::from(decimals))
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn same_address(left: impl AsRef<str>, right: impl AsRef<str>) -> bool {
    normalize_address(left) == normalize_address(right)
}

#[cfg(test)]
mod tests;
