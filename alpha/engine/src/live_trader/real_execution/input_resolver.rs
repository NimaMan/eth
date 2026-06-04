use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::execution::real::LiveTxPlanningInputResolver;
use async_trait::async_trait;
use chrono::Utc;
use eth_alpha_core::{
    error::AlphaCoreError,
    ids::{PositionId, TokenPoolId},
    market::PoolSnapshot,
    order::OrderIntent,
    position::{Position, PositionKey},
};
use eth_alpha_store::PostgresTradingStore;
use eth_live_trading::{LivePrioritySellPlannerInput, PlannerTxContext, TxPrepRequestContext};
use serde_json::json;

#[derive(Clone)]
pub(super) struct LiveRealInputResolver {
    pub(super) store: PostgresTradingStore,
    pub(super) pools: Arc<Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    pub(super) current_block: Arc<AtomicU64>,
    pub(super) last_frame_block: Arc<AtomicU64>,
    pub(super) last_frame_hash: Arc<Mutex<Option<String>>>,
    pub(super) from: String,
    pub(super) run_id: String,
    pub(super) chain_id: u64,
}

#[async_trait]
impl LiveTxPlanningInputResolver for LiveRealInputResolver {
    async fn resolve_priority_sell_input(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<LivePrioritySellPlannerInput> {
        let position = self.resolve_position(intent).await?;
        let pool = self.resolve_pool(intent).await?;
        let current_block = self.current_block(&pool);
        let required_state_block = required_state_block(current_block, &pool);
        let now = Utc::now().timestamp().max(0) as u64;

        Ok(LivePrioritySellPlannerInput {
            context: PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: self.chain_id,
                    from: self.from.clone(),
                    strategy_name: intent.strategy_name.0.clone(),
                    strategy_run_id: Some(self.run_id.clone()),
                    observed_block: Some(current_block),
                    required_state_block,
                    source_metadata: self.source_metadata(
                        &pool,
                        current_block,
                        required_state_block,
                    ),
                },
                current_block,
                deadline_unix_secs: now.saturating_add(intent.deadline_secs),
            },
            intent: intent.clone(),
            position,
            pool,
            min_output_amount: None,
            source_metadata: json!({
                "decision_reason": intent.decision_reason.clone(),
                "trade_id": intent.trade_id.clone(),
            }),
        })
    }
}

impl LiveRealInputResolver {
    pub(super) async fn resolve_buy_input(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<LivePrioritySellPlannerInput> {
        let pool = self.resolve_pool(intent).await?;
        let current_block = self.current_block(&pool);
        let required_state_block = required_state_block(current_block, &pool);
        let now = Utc::now().timestamp().max(0) as u64;
        let trade_id = intent.trade_id.clone().ok_or_else(|| {
            AlphaCoreError::Execution(
                "live real buy planner requires engine-assigned trade_id".to_string(),
            )
        })?;
        let position = Position::with_trade_id(
            PositionId(trade_id.0.clone()),
            trade_id,
            PositionKey {
                portfolio_id: intent.portfolio_id.clone(),
                wallet_id: intent.wallet_id.clone(),
                strategy_name: intent.strategy_name.clone(),
                token_address: intent.token_address,
                pool_address: intent.pool_address.clone(),
                protocol: intent.protocol.clone(),
            },
        );

        Ok(LivePrioritySellPlannerInput {
            context: PlannerTxContext {
                tx: TxPrepRequestContext {
                    chain_id: self.chain_id,
                    from: self.from.clone(),
                    strategy_name: intent.strategy_name.0.clone(),
                    strategy_run_id: Some(self.run_id.clone()),
                    observed_block: Some(current_block),
                    required_state_block,
                    source_metadata: self.source_metadata(
                        &pool,
                        current_block,
                        required_state_block,
                    ),
                },
                current_block,
                deadline_unix_secs: now.saturating_add(intent.deadline_secs),
            },
            intent: intent.clone(),
            position,
            pool,
            min_output_amount: None,
            source_metadata: json!({
                "decision_reason": intent.decision_reason.clone(),
                "trade_id": intent.trade_id.clone(),
            }),
        })
    }

    async fn resolve_pool(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<PoolSnapshot> {
        let pool = {
            let pools = self
                .pools
                .lock()
                .map_err(|_| AlphaCoreError::Execution("pool cache lock poisoned".to_string()))?;
            pools.get(&intent.pool_address).cloned()
        };
        pool.ok_or_else(|| {
            AlphaCoreError::Execution(format!(
                "live real planner has no pool snapshot for {}",
                intent.pool_address
            ))
        })
    }

    async fn resolve_position(
        &self,
        intent: &OrderIntent,
    ) -> eth_alpha_core::error::Result<Position> {
        let positions = self
            .store
            .load_active_positions(&intent.strategy_name.0)
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;

        positions
            .into_iter()
            .find(|position| {
                intent
                    .trade_id
                    .as_ref()
                    .map(|trade_id| &position.trade_id == trade_id)
                    .unwrap_or(true)
                    && position.key.strategy_name == intent.strategy_name
                    && position.key.token_address == intent.token_address
                    && position.key.pool_address == intent.pool_address
                    && position.can_submit_exit()
            })
            .ok_or_else(|| {
                AlphaCoreError::Execution(format!(
                    "no active sellable position found for strategy={} token={} pool={}",
                    intent.strategy_name.0, intent.token_address, intent.pool_address
                ))
            })
    }

    fn current_block(&self, pool: &PoolSnapshot) -> u64 {
        match self.current_block.load(Ordering::Relaxed) {
            0 => pool.latest_block,
            block => block,
        }
    }

    fn last_frame_hash_value(&self) -> Option<String> {
        self.last_frame_hash.lock().ok().and_then(|g| g.clone())
    }

    fn source_metadata(
        &self,
        pool: &PoolSnapshot,
        current_block: u64,
        required_state_block: u64,
    ) -> serde_json::Value {
        json!({
            "resolver": "eth_alpha_live_trader_real_execution",
            "execution_mode": "eth-tx-real",
            "route": "uniswap_v2_trading_vault",
            "simulation_provider": "reth_exact_calldata_uniswap_v2_trading_vault",
            "gas_rank_provider": "eth_chain_server_gas_rank",
            "min_output_policy": "exact_pre_submit_simulation_slippage_bps",
            "decision_block": current_block,
            "required_state_block": required_state_block,
            "pool_creation_block": pool.creation_block,
            "pool_latest_block": pool.latest_block,
            "input_frame_block": self.last_frame_block.load(Ordering::Relaxed),
            "input_frame_hash": self.last_frame_hash_value(),
        })
    }
}

fn required_state_block(current_block: u64, pool: &PoolSnapshot) -> u64 {
    let mut required = current_block.max(pool.latest_block);
    if let Some(creation_block) = pool.creation_block {
        required = required.max(creation_block);
    }
    required
}
