use super::super::data_models::receipt_models::*;
use super::{DecodedEvent, LogDecoder};
use alloy_primitives::{Address, Log as AlloyLog, U256};
use eyre::Result;

impl LogDecoder {
    pub(super) fn decode_deposit(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        // Handle two types of deposits:
        // 1. Complex deposit: Deposit(uint256 id, address indexed tokenAddress, address indexed withdrawalAddress, uint256 amount, uint256 unlockTime)
        // 2. WETH deposit: Deposit(address indexed dst, uint256 wad)

        // Check if this is a complex deposit (with id, token, withdrawal - has more topics)
        if log.topics().len() > 2 {
            // Complex deposit has 3 topics (signature + 2 indexed addresses)
            // Data contains: id, amount, unlockTime
            if log.data.data.len() < 96 {
                return Ok(None);
            }

            let token_bytes: &[u8] = log.topics()[1].as_ref();
            let withdrawal_bytes: &[u8] = log.topics()[2].as_ref();

            if token_bytes.len() < 32 || withdrawal_bytes.len() < 32 {
                return Ok(None);
            }

            let token_address = Address::from_slice(&token_bytes[12..32]);
            let withdrawal_address = Address::from_slice(&withdrawal_bytes[12..32]);

            let id = u64::try_from(U256::from_be_slice(
                log.data
                    .data
                    .get(0..32)
                    .ok_or_else(|| eyre::eyre!("Invalid data length for id"))?,
            ))
            .unwrap_or(0);
            let amount = U256::from_be_slice(
                log.data
                    .data
                    .get(32..64)
                    .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?,
            );
            let unlock_time = u64::try_from(U256::from_be_slice(
                log.data
                    .data
                    .get(64..96)
                    .ok_or_else(|| eyre::eyre!("Invalid data length for unlock_time"))?,
            ))
            .unwrap_or(0);

            Ok(Some(DecodedEvent::DepositEvent(DepositEvent {
                id: Some(id),
                token_address: Some(token_address),
                withdrawal_address: Some(withdrawal_address),
                amount: Some(amount),
                unlock_time: Some(unlock_time),
                pair_address: None,
                sender: None,
                log_index: Some(log_index),
            })))
        } else if log.topics().len() == 2 && log.data.data.len() == 32 {
            // Simple deposit (like WETH) - data contains only amount
            let sender_bytes: &[u8] = log.topics()[1].as_ref();
            if sender_bytes.len() < 32 {
                return Ok(None);
            }

            let sender = Address::from_slice(&sender_bytes[12..32]);
            let amount = U256::from_be_slice(&log.data.data);

            Ok(Some(DecodedEvent::DepositEvent(DepositEvent {
                id: None,
                token_address: None,
                withdrawal_address: None,
                amount: Some(amount),
                unlock_time: None,
                pair_address: Some(log.address),
                sender: Some(sender),
                log_index: Some(log_index),
            })))
        } else {
            Ok(None)
        }
    }

    // Withdraw decoder
    pub(super) fn decode_withdraw(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);

