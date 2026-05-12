use super::super::data_models::receipt_models::*;
use super::{DecodedEvent, LogDecoder};
use alloy_primitives::{Address, Log as AlloyLog, U256};
use eyre::Result;

impl LogDecoder {
    pub(super) fn decode_uniswap_v3_swap(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 160 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let recipient_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || recipient_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V3 swap"));
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);
        let recipient = Address::from_slice(&recipient_bytes[12..32]);

        // Decode signed integers from data with safe slicing
        let amount0_bytes = log
            .data
            .data
            .get(16..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?;
        let amount0 = i128::from_be_bytes(amount0_bytes);

        let amount1_bytes = log
            .data
            .data
            .get(48..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?;
        let amount1 = i128::from_be_bytes(amount1_bytes);

        let sqrt_price_x96 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?,
        );

        let liquidity_bytes = log
            .data
            .data
            .get(112..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert liquidity bytes"))?;
        let liquidity = u128::from_be_bytes(liquidity_bytes);

        let tick_bytes = log
            .data
            .data
            .get(156..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?;
        let tick = i32::from_be_bytes(tick_bytes);

        Ok(Some(DecodedEvent::UniswapV3SwapEvent(UniswapV3SwapEvent {
            pool_address: log.address,
            sender,
            recipient,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
            log_index,
        })))
    }

    // Uniswap V3 Mint decoder
    pub(super) fn decode_uniswap_v3_mint(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 128 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let tick_lower_bytes: &[u8] = log.topics()[2].as_ref();
        let tick_upper_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || tick_lower_bytes.len() < 32 || tick_upper_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?,
        );
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?,
        );

        let sender_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sender"))?;
        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?;
        let amount = U256::from_be_slice(amount_bytes);

        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(96..128)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3MintEvent(UniswapV3MintEvent {
            pool_address: log.address,
            sender,
            owner,
            tick_lower,
            tick_upper,
            amount,
            amount0,
            amount1,
            log_index,
        })))
    }

    // Uniswap V3 Burn decoder
    pub(super) fn decode_uniswap_v3_burn(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let tick_lower_bytes: &[u8] = log.topics()[2].as_ref();
        let tick_upper_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || tick_lower_bytes.len() < 32 || tick_upper_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?,
        );
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?,
        );

        let amount_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?;
        let amount = U256::from_be_slice(amount_bytes);

        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3BurnEvent(UniswapV3BurnEvent {
            pool_address: log.address,
            owner,
            tick_lower,
            tick_upper,
            amount,
            amount0,
            amount1,
            log_index,
        })))
    }

    // Uniswap V3 PoolCreated decoder
    pub(super) fn decode_uniswap_v3_pool_created(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let token0_bytes: &[u8] = log.topics()[1].as_ref();
        let token1_bytes: &[u8] = log.topics()[2].as_ref();
        let fee_bytes: &[u8] = log.topics()[3].as_ref();

        if token0_bytes.len() < 32 || token1_bytes.len() < 32 || fee_bytes.len() < 32 {
            return Ok(None);
        }

        let token0 = Address::from_slice(&token0_bytes[12..32]);
        let token1 = Address::from_slice(&token1_bytes[12..32]);
        let fee = u32::from_be_bytes(
            fee_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?,
        );

        let tick_spacing_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_spacing"))?;
        let tick_spacing = i32::from_be_bytes(
            tick_spacing_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_spacing bytes"))?,
        );

        let pool_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for pool"))?;
        let pool = Address::from_slice(&pool_bytes[12..32]);

        Ok(Some(DecodedEvent::UniswapV3PoolCreatedEvent(
            UniswapV3PoolCreatedEvent {
                factory_address: log.address,
                token0,
                token1,
                fee,
                tick_spacing,
                pool,
                log_index,
            },
        )))
    }

    // Uniswap V3 Initialize decoder
    pub(super) fn decode_uniswap_v3_initialize(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let sqrt_price_x96_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?;
        let sqrt_price_x96 = U256::from_be_slice(sqrt_price_x96_bytes);

        let tick_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3InitializeEvent(
            UniswapV3InitializeEvent {
                pool_address: log.address,
                sqrt_price_x96,
                tick,
                log_index,
            },
        )))
    }

    // Uniswap V3 Position decoder (similar to IncreaseLiquidity but with more fields)
    pub(super) fn decode_uniswap_v3_position(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        // This handles position events that have extended topics for owner/tick information
        // Based on Python's parse_uniswap_v3_position logic
        if log.topics().len() < 2 || log.data.data.len() < 96 {
            return Ok(None);
        }

        let token_id_bytes: &[u8] = log.topics()[1].as_ref();
        let token_id = U256::from_be_slice(token_id_bytes);

        // Extract owner from topic 2 if present
        let owner = if log.topics().len() > 2 {
            let owner_bytes: &[u8] = log.topics()[2].as_ref();
            if owner_bytes.len() >= 32 {
                Some(Address::from_slice(&owner_bytes[12..32]))
            } else {
                None
            }
        } else {
            None
        };

        // Extract tick bounds from topics 3 and 4 if present
        let tick_lower = if log.topics().len() > 3 {
            let tick_bytes: &[u8] = log.topics()[3].as_ref();
            if tick_bytes.len() >= 32 {
                i32::from_be_bytes(tick_bytes[28..32].try_into().unwrap_or([0, 0, 0, 0]))
            } else {
                0
            }
        } else {
            0
        };

        let tick_upper = if log.topics().len() > 4 {
            let tick_bytes: &[u8] = log.topics()[4].as_ref();
            if tick_bytes.len() >= 32 {
                i32::from_be_bytes(tick_bytes[28..32].try_into().unwrap_or([0, 0, 0, 0]))
            } else {
                0
            }
        } else {
            0
        };

        // Parse data fields
        let liquidity = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?,
        );
        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3PositionEvent(
            UniswapV3PositionEvent {
                token_id,
                liquidity,
                amount0,
                amount1,
                pool_address: log.address,
                owner: owner.unwrap_or(Address::ZERO),
                tick_lower,
                tick_upper,
                log_index,
            },
        )))
    }

    pub(super) fn decode_uniswap_v3_increase_liquidity(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());

        let liquidity_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = U256::from_be_slice(liquidity_bytes);

        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3IncreaseLiquidityEvent(
            UniswapV3IncreaseLiquidityEvent {
                token_id,
                liquidity,
                amount0,
                amount1,
                pool_address: log.address,
                log_index,
            },
        )))
    }

    // Uniswap V3 DecreaseLiquidity decoder
    pub(super) fn decode_uniswap_v3_decrease_liquidity(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());

        let liquidity_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = U256::from_be_slice(liquidity_bytes);

        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3DecreaseLiquidityEvent(
            UniswapV3DecreaseLiquidityEvent {
                token_id,
                liquidity,
                amount0,
                amount1,
                pool_address: log.address,
                log_index,
            },
        )))
    }

    // Uniswap V3 Collect decoder
    pub(super) fn decode_uniswap_v3_collect(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());
        let recipient_bytes: &[u8] = log.topics()[2].as_ref();

        if recipient_bytes.len() < 32 {
            return Ok(None);
        }

        let recipient = Address::from_slice(&recipient_bytes[12..32]);

        let amount0 = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?,
        );
        let amount1 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV3CollectEvent(
            UniswapV3CollectEvent {
                token_id,
                recipient,
                amount0,
                amount1,
                pool_address: log.address,
                log_index,
            },
        )))
    }
}
