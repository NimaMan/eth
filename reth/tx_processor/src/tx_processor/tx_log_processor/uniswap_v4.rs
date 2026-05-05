use super::super::data_models::receipt_models::*;
use super::{DecodedEvent, LogDecoder};
use alloy_primitives::{Address, Log as AlloyLog, B256, U256};
use eyre::Result;

impl LogDecoder {
    pub(super) fn decode_uniswap_v4_swap(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 192 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount0_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?;
        let amount0 = i128::from_be_bytes(
            amount0_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?,
        );

        let amount1_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?;
        let amount1 = i128::from_be_bytes(
            amount1_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?,
        );

        let sqrt_price_x96 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?,
        );

        let liquidity_bytes = log
            .data
            .data
            .get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = u128::from_be_bytes(
            liquidity_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert liquidity bytes"))?,
        );

        let tick_bytes = log
            .data
            .data
            .get(128..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?,
        );

        let fee_bytes = log
            .data
            .data
            .get(160..192)
            .ok_or_else(|| eyre::eyre!("Invalid data length for fee"))?;
        let fee = u32::from_be_bytes(
            fee_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4SwapEvent(UniswapV4SwapEvent {
            pool_manager_address: log.address,
            event_id: id,
            sender,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
            fee,
            log_index,
        })))
    }

    // Uniswap V4 Initialize decoder
    pub(super) fn decode_uniswap_v4_initialize(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 160 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let currency0_bytes: &[u8] = log.topics()[2].as_ref();
        let currency1_bytes: &[u8] = log.topics()[3].as_ref();

        if currency0_bytes.len() < 32 || currency1_bytes.len() < 32 {
            return Ok(None);
        }

        let currency0 = Address::from_slice(&currency0_bytes[12..32]);
        let currency1 = Address::from_slice(&currency1_bytes[12..32]);

        let fee_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for fee"))?;
        let fee = u32::from_be_bytes(
            fee_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?,
        );

        let tick_spacing_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_spacing"))?;
        let tick_spacing = i32::from_be_bytes(
            tick_spacing_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_spacing bytes"))?,
        );

        let hooks_bytes = log
            .data
            .data
            .get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for hooks"))?;
        let hooks = Address::from_slice(&hooks_bytes[12..32]);

        let sqrt_price_x96 = U256::from_be_slice(
            log.data
                .data
                .get(96..128)
                .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?,
        );

        let tick_bytes = log
            .data
            .data
            .get(128..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4InitializeEvent(
            UniswapV4InitializeEvent {
                pool_manager_address: log.address,
                event_id: id,
                currency0,
                currency1,
                fee,
                tick_spacing,
                hooks,
                sqrt_price_x96,
                tick,
                log_index,
            },
        )))
    }

    // Uniswap V4 ModifyLiquidity decoder
    pub(super) fn decode_uniswap_v4_modify_liquidity(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 128 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let tick_lower_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_lower"))?;
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?,
        );

        let tick_upper_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_upper"))?;
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?,
        );

        let liquidity_delta_bytes = log
            .data
            .data
            .get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity_delta"))?;
        let liquidity_delta = i128::from_be_bytes(
            liquidity_delta_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert liquidity_delta bytes"))?,
        );

        let salt = B256::from_slice(
            log.data
                .data
                .get(96..128)
                .ok_or_else(|| eyre::eyre!("Invalid data length for salt"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4ModifyLiquidityEvent(
            UniswapV4ModifyLiquidityEvent {
                pool_manager_address: log.address,
                event_id: id,
                sender,
                tick_lower,
                tick_upper,
                liquidity_delta,
                salt,
                log_index,
            },
        )))
    }

    // Uniswap V4 Donate decoder
    pub(super) fn decode_uniswap_v4_donate(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount0_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?;
        let amount0 = i128::from_be_bytes(
            amount0_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?,
        );

        let amount1_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?;
        let amount1 = i128::from_be_bytes(
            amount1_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4DonateEvent(
            UniswapV4DonateEvent {
                pool_manager_address: log.address,
                event_id: id,
                sender,
                amount0: U256::from(amount0 as u128),
                amount1: U256::from(amount1 as u128),
                log_index,
            },
        )))
    }

    // Uniswap V4 ProtocolFeeUpdated decoder
    pub(super) fn decode_uniswap_v4_protocol_fee_updated(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let id = log.topics()[1];

        let protocol_fee_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for protocol_fee"))?;
        let protocol_fee = u32::from_be_bytes(
            protocol_fee_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert protocol_fee bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4FeeUpdatedEvent(
            UniswapV4FeeUpdatedEvent {
                pool_manager_address: log.address,
                event_id: id,
                protocol_fee,
                log_index,
            },
        )))
    }

    // Uniswap V4 DynamicLPFeeUpdated decoder
    pub(super) fn decode_uniswap_v4_dynamic_lp_fee_updated(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let id = log.topics()[1];

        let dynamic_lp_fee_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for dynamic_lp_fee"))?;
        let dynamic_lp_fee = u32::from_be_bytes(
            dynamic_lp_fee_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert dynamic_lp_fee bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4DynamicLPFeeUpdatedEvent(
            UniswapV4DynamicLPFeeUpdatedEvent {
                pool_manager_address: log.address,
                event_id: id,
                dynamic_lp_fee,
                log_index,
            },
        )))
    }

    // Uniswap V4 ProtocolFeeControllerUpdated decoder
    pub(super) fn decode_uniswap_v4_protocol_fee_controller_updated(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let protocol_fee_controller_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for protocol_fee_controller"))?;
        let protocol_fee_controller = Address::from_slice(&protocol_fee_controller_bytes[12..32]);

        Ok(Some(DecodedEvent::UniswapV4FeeControllerUpdatedEvent(
            UniswapV4FeeControllerUpdatedEvent {
                pool_manager_address: log.address,
                protocol_fee_controller,
                log_index,
            },
        )))
    }

    // Uniswap V4 BalanceDelta decoder
    pub(super) fn decode_uniswap_v4_balance_delta(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let pool_id = log.topics()[1];
        let settler_bytes: &[u8] = log.topics()[2].as_ref();

        if settler_bytes.len() < 32 {
            return Ok(None);
        }

        let settler = Address::from_slice(&settler_bytes[12..32]);

        let delta0_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for delta0"))?;
        let delta0 = i128::from_be_bytes(
            delta0_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert delta0 bytes"))?,
        );

        let delta1_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for delta1"))?;
        let delta1 = i128::from_be_bytes(
            delta1_bytes[16..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert delta1 bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV4BalanceDeltaEvent(
            UniswapV4BalanceDeltaEvent {
                pool_manager_address: log.address,
                pool_id,
                settler,
                delta0,
                delta1,
                log_index,
            },
        )))
    }
}