        Ok(Some(DecodedEvent::WithdrawEvent(WithdrawEvent {
            pair_address: log.address,
            sender: Some(sender),
            amount,
            log_index,
        })))
    }

    // OwnershipTransferred decoder
    pub(super) fn decode_ownership_transferred(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 {
            return Ok(None);
        }

        let previous_owner_bytes: &[u8] = log.topics()[1].as_ref();
        let new_owner_bytes: &[u8] = log.topics()[2].as_ref();

        if previous_owner_bytes.len() < 32 || new_owner_bytes.len() < 32 {
            return Ok(None);
        }

        let previous_owner = Address::from_slice(&previous_owner_bytes[12..32]);
        let new_owner = Address::from_slice(&new_owner_bytes[12..32]);

        Ok(Some(DecodedEvent::OwnershipTransferredEvent(
            OwnershipTransferredEvent {
                contract_address: log.address,
                previous_owner,
                new_owner,
                log_index,
            },
        )))
    }

    pub(super) fn decode_ownership_transfer_started(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 {
            return Ok(None);
        }

        let previous_owner_bytes: &[u8] = log.topics()[1].as_ref();
        let new_owner_bytes: &[u8] = log.topics()[2].as_ref();

        if previous_owner_bytes.len() < 32 || new_owner_bytes.len() < 32 {
            return Ok(None);
        }

        let previous_owner = Address::from_slice(&previous_owner_bytes[12..32]);
        let new_owner = Address::from_slice(&new_owner_bytes[12..32]);

        Ok(Some(DecodedEvent::OwnershipTransferStartedEvent(
            OwnershipTransferStartedEvent {
                contract_address: log.address,
                previous_owner,
                new_owner,
                log_index,
            },
        )))
    }

    pub(super) fn decode_access_control_role_granted(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let account_bytes: &[u8] = log.topics()[2].as_ref();
        let sender_bytes: &[u8] = log.topics()[3].as_ref();
        if account_bytes.len() < 32 || sender_bytes.len() < 32 {
            return Ok(None);
        }

        let role = log.topics()[1];
        let account = Address::from_slice(&account_bytes[12..32]);
        let sender = Address::from_slice(&sender_bytes[12..32]);

        Ok(Some(DecodedEvent::AccessControlRoleGrantedEvent(
            AccessControlRoleGrantedEvent {
                contract_address: log.address,
                role,
                account,
                sender,
                log_index,
            },
        )))
    }

    pub(super) fn decode_access_control_role_revoked(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let account_bytes: &[u8] = log.topics()[2].as_ref();
        let sender_bytes: &[u8] = log.topics()[3].as_ref();
        if account_bytes.len() < 32 || sender_bytes.len() < 32 {
            return Ok(None);
        }

        let role = log.topics()[1];
        let account = Address::from_slice(&account_bytes[12..32]);
        let sender = Address::from_slice(&sender_bytes[12..32]);

        Ok(Some(DecodedEvent::AccessControlRoleRevokedEvent(
            AccessControlRoleRevokedEvent {
                contract_address: log.address,
                role,
                account,
                sender,
                log_index,
            },
        )))
    }

    pub(super) fn decode_proxy_admin_changed(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 {
            return Ok(None);
        }

        let previous_admin_bytes: &[u8] = log.topics()[1].as_ref();
        let new_admin_bytes: &[u8] = log.topics()[2].as_ref();
        if previous_admin_bytes.len() < 32 || new_admin_bytes.len() < 32 {
            return Ok(None);
        }

        let previous_admin = Address::from_slice(&previous_admin_bytes[12..32]);
        let new_admin = Address::from_slice(&new_admin_bytes[12..32]);

        Ok(Some(DecodedEvent::ProxyAdminChangedEvent(
            ProxyAdminChangedEvent {
                contract_address: log.address,
                previous_admin,
                new_admin,
                log_index,
            },
        )))
    }

    // TradingEnabled decoder
    pub(super) fn decode_trading_enabled(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 {
            return Ok(None);
        }

        Ok(Some(DecodedEvent::TradingEnabledEvent(
            TradingEnabledEvent {
                token_address: log.address,
                block_number: 0, // Will be set by the processor
                log_index,
            },
        )))
    }

    // TradingDisabled decoder
    pub(super) fn decode_trading_disabled(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 {
            return Ok(None);
        }

        Ok(Some(DecodedEvent::TradingDisabledEvent(
            TradingDisabledEvent {
                token_address: log.address,
                block_number: 0, // Will be set by the processor
                log_index,
            },
        )))
    }

    // Permit2Event decoder
    pub(super) fn decode_permit2(
        &self,
        log: &AlloyLog,
        log_index: u64,
    ) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let token_bytes: &[u8] = log.topics()[2].as_ref();
        let spender_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || token_bytes.len() < 32 || spender_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let token = Address::from_slice(&token_bytes[12..32]);
        let spender = Address::from_slice(&spender_bytes[12..32]);

        let amount = U256::from_be_slice(
            log.data
                .data
                .get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?,
        );

        let expiration_bytes = log
            .data
            .data
            .get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for expiration"))?;
        let expiration = u64::from_be_bytes(
            expiration_bytes[24..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert expiration bytes"))?,
        );

        let nonce_bytes = log
            .data
            .data
            .get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for nonce"))?;
        let nonce = u64::from_be_bytes(
            nonce_bytes[24..32]
                .try_into()
                .map_err(|_| eyre::eyre!("Failed to convert nonce bytes"))?,
        );

        Ok(Some(DecodedEvent::Permit2Event(Permit2Event {
            pool_manager_address: log.address,
            owner,
            token,
            spender,
            amount,
            expiration,
            nonce,
            log_index,
        })))
    }
}
