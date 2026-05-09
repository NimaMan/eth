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

        self.base
            .mark_can_buy_from_event(tx.block_number, tx.tx_hash.clone(), tx.block_timestamp);

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
mod tests {
    use super::*;

    fn pool() -> UniswapV2Pool {
        UniswapV2Pool::new(
            "0xPOOL",
            "0xTOKEN",
            "0xDENOM",
            BasePoolConfig {
                token_decimals: 18,
                denom_decimals: Some(18),
                token1_is_denom: Some(true),
                history_limit: 10,
                denom_threshold: 0.05,
                threshold_unit: Some("ETH".to_string()),
                test_buy_amount_eth: 0.01,
            },
            ["0xROUTER"],
        )
    }

    #[test]
    fn sync_updates_reserves_in_pool_orientation() {
        let mut pool = pool();
        let tx = UniswapV2TxContext::new(100, 1_700, "0xTX");

        pool.process_sync(
            &UniswapV2SyncEvent {
                pair_address: "0xpool".to_string(),
                reserve0: "100000000000000000000".to_string(),
                reserve1: "2000000000000000000".to_string(),
            },
            &tx,
        )
        .unwrap();

        assert_eq!(pool.base.token_reserve(), 100.0);
        assert_eq!(pool.base.denom_reserve(), 2.0);
        assert_eq!(pool.base.price(), 0.02);
        assert_eq!(pool.latest_sync().unwrap()["tx_hash"], "0xTX");
    }

    #[test]
    fn swap_tracks_volumes_direction_and_trading_status() {
        let mut pool = pool();
        let tx = UniswapV2TxContext::new(101, 1_701, "0xSWAP");

        pool.process_swap(
            &UniswapV2SwapEvent {
                pair_address: "0xPOOL".to_string(),
                sender: Some("0xSENDER".to_string()),
                to: Some("0xTO".to_string()),
                amount0_in: "0".to_string(),
                amount1_in: "1000000000000000000".to_string(),
                amount0_out: "50000000000000000000".to_string(),
                amount1_out: "0".to_string(),
            },
            &tx,
        )
        .unwrap();

        assert!(pool.base.trading_enabled());
        assert_eq!(pool.base.state.total_swaps, 1);
        assert_eq!(pool.base.state.denom_volume_in, 1_000_000_000_000_000_000.0);
        assert_eq!(pool.recent_swaps(1)[0]["is_buy"], true);
    }

    #[test]
    fn mint_and_burn_append_events_and_counters() {
        let mut pool = pool();
        let tx = UniswapV2TxContext::new(102, 1_702, "0xLIQ");

        pool.process_mint(
            &UniswapV2MintEvent {
                pair_address: "0xPOOL".to_string(),
                to: Some("0xTO".to_string()),
                amount: Some("10".to_string()),
            },
            &tx,
        )
        .unwrap();
        pool.process_burn(
            &UniswapV2BurnEvent {
                pair_address: "0xPOOL".to_string(),
                sender: Some("0xFROM".to_string()),
                amount0: "3".to_string(),
                amount1: "4".to_string(),
            },
            &tx,
        )
        .unwrap();

        assert_eq!(pool.base.state.total_mints, 1);
        assert_eq!(pool.base.state.total_burns, 1);
        assert_eq!(pool.base.burn_events[0]["amount"], 3.0);
    }

