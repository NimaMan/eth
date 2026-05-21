use super::*;

impl UniswapV4Pool {
    pub(super) fn process_swap(&mut self, event: &ProcessedV4SwapEvent, tx: &UniswapV2TxContext) {
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        self.active_liquidity = event.liquidity;
        self.last_swap_fee = Some(event.fee);

        let token0_amount = scale_i128(event.amount0, token0_decimals(self));
        let token1_amount = scale_i128(event.amount1, token1_decimals(self));
        let (event_token_amount, event_denom_amount) =
            self.base.map_token_and_denom(token0_amount, token1_amount);
        let token_amount = -event_token_amount;
        let denom_amount = -event_denom_amount;
        self.base.state.record_swap(
            denom_amount.max(0.0),
            token_amount.max(0.0),
            (-denom_amount).max(0.0),
            (-token_amount).max(0.0),
        );
        let mut swap_event = event_json(event, tx);
        if let Some(object) = swap_event.as_object_mut() {
            object.insert("token_amount".to_string(), json!(token_amount));
            object.insert("denom_amount".to_string(), json!(denom_amount));
            object.insert(
                "is_buy".to_string(),
                json!(token_amount < 0.0 && denom_amount > 0.0),
            );
            object.insert(
                "is_sell".to_string(),
                json!(token_amount > 0.0 && denom_amount < 0.0),
            );
        }
        append_with_history_limit(
            &mut self.base.swap_events,
            swap_event,
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }
}
