use super::*;

impl UniswapV4Pool {
    pub(super) fn process_initialize(
        &mut self,
        event: &ProcessedV4InitializeEvent,
        tx: &UniswapV2TxContext,
    ) {
        self.pool_key = UniswapV4PoolKey {
            currency0: address_string(&event.currency0),
            currency1: address_string(&event.currency1),
            fee: event.fee,
            tick_spacing: event.tick_spacing,
            hooks: address_string(&event.hooks),
        }
        .normalized();
        self.current_tick = Some(event.tick);
        self.sqrt_price_x96 = Some(event.sqrt_price_x96.to_string());
        append_with_history_limit(
            &mut self.initialize_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
        self.refresh_virtual_reserves(tx);
    }

    pub(super) fn process_donate(
        &mut self,
        event: &ProcessedV4DonateEvent,
        tx: &UniswapV2TxContext,
    ) {
        append_with_history_limit(
            &mut self.donate_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }

    pub(super) fn process_protocol_fee_update(
        &mut self,
        event: &ProcessedV4FeeUpdatedEvent,
        tx: &UniswapV2TxContext,
    ) {
        self.protocol_fee = Some(event.protocol_fee);
        append_with_history_limit(
            &mut self.fee_update_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }

    pub(super) fn process_dynamic_fee_update(
        &mut self,
        event: &ProcessedV4DynamicLPFeeUpdatedEvent,
        tx: &UniswapV2TxContext,
    ) {
        self.dynamic_lp_fee = Some(event.dynamic_lp_fee);
        append_with_history_limit(
            &mut self.fee_update_events,
            event_json(event, tx),
            self.base.config.history_limit,
        );
    }
}

pub(super) enum V4PoolAction<'a> {
    Initialize(&'a ProcessedV4InitializeEvent),
    ModifyLiquidity(&'a ProcessedV4ModifyLiquidityEvent),
    Swap(&'a ProcessedV4SwapEvent),
    Donate(&'a ProcessedV4DonateEvent),
    ProtocolFeeUpdate(&'a ProcessedV4FeeUpdatedEvent),
    DynamicFeeUpdate(&'a ProcessedV4DynamicLPFeeUpdatedEvent),
}

impl V4PoolAction<'_> {
    pub(super) fn log_index(&self) -> u64 {
        match self {
            Self::Initialize(event) => event.log_index,
            Self::ModifyLiquidity(event) => event.log_index,
            Self::Swap(event) => event.log_index,
            Self::Donate(event) => event.log_index,
            Self::ProtocolFeeUpdate(event) => event.log_index,
            Self::DynamicFeeUpdate(event) => event.log_index,
        }
    }
}
