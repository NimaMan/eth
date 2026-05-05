use super::super::data_models::receipt_models::*;
use super::abi::{read_abi_offset, read_u256_array};
use super::{DecodedEvent, LogDecoder};
use alloy_primitives::{Address, Log as AlloyLog, U256};
use eyre::Result;

impl LogDecoder {
    pub(super) fn decode_erc20_transfer(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let from_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction (32 bytes in topic, address uses last 20)
        if from_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for ERC20 transfer"));
        }

        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);

        Ok(Some(DecodedEvent::ERC20TransferEvent(ERC20TransferEvent {
            token_address: log.address,
            from_address,
            to_address,
            amount,
            log_index,
        })))
    }

    pub(super) fn decode_erc20_approval(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let spender_bytes: &[u8] = log.topics()[2].as_ref();

        // Ensure we have enough bytes for address extraction
        if owner_bytes.len() < 32 || spender_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for ERC20 approval"));
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let spender = Address::from_slice(&spender_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);

        Ok(Some(DecodedEvent::ERC20ApprovalEvent(ERC20ApprovalEvent {
            token_address: log.address,
            owner,
            spender,
            amount,
            log_index,
        })))
    }

    // ERC721 Transfer decoder
    pub(super) fn decode_erc721_transfer(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let from_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        let token_id_bytes: &[u8] = log.topics()[3].as_ref();

        if from_bytes.len() < 32 || to_bytes.len() < 32 || token_id_bytes.len() < 32 {
            return Ok(None);
        }

        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);
        let token_id = U256::from_be_slice(token_id_bytes);

        Ok(Some(DecodedEvent::ERC721TransferEvent(
            ERC721TransferEvent {
                token_address: log.address,
                from_address,
                to_address,
                token_id,
                log_index,
            },
        )))
    }

    // ERC721 Approval decoder
    pub(super) fn decode_erc721_approval(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let approved_bytes: &[u8] = log.topics()[2].as_ref();
        let token_id_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || approved_bytes.len() < 32 || token_id_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let approved = Address::from_slice(&approved_bytes[12..32]);
        let token_id = U256::from_be_slice(token_id_bytes);

        Ok(Some(DecodedEvent::ERC721ApprovalEvent(
            ERC721ApprovalEvent {
                token_address: log.address,
                owner,
                approved_address: approved,
                token_id,
                log_index,
            },
        )))
    }

    // ERC721/ERC1155 ApprovalForAll decoder
    pub(super) fn decode_approval_for_all(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let operator_bytes: &[u8] = log.topics()[2].as_ref();

        if owner_bytes.len() < 32 || operator_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let operator = Address::from_slice(&operator_bytes[12..32]);
        let approved = log.data.data[31] != 0;

        Ok(Some(DecodedEvent::ApprovalForAllEvent(
            ApprovalForAllEvent {
                token_address: log.address,
                owner,
                operator,
                approved,
                log_index,
            },
        )))
    }

    // ERC1155 TransferSingle decoder
    pub(super) fn decode_erc1155_transfer(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let operator_bytes: &[u8] = log.topics()[1].as_ref();
        let from_bytes: &[u8] = log.topics()[2].as_ref();
        let to_bytes: &[u8] = log.topics()[3].as_ref();

        if operator_bytes.len() < 32 || from_bytes.len() < 32 || to_bytes.len() < 32 {
            return Ok(None);
        }

        let operator = Address::from_slice(&operator_bytes[12..32]);
        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);

        let token_id = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for token_id"))?,
        );
        let amount = U256::from_be_slice(
            log.data
                .data
                .get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?,
        );

        Ok(Some(DecodedEvent::ERC1155TransferEvent(
            ERC1155TransferEvent {
                token_address: log.address,
                operator,
                from_address,
                to_address,
                token_ids: vec![token_id],
                amounts: vec![amount],
                log_index,
            },
        )))
    }

    // ERC1155 TransferBatch decoder
    pub(super) fn decode_erc1155_transfer_batch(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() < 128 {
            return Ok(None);
        }

        let operator_bytes: &[u8] = log.topics()[1].as_ref();
        let from_bytes: &[u8] = log.topics()[2].as_ref();
        let to_bytes: &[u8] = log.topics()[3].as_ref();

        if operator_bytes.len() < 32 || from_bytes.len() < 32 || to_bytes.len() < 32 {
            return Ok(None);
        }

        let operator = Address::from_slice(&operator_bytes[12..32]);
        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);

        let data = log.data.data.as_ref();
        let ids_offset = match read_abi_offset(data, 0) {
            Some(offset) => offset,
            None => return Ok(None),
        };
        let amounts_offset = match read_abi_offset(data, 32) {
            Some(offset) => offset,
            None => return Ok(None),
        };

        let token_ids = match read_u256_array(data, ids_offset) {
            Some(values) => values,
            None => return Ok(None),
        };
        let amounts = match read_u256_array(data, amounts_offset) {
            Some(values) => values,
            None => return Ok(None),
        };

        if token_ids.len() != amounts.len() {
            return Ok(None);
        }

        Ok(Some(DecodedEvent::ERC1155TransferEvent(
            ERC1155TransferEvent {
                token_address: log.address,
                operator,
                from_address,
                to_address,
                token_ids,
                amounts,
                log_index,
            },
        )))
    }
}
