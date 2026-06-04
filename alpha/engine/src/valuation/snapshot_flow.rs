use std::collections::HashMap;

use eth_alpha_core::{
    error::Result,
    ids::{TokenAddress, TokenPoolId},
    market::{MarketEvent, PoolSnapshot},
    position::{Position, PositionState},
    risk::RiskPolicy,
    store::TradingStore,
};

use crate::{AlphaEngine, EngineExecutionAdapter};

use super::{
    should_snapshot_position_for_pool, simulated_value_snapshot, valuation_safe_pool,
    zero_value_snapshot,
};

pub(crate) fn market_open_valuation_pool(event: &MarketEvent) -> Option<&TokenPoolId> {
    match event {
        MarketEvent::PoolUpdated { pool, .. } => Some(&pool.address),
        MarketEvent::TokenUpdated { .. } | MarketEvent::BlockCompleted { .. } => None,
    }
}

pub(crate) fn market_owns_open_valuation(
    market_valuation_pool: Option<&TokenPoolId>,
    position: &Position,
) -> bool {
    market_valuation_pool
        .map(|pool_address| pool_address == &position.key.pool_address)
        .unwrap_or(false)
}

impl<E, R, S> AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    pub(crate) async fn snapshot_open_positions_for_pool(
        &mut self,
        event: &MarketEvent,
    ) -> Result<()> {
        let MarketEvent::PoolUpdated { pool, block_number } = event else {
            return Ok(());
        };
        let positions = self
            .portfolio
            .positions
            .values()
            .filter(|position| {
                position.key.pool_address == pool.address
                    && should_snapshot_position_for_pool(position)
            })
            .cloned()
            .collect::<Vec<_>>();

        for position in positions {
            let snapshot = if position.drained {
                Some(zero_value_snapshot(&position, *block_number, Some(pool)))
            } else {
                self.execution
                    .simulate_position_value(&position, pool)
                    .await?
                    .map(|value| simulated_value_snapshot(&position, value, Some(pool)))
            };
            if let Some(snapshot) = snapshot {
                self.append_position_snapshot_once(snapshot).await?;
            }
        }
        Ok(())
    }

    /// Value every open position against the supplied pool-snapshot cache and
    /// write a position snapshot for each. This is the real-execution parity
    /// path: the live-backtest values open positions on every `PoolUpdated`
    /// event, but the real runner receives almost no `PoolUpdated` events (its
    /// block-frame poll is near-instant and held pools fall out of the tracker),
    /// so the real loop calls this once per block instead. The pool snapshot
    /// only identifies the pool/token/denom; the execution adapter values the
    /// position by simulating a sell against exact current block state, so a
    /// stale cached snapshot still yields an accurate current value.
    pub async fn value_open_positions(
        &mut self,
        block_number: u64,
        pools: &HashMap<TokenPoolId, PoolSnapshot>,
    ) -> Result<usize> {
        self.value_open_positions_filtered(block_number, pools, None)
            .await
    }

    /// Value only the open positions whose token appears in `updated_tokens`.
    ///
    /// This is the live-backtest parity trigger for token-affecting blocks. The
    /// backtest values positions on every `PoolUpdated` event, but a
    /// holder-balance drain (or any token/control activity that does not move
    /// pool reserves) produces no `PoolUpdated`, so the position is never
    /// re-valued at that block. Calling this with the block frame's
    /// `updated_tokens` revalues a held position whenever its token had activity,
    /// even without a pool reserve update — token-scoped so normal blocks add no
    /// extra snapshots.
    pub async fn value_open_positions_for_tokens(
        &mut self,
        block_number: u64,
        updated_tokens: &std::collections::HashSet<TokenAddress>,
        pools: &HashMap<TokenPoolId, PoolSnapshot>,
    ) -> Result<usize> {
        if updated_tokens.is_empty() {
            return Ok(0);
        }
        self.value_open_positions_filtered(block_number, pools, Some(updated_tokens))
            .await
    }

    async fn value_open_positions_filtered(
        &mut self,
        block_number: u64,
        pools: &HashMap<TokenPoolId, PoolSnapshot>,
        token_filter: Option<&std::collections::HashSet<TokenAddress>>,
    ) -> Result<usize> {
        let positions = self
            .portfolio
            .positions
            .values()
            .filter(|position| should_snapshot_position_for_pool(position))
            .filter(|position| {
                token_filter
                    .map(|tokens| tokens.contains(&position.key.token_address))
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut written = 0usize;
        for position in positions {
            let Some(pool) = pools.get(&position.key.pool_address).cloned() else {
                continue;
            };
            let snapshot = if position.drained {
                Some(zero_value_snapshot(&position, block_number, Some(&pool)))
            } else {
                self.execution
                    .simulate_position_value(&position, &pool)
                    .await?
                    .map(|value| simulated_value_snapshot(&position, value, Some(&pool)))
            };
            if let Some(snapshot) = snapshot {
                self.append_position_snapshot_once(snapshot).await?;
                written += 1;
            }
        }
        Ok(written)
    }

    pub(crate) async fn append_position_snapshot_once(
        &mut self,
        snapshot: eth_alpha_core::position::PositionSnapshot,
    ) -> Result<()> {
        let key = format!(
            "{}:{}:{:?}:{}",
            snapshot.trade_id.0,
            snapshot.block_number,
            snapshot.state,
            snapshot
                .valuation_block_number
                .map(|block| block.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
        if let Some(previous_observed_block) =
            self.written_snapshot_observed_blocks.get(&key).copied()
        {
            if !observed_block_improves(previous_observed_block, snapshot.observed_block_number) {
                return Ok(());
            }
        }
        self.written_snapshot_observed_blocks
            .insert(key, snapshot.observed_block_number);
        self.store.append_position_snapshot(&snapshot).await
    }

    pub(crate) async fn snapshot_confirmed_buy_position(
        &mut self,
        position: &Position,
    ) -> Result<()> {
        if position.state != PositionState::BuyConfirmed {
            return Ok(());
        }

        let valuation_block = position
            .entry_block
            .or(self.current_event_block)
            .unwrap_or_default();
        let Some(pool) = valuation_safe_pool(
            self.pool_snapshots.get(&position.key.pool_address),
            valuation_block,
        )
        .cloned() else {
            return Ok(());
        };

        if let Some(value) = self
            .execution
            .simulate_position_value(position, &pool)
            .await?
        {
            let snapshot = simulated_value_snapshot(position, value, Some(&pool));
            self.append_position_snapshot_once(snapshot).await?;
        }

        Ok(())
    }
}

fn observed_block_improves(previous: Option<u64>, next: Option<u64>) -> bool {
    match (previous, next) {
        (None, Some(_)) => true,
        (Some(previous), Some(next)) => next > previous,
        _ => false,
    }
}
