use super::*;

impl UniswapV4Pool {
    pub fn price_from_sqrt_price(&self) -> Option<f64> {
        token_price_from_sqrt_price_x96(
            self.parsed_sqrt_price_x96()?,
            self.base.config.token_decimals,
            self.denom_decimals(),
            self.base.config.token1_is_denom.unwrap_or(false),
        )
    }

    pub fn virtual_reserves(&self) -> Option<VirtualReserves> {
        virtual_reserves_from_liquidity(
            self.active_liquidity,
            self.parsed_sqrt_price_x96()?,
            self.base.config.token_decimals,
            self.denom_decimals(),
            self.base.config.token1_is_denom.unwrap_or(false),
        )
    }

    pub fn denom_decimals(&self) -> u8 {
        self.base.config.denom_decimals.unwrap_or(18)
    }

    pub(super) fn process_modify_liquidity(
        &mut self,
        event: &ProcessedV4ModifyLiquidityEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) {
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_lower,
            event.liquidity_delta,
        );
        update_tick_delta(
            &mut self.tick_liquidity_net,
            event.tick_upper,
            -event.liquidity_delta,
        );
        if current_tick_in_range(self.current_tick, event.tick_lower, event.tick_upper) {
            self.apply_active_liquidity_delta(event.liquidity_delta);
        }
        if event.liquidity_delta >= 0 {
            self.base.state.total_mints += 1;
        } else {
            self.base.state.total_burns += 1;
        }
        let position_update = self.record_liquidity_position(event, tx, transaction);
        let mut modify_event = event_json(event, tx);
        if let Some(object) = modify_event.as_object_mut() {
            object.insert(
                "liquidity_provider".to_string(),
                json!(position_update.owner.clone()),
            );
            object.insert(
                "position_id".to_string(),
                json!(position_update.position_id.clone()),
            );
            object.insert(
                "position_manager_address".to_string(),
                json!(position_update.position_manager_address.clone()),
            );
            object.insert(
                "position_liquidity".to_string(),
                json!(position_update.liquidity.to_string()),
            );
        }
        append_with_history_limit(
            &mut self.modify_liquidity_events,
            modify_event.clone(),
            self.base.config.history_limit,
        );
        append_with_history_limit(
            &mut self.liquidity_position_events,
            modify_event,
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    fn apply_active_liquidity_delta(&mut self, delta: i128) {
        if delta >= 0 {
            self.active_liquidity = self.active_liquidity.saturating_add(delta as u128);
        } else {
            self.active_liquidity = self.active_liquidity.saturating_sub(delta.unsigned_abs());
        }
    }

    pub(super) fn refresh_virtual_reserves(&mut self, tx: &UniswapV2TxContext) {
        let Some(reserves) = self.virtual_reserves() else {
            return;
        };
        self.last_virtual_reserves = Some(reserves.into());
        self.base.update_reserves(
            reserves.token_reserve,
            reserves.denom_reserve,
            tx.block_number,
            tx.block_timestamp,
            tx.tx_hash.clone(),
        );
    }

    fn parsed_sqrt_price_x96(&self) -> Option<U256> {
        let value = self.sqrt_price_x96.as_deref()?;
        U256::from_str_radix(value.trim_start_matches("0x"), 10).ok()
    }
}
