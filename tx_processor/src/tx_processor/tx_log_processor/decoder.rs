use super::{DecodedEvent, EventSignatures};
use alloy_primitives::Log as AlloyLog;
use eyre::Result;

pub struct LogDecoder {
    pub(super) signatures: EventSignatures,
}

impl LogDecoder {
    pub fn new() -> Self {
        Self {
            signatures: EventSignatures::new(),
        }
    }

    /// Decode a log into known event types
    pub fn decode_log(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().is_empty() {
            return Ok(None);
        }

        let event_signature = log.topics()[0];

        // ERC20/ERC721 Transfer (same signature, differentiated by topic count)
        if event_signature == self.signatures.transfer && log.topics().len() == 3 {
            return self.decode_erc20_transfer(log, log_index);
        }
        if event_signature == self.signatures.transfer_erc721 && log.topics().len() == 4 {
            return self.decode_erc721_transfer(log, log_index);
        }

        // ERC20/ERC721 Approval (same signature, differentiated by topic count)
        if event_signature == self.signatures.approval && log.topics().len() == 3 {
            return self.decode_erc20_approval(log, log_index);
        }
        if event_signature == self.signatures.approval_erc721 && log.topics().len() == 4 {
            return self.decode_erc721_approval(log, log_index);
        }
        if event_signature == self.signatures.approval_for_all && log.topics().len() == 3 {
            return self.decode_approval_for_all(log, log_index);
        }

        // ERC1155 TransferSingle
        if event_signature == self.signatures.transfer_single && log.topics().len() == 4 {
            return self.decode_erc1155_transfer(log, log_index);
        }
        if event_signature == self.signatures.transfer_batch && log.topics().len() == 4 {
            return self.decode_erc1155_transfer_batch(log, log_index);
        }

        // Uniswap V2 Swap
        if event_signature == self.signatures.swap && log.topics().len() == 3 {
            return self.decode_uniswap_v2_swap(log, log_index);
        }

        // Uniswap V2 Sync
        if event_signature == self.signatures.sync && log.topics().len() == 1 {
            return self.decode_uniswap_v2_sync(log, log_index);
        }

        // Uniswap V2 Mint
        if event_signature == self.signatures.mint && log.topics().len() == 2 {
            return self.decode_uniswap_v2_mint(log, log_index);
        }

        // Uniswap V2 Burn
        if event_signature == self.signatures.burn && log.topics().len() == 3 {
            return self.decode_uniswap_v2_burn(log, log_index);
        }

        // Uniswap V2 PairCreated
        if event_signature == self.signatures.pair_created && log.topics().len() == 4 {
            return self.decode_uniswap_v2_pair_created(log, log_index);
        }

        // Uniswap V3 Events
        if event_signature == self.signatures.swap_v3 && log.topics().len() == 3 {
            return self.decode_uniswap_v3_swap(log, log_index);
        }
        if event_signature == self.signatures.mint_v3 && log.topics().len() == 4 {
            return self.decode_uniswap_v3_mint(log, log_index);
        }
        if event_signature == self.signatures.burn_v3 && log.topics().len() == 4 {
            return self.decode_uniswap_v3_burn(log, log_index);
        }
        if event_signature == self.signatures.pool_created && log.topics().len() == 4 {
            return self.decode_uniswap_v3_pool_created(log, log_index);
        }
        if event_signature == self.signatures.initialize && log.topics().len() == 1 {
            return self.decode_uniswap_v3_initialize(log, log_index);
        }

        // Uniswap V4 Events
        if event_signature == self.signatures.swap_v4 && log.topics().len() == 3 {
            return self.decode_uniswap_v4_swap(log, log_index);
        }
        if event_signature == self.signatures.initialize_v4 && log.topics().len() == 4 {
            return self.decode_uniswap_v4_initialize(log, log_index);
        }
        if event_signature == self.signatures.modify_liquidity && log.topics().len() == 3 {
            return self.decode_uniswap_v4_modify_liquidity(log, log_index);
        }
        if event_signature == self.signatures.donate && log.topics().len() == 3 {
            return self.decode_uniswap_v4_donate(log, log_index);
        }
        if event_signature == self.signatures.protocol_fee_updated && log.topics().len() == 2 {
            return self.decode_uniswap_v4_protocol_fee_updated(log, log_index);
        }
        if event_signature == self.signatures.dynamic_lp_fee_updated && log.topics().len() == 2 {
            return self.decode_uniswap_v4_dynamic_lp_fee_updated(log, log_index);
        }
        if event_signature == self.signatures.protocol_fee_controller_updated
            && log.topics().len() == 1
        {
            return self.decode_uniswap_v4_protocol_fee_controller_updated(log, log_index);
        }
        if event_signature == self.signatures.balance_delta && log.topics().len() == 3 {
            return self.decode_uniswap_v4_balance_delta(log, log_index);
        }

        // Uniswap V3 missing events
        if event_signature == self.signatures.increase_liquidity && log.topics().len() == 2 {
            return self.decode_uniswap_v3_increase_liquidity(log, log_index);
        }
        if event_signature == self.signatures.decrease_liquidity && log.topics().len() == 2 {
            return self.decode_uniswap_v3_decrease_liquidity(log, log_index);
        }
        if event_signature == self.signatures.collect && log.topics().len() == 3 {
            return self.decode_uniswap_v3_collect(log, log_index);
        }
        // Check if this could be a position event (with more topics for extended data)
        if event_signature == self.signatures.increase_liquidity && log.topics().len() > 2 {
            return self.decode_uniswap_v3_position(log, log_index);
        }

        // General events
        if event_signature == self.signatures.deposit && log.topics().len() == 2 {
            return self.decode_deposit(log, log_index);
        }
        if event_signature == self.signatures.withdraw && log.topics().len() == 2 {
            return self.decode_withdraw(log, log_index);
        }
        if event_signature == self.signatures.ownership_transferred && log.topics().len() == 3 {
            return self.decode_ownership_transferred(log, log_index);
        }
        if event_signature == self.signatures.ownership_transfer_started && log.topics().len() == 3
        {
            return self.decode_ownership_transfer_started(log, log_index);
        }
        if event_signature == self.signatures.role_granted && log.topics().len() == 4 {
            return self.decode_access_control_role_granted(log, log_index);
        }
        if event_signature == self.signatures.role_revoked && log.topics().len() == 4 {
            return self.decode_access_control_role_revoked(log, log_index);
        }
        if event_signature == self.signatures.admin_changed && log.topics().len() == 3 {
            return self.decode_proxy_admin_changed(log, log_index);
        }
        if event_signature == self.signatures.trading_enabled && log.topics().len() == 1 {
            return self.decode_trading_enabled(log, log_index);
        }
        if event_signature == self.signatures.trading_disabled && log.topics().len() == 1 {
            return self.decode_trading_disabled(log, log_index);
        }
        if event_signature == self.signatures.permit2 && log.topics().len() == 4 {
            return self.decode_permit2(log, log_index);
        }

        Ok(None)
    }

    /// Extract logs from CallFrame recursively
    ///
    /// This method replaces the functionality from tx_simulator's trace_extraction.rs
    /// Moving log extraction to tx_processor where it belongs - tx_simulator should only simulate,
    /// tx_processor should handle result processing including log extraction.
    pub fn extract_logs_from_call_frame(&self, frame: &tx_simulator::CallFrame) -> Vec<AlloyLog> {
        let mut logs = Vec::new();
        self.extract_logs_recursive(frame, &mut logs);
        logs
    }

    /// Recursively extract logs from call frame and its children
    fn extract_logs_recursive(&self, frame: &tx_simulator::CallFrame, logs: &mut Vec<AlloyLog>) {
        // Add logs from this frame
        for log in &frame.logs {
            if let (Some(address), Some(topics)) = (&log.address, &log.topics) {
                logs.push(AlloyLog::new_unchecked(
                    *address,
                    topics.clone(),
                    log.data.clone().unwrap_or_default(),
                ));
            }
        }

        // Process child calls
        for child in &frame.calls {
            self.extract_logs_recursive(child, logs);
        }
    }
}
