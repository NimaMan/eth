use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use eth_live_feed::{LiveTokenPoolSnapshot, LiveTokenProgress, LiveTokenSnapshot};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct LiveBlockFrameStore {
    inner: Arc<RwLock<VecDeque<LiveBlockFrame>>>,
    capacity: usize,
    tx: broadcast::Sender<LiveBlockFrame>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveBlockFrame {
    pub schema: &'static str,
    pub block_number: u64,
    pub block_hash: String,
    pub published_at_unix_ms: u64,
    pub status: LiveBlockFrameStatus,
    pub updated_tokens: Vec<String>,
    pub updated_v2_pools: Vec<String>,
    pub updated_v3_pools: Vec<String>,
    pub updated_v4_pools: Vec<String>,
    pub tokens: Vec<LiveTokenSnapshot>,
    pub pools: Vec<LiveBlockFramePool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveBlockFrameStatus {
    pub progress: LiveTokenProgress,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveBlockFramePool {
    pub token_address: String,
    pub token_decimals: Option<u8>,
    pub pool_address: String,
    pub protocol: String,
    pub pool_id: Option<String>,
    pub pool_manager_address: Option<String>,
    pub currency0: Option<String>,
    pub currency1: Option<String>,
    pub fee_tier: Option<u32>,
    pub tick_spacing: Option<i32>,
    pub hooks: Option<String>,
    pub denom_address: Option<String>,
    pub denom_symbol: Option<String>,
    pub currency: Option<String>,
    pub denom_reserve: Option<f64>,
    pub token_reserve: Option<f64>,
    pub price: Option<f64>,
    pub initial_price: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub creation_block: Option<u64>,
    pub can_buy_block: Option<u64>,
    pub latest_block_number: Option<u64>,
    pub runtime_state: LiveBlockFramePoolRuntimeState,
    pub lp_last_approval_block: Option<u64>,
    pub lp_last_approval: Option<Value>,
    pub lp_approval_count: u64,
    pub lp_approved_percentage: Option<f64>,
    pub liquidity_removal: bool,
    pub liquidity_removal_block: Option<u64>,
    pub liquidity_removal_tx_hash: Option<String>,
    pub liquidity_removal_label: Option<String>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub is_scam: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveBlockFramePoolRuntimeState {
    pub last_update_block: Option<u64>,
    pub last_sync_block: Option<u64>,
    pub can_buy: Option<bool>,
    pub can_sell: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct LiveBlockFrameInput {
    pub block_number: u64,
    pub block_hash: String,
    pub progress: LiveTokenProgress,
    pub updated_tokens: Vec<String>,
    pub updated_v2_pools: Vec<String>,
    pub updated_v3_pools: Vec<String>,
    pub updated_v4_pools: Vec<String>,
    pub token_snapshots: Vec<LiveTokenSnapshot>,
}

impl LiveBlockFrameStore {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self {
            inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
            tx,
        }
    }

    pub fn record(&self, frame: LiveBlockFrame) {
        {
            let mut frames = self.inner.write().expect("live block frame lock poisoned");
            if let Some(existing) = frames
                .iter_mut()
                .find(|existing| existing.block_number == frame.block_number)
            {
                *existing = frame.clone();
            } else {
                frames.push_front(frame.clone());
                while frames.len() > self.capacity {
                    frames.pop_back();
                }
            }
        }
        let _ = self.tx.send(frame);
    }

    pub fn latest(&self) -> Option<LiveBlockFrame> {
        self.inner
            .read()
            .expect("live block frame lock poisoned")
            .front()
            .cloned()
    }

    pub fn get(&self, block_number: u64) -> Option<LiveBlockFrame> {
        self.inner
            .read()
            .expect("live block frame lock poisoned")
            .iter()
            .find(|frame| frame.block_number == block_number)
            .cloned()
    }

    pub fn next_after(&self, after_block: u64) -> Option<LiveBlockFrame> {
        self.inner
            .read()
            .expect("live block frame lock poisoned")
            .iter()
            .filter(|frame| frame.block_number > after_block)
            .min_by_key(|frame| frame.block_number)
            .cloned()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LiveBlockFrame> {
        self.tx.subscribe()
    }
}

impl LiveBlockFrame {
    pub fn from_input(input: LiveBlockFrameInput) -> Self {
        let updated_pool_ids = updated_pool_ids(
            &input.updated_v2_pools,
            &input.updated_v3_pools,
            &input.updated_v4_pools,
        );
        let pools = input
            .token_snapshots
            .iter()
            .flat_map(|token| {
                token
                    .pools
                    .iter()
                    .filter(|pool| pool_is_in_frame(pool, input.block_number, &updated_pool_ids))
                    .map(|pool| LiveBlockFramePool::from_snapshot(token, pool))
            })
            .collect::<Vec<_>>();

        let mut progress = input.progress;
        progress.current_block = Some(input.block_number);
        progress.current_block_hash = Some(input.block_hash.clone());

        Self {
            schema: "eth_alpha_live_block_frame_v1",
            block_number: input.block_number,
            block_hash: input.block_hash,
            published_at_unix_ms: now_unix_ms(),
            status: LiveBlockFrameStatus { progress },
            updated_tokens: input.updated_tokens,
            updated_v2_pools: input.updated_v2_pools,
            updated_v3_pools: input.updated_v3_pools,
            updated_v4_pools: input.updated_v4_pools,
            tokens: input.token_snapshots,
            pools,
        }
    }
}

impl LiveBlockFramePool {
    fn from_snapshot(token: &LiveTokenSnapshot, pool: &LiveTokenPoolSnapshot) -> Self {
        Self {
            token_address: pool.token_address.clone(),
            token_decimals: Some(token.decimals),
            pool_address: pool.pool_address.clone(),
            protocol: pool.protocol.clone(),
            pool_id: pool.pool_id.clone(),
            pool_manager_address: pool.pool_manager_address.clone(),
            currency0: pool.currency0.clone(),
            currency1: pool.currency1.clone(),
            fee_tier: pool.fee_tier,
            tick_spacing: pool.tick_spacing,
            hooks: pool.hooks.clone(),
            denom_address: Some(pool.denom_address.clone()),
            denom_symbol: Some(pool.denom_symbol.clone()),
            currency: Some(pool.denom_symbol.clone()),
            denom_reserve: Some(pool.denom_reserve),
            token_reserve: Some(pool.token_reserve),
            price: Some(pool.price),
            initial_price: pool.initial_price,
            price_ratio_to_initial: pool.price_ratio_to_initial,
            creation_block: pool.creation_block,
            can_buy_block: pool.trading_enabled_block,
            latest_block_number: pool.latest_block_number,
            runtime_state: LiveBlockFramePoolRuntimeState {
                last_update_block: Some(pool.runtime_state.last_update_block)
                    .filter(|block| *block > 0),
                last_sync_block: Some(pool.runtime_state.last_sync_block)
                    .filter(|block| *block > 0),
                can_buy: Some(pool.runtime_state.can_buy),
                can_sell: Some(pool.runtime_state.can_sell),
            },
            lp_last_approval_block: pool.lp_last_approval_block,
            lp_last_approval: pool.lp_last_approval.clone(),
            lp_approval_count: pool.lp_approval_count,
            lp_approved_percentage: pool.lp_tokens_approved_percentage,
            liquidity_removal: pool.liquidity_removal,
            liquidity_removal_block: pool.liquidity_removal_block,
            liquidity_removal_tx_hash: pool.liquidity_removal_tx_hash.clone(),
            liquidity_removal_label: pool.liquidity_removal_label.clone(),
            can_buy: pool.can_buy,
            can_sell: pool.can_sell,
            is_scam: pool.is_scam,
        }
    }
}

fn pool_is_in_frame(
    pool: &LiveTokenPoolSnapshot,
    block_number: u64,
    updated_pool_ids: &BTreeSet<String>,
) -> bool {
    pool.latest_block_number == Some(block_number)
        || updated_pool_ids.contains(&pool.pool_address.to_ascii_lowercase())
}

fn updated_pool_ids(
    updated_v2_pools: &[String],
    updated_v3_pools: &[String],
    updated_v4_pools: &[String],
) -> BTreeSet<String> {
    updated_v2_pools
        .iter()
        .chain(updated_v3_pools)
        .chain(updated_v4_pools)
        .map(|pool| pool.to_ascii_lowercase())
        .collect()
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