    #[test]
    fn lp_tracker_tracks_supply_balances_and_router_approvals() {
        let mut pool = pool();
        pool.lp_tracker
            .record_transfer(ZERO_ADDRESS, "0xHOLDER", 100.0, 1, "0xMINT", None);
        pool.lp_tracker
            .record_approval("0xHOLDER", "0xROUTER", 80.0, 2, "0xAPPROVE", Some(55));

        assert_eq!(pool.lp_tracker.total_supply, 100.0);
        assert_eq!(pool.lp_tracker.share("0xholder"), 100.0);
        assert_eq!(pool.lp_tracker.total_approved_to_routers(), 80.0);
        assert_eq!(pool.lp_tracker.approved_percentage(), 80.0);
        assert_eq!(pool.lp_tracker.last_approval_block(), Some(2));
        assert_eq!(
            pool.lp_tracker.last_approval_event().unwrap()["tx_hash"],
            "0xAPPROVE"
        );
        assert_eq!(pool.lp_tracker.holder_snapshots()[0].address, "0xholder");
        assert_eq!(
            pool.lp_tracker.holders_with_approvals(),
            vec!["0xholder".to_string()]
        );
    }

    #[test]
    fn pool_data_for_publishing_matches_python_keys() {
        let mut pool = pool();
        pool.base.mark_can_buy_from_event(200, "0xBUY", 1_800);
        pool.base.update_reserves(100.0, 2.0, 200, 1_800, "0xSYNC");

        let data = pool.pool_data_for_publishing();

        assert_eq!(data["pool_address"], "0xpool");
        assert_eq!(data["trading_enabled"], true);
        assert_eq!(data["trading_enabled_block"], 200);
        assert_eq!(data["trading_enabled_tx"], "0xBUY");
    }

    #[test]
    fn update_from_events_processes_v2_event_batch() {
        let mut pool = pool();
        let tx = UniswapV2TxContext::new(300, 1_900, "0xBATCH");
        let events = UniswapV2TransactionEvents {
            syncs: vec![UniswapV2SyncEvent {
                pair_address: "0xPOOL".to_string(),
                reserve0: "100000000000000000000".to_string(),
                reserve1: "2000000000000000000".to_string(),
            }],
            swaps: vec![UniswapV2SwapEvent {
                pair_address: "0xPOOL".to_string(),
                sender: None,
                to: None,
                amount0_in: "0".to_string(),
                amount1_in: "1".to_string(),
                amount0_out: "2".to_string(),
                amount1_out: "0".to_string(),
            }],
            mints: vec![UniswapV2MintEvent {
                pair_address: "0xPOOL".to_string(),
                to: None,
                amount: None,
            }],
            burns: vec![UniswapV2BurnEvent {
                pair_address: "0xPOOL".to_string(),
                sender: None,
                amount0: "0".to_string(),
                amount1: "0".to_string(),
            }],
        };

        pool.update_from_events(&events, &tx).unwrap();

        assert_eq!(pool.base.sync_events.len(), 1);
        assert_eq!(pool.base.swap_events.len(), 1);
        assert_eq!(pool.base.mint_events[0]["amount"], 0.0);
        assert_eq!(pool.base.burn_events.len(), 1);
    }

    #[test]
    fn lp_event_helpers_scale_raw_amounts_like_python() {
        let mut pool = pool();

        pool.process_lp_transfer(&LPTransferEvent {
            from_address: ZERO_ADDRESS.to_string(),
            to_address: "0xHOLDER".to_string(),
            amount: "100000000000000000000".to_string(),
            block_number: Some(1),
            tx_hash: Some("0xMINT".to_string()),
            log_index: Some(7),
        })
        .unwrap();
        pool.process_lp_approval(&LPApprovalEvent {
            owner: "0xHOLDER".to_string(),
            spender: "0xROUTER".to_string(),
            amount: "25000000000000000000".to_string(),
            block_number: Some(2),
            tx_hash: Some("0xAPPROVE".to_string()),
            block_timestamp: Some(55),
        })
        .unwrap();

        assert_eq!(pool.lp_share("0xholder"), 100.0);
        assert_eq!(pool.total_approved_to_routers(), 25.0);
        assert_eq!(pool.lp_approved_percentage(), 25.0);
        assert_eq!(pool.last_lp_approval_block(), Some(2));
        assert_eq!(pool.holders_with_approvals(), vec!["0xholder".to_string()]);
    }
}
