use eth_alpha_core::{
    error::Result,
    ids::TokenPoolId,
    market::MarketEvent,
    position::{Position, PositionState},
    risk::RiskPolicy,
    store::TradingStore,
};

use crate::{
    snapshots::{
        should_snapshot_position_for_pool, simulated_value_snapshot, valuation_safe_pool,
        zero_value_snapshot,
    },
    AlphaEngine, EngineExecutionAdapter,
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
        if !self.written_snapshot_keys.insert(key) {
            return Ok(());
        }
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
