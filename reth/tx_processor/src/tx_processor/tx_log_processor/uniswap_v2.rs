use super::super::data_models::receipt_models::*;
use super::{DecodedEvent, LogDecoder};
use alloy_primitives::{Address, Log as AlloyLog, U256};
use eyre::Result;

impl LogDecoder {
    pub(super) fn decode_uniswap_v2_swap(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 128 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 swap"));
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);
        let to = Address::from_slice(&to_bytes[12..32]);

        // Use get() for safe slicing with proper error handling
        let amount0_in = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0_in"))?,
        );
        let amount1_in = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1_in"))?,
        );
        let amount0_out = U256::from_be_slice(
            log.data
                .data
                .get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount0_out"))?,
        );
        let amount1_out = U256::from_be_slice(
            log.data
                .data
                .get(96..128)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount1_out"))?,
        );

        Ok(Some(DecodedEvent::UniswapV2SwapEvent(UniswapV2SwapEvent {
            pair_address: log.address,
            sender,
            to,
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            log_index,
        })))
    }

    pub(super) fn decode_uniswap_v2_sync(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 64 {
            return Ok(None);
        }

        // Use get() for safe slicing
        let reserve0 = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for reserve0"))?,
        );
        let reserve1 = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for reserve1"))?,
        );

        Ok(Some(DecodedEvent::UniswapV2SyncEvent(UniswapV2SyncEvent {
            pair_address: log.address,
            reserve0,
            reserve1,
            log_index,
        })))
    }

    pub(super) fn decode_uniswap_v2_mint(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();

        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 mint"));
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        // Use get() for safe slicing
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

        Ok(Some(DecodedEvent::UniswapV2MintEvent(UniswapV2MintEvent {
            pair_address: log.address,
            sender,
            amount0,
            amount1,
            log_index,
        })))
    }

    pub(super) fn decode_uniswap_v2_burn(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        // Uniswap V2 Burn event: Burn(address indexed sender, uint amount0, uint amount1, address indexed to)
        // Topics: [signature, sender, to]
        // Data: [amount0, amount1]
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 burn"));
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);
        // to address is indexed but not used in UniswapV2BurnEvent

        // Extract amounts from data
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

        Ok(Some(DecodedEvent::UniswapV2BurnEvent(UniswapV2BurnEvent {
            pair_address: log.address,
            sender,
            amount0,
            amount1,
            log_index,
        })))
    }

    pub(super) fn decode_uniswap_v2_pair_created(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let token0_bytes: &[u8] = log.topics()[1].as_ref();
        let token1_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction
        if token0_bytes.len() < 32 || token1_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for PairCreated"));
        }

        let token0 = Address::from_slice(&token0_bytes[12..32]);
        let token1 = Address::from_slice(&token1_bytes[12..32]);

        // Extract pair address from data
        let pair_bytes = log
            .data
            .data
            .get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for pair address"))?;
        let pair_address = Address::from_slice(&pair_bytes[12..32]);

        Ok(Some(DecodedEvent::UniswapV2PairCreatedEvent(
            UniswapV2PairCreatedEvent {
                pair_address,
                token0,
                token1,
                log_index,
            },
        )))
    }
}
