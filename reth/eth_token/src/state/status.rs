use alloy_primitives::{B256, U256};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

pub const DEFAULT_HIDDEN_MINT_THRESHOLD: f64 = 1.01;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenStatusManager {
    pub total_supply: String,
    pub hidden_mint_threshold: f64,
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<u64>,
    pub trading_enabled_timestamp: Option<u64>,
    pub trading_enabled_tx: Option<String>,
    pub trading_enabled_event_index: Option<u64>,
    pub trading_enabled_event_log_index: Option<u64>,
    pub trading_disabled_block: Option<u64>,
    pub trading_disabled_tx: Option<String>,
    pub trading_disabled_event_index: Option<u64>,
    pub trading_disabled_event_log_index: Option<u64>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_block: Option<u64>,
    pub scam_tx: Option<String>,
}

impl TokenStatusManager {
    pub fn new(total_supply: impl Into<String>) -> Self {
        Self {
            total_supply: total_supply.into(),
            hidden_mint_threshold: DEFAULT_HIDDEN_MINT_THRESHOLD,
            trading_enabled: false,
            trading_enabled_block: None,
            trading_enabled_timestamp: None,
            trading_enabled_tx: None,
            trading_enabled_event_index: None,
            trading_enabled_event_log_index: None,
            trading_disabled_block: None,
            trading_disabled_tx: None,
            trading_disabled_event_index: None,
            trading_disabled_event_log_index: None,
            is_scam: false,
            scam_label: None,
            scam_block: None,
            scam_tx: None,
        }
    }

    pub fn update_from_processed_transaction(
        &mut self,
        tx: &ProcessedTransaction,
        total_supply_from_transfers: f64,
        token_decimals: u8,
    ) -> Result<()> {
        self.process_trading_events(tx);
        self.detect_hidden_mint(tx, total_supply_from_transfers, token_decimals)?;
        Ok(())
    }

    pub fn mark_scam(
        &mut self,
        label: impl Into<String>,
        block_number: Option<u64>,
        tx_hash: Option<String>,
    ) {
        let label = label.into();
        if self.is_scam && self.scam_label.as_deref() == Some(label.as_str()) {
            return;
        }
        self.is_scam = true;
        self.scam_label = Some(label);
        self.scam_block = block_number;
        self.scam_tx = tx_hash;
    }

    pub fn clear_scam_flag(&mut self) {
        self.is_scam = false;
        self.scam_label = None;
        self.scam_block = None;
        self.scam_tx = None;
    }

    fn process_trading_events(&mut self, tx: &ProcessedTransaction) {
        let tx_hash = hash_string(&tx.hash);
        if let Some(event) = tx.trading_enabled_events.last() {
            if !self.trading_enabled {
                self.trading_enabled = true;
                self.trading_enabled_block = Some(tx.block_number);
                self.trading_enabled_timestamp = Some(tx.block_timestamp);
                self.trading_enabled_tx = Some(tx_hash.clone());
                self.trading_enabled_event_index = Some(tx.tx_index);
                self.trading_enabled_event_log_index = Some(event.log_index);
            }
        }

        if let Some(event) = tx.trading_disabled_events.last() {
            self.trading_enabled = false;
            self.trading_disabled_block = Some(tx.block_number);
            self.trading_disabled_tx = Some(tx_hash);
            self.trading_disabled_event_index = Some(tx.tx_index);
            self.trading_disabled_event_log_index = Some(event.log_index);
        }
    }

    fn detect_hidden_mint(
        &mut self,
        tx: &ProcessedTransaction,
        total_supply_from_transfers: f64,
        token_decimals: u8,
    ) -> Result<bool> {
        let total_supply = scaled_supply(&self.total_supply, token_decimals)?;
        if total_supply > 0.0
            && total_supply_from_transfers > total_supply * self.hidden_mint_threshold
        {
            self.mark_scam(
                "hidden_mint",
                Some(tx.block_number),
                Some(hash_string(&tx.hash)),
            );
            return Ok(true);
        }
        Ok(false)
    }
}

fn scaled_supply(raw_supply: &str, decimals: u8) -> Result<f64> {
    let raw = if raw_supply.trim().starts_with("0x") {
        U256::from_str_radix(raw_supply.trim_start_matches("0x"), 16)?.to_string()
    } else {
        raw_supply.to_string()
    };
    Ok(raw.parse::<f64>()? / 10_f64.powi(i32::from(decimals)))
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use tx_processor::tx_processor::data_models::{TradingDisabledEvent, TradingEnabledEvent};

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            5,
            address!("1111111111111111111111111111111111111111"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn tracks_trading_enabled_and_disabled_events() {
        let mut enabled_tx = tx();
        enabled_tx.trading_enabled_events.push(TradingEnabledEvent {
            token_address: address!("2222222222222222222222222222222222222222"),
            block_number: 100,
            log_index: 7,
        });
        let mut status_manager = TokenStatusManager::new("100000000000000000000");

        status_manager
            .update_from_processed_transaction(&enabled_tx, 0.0, 18)
            .unwrap();

        assert!(status_manager.trading_enabled);
        assert_eq!(status_manager.trading_enabled_block, Some(100));
        assert_eq!(status_manager.trading_enabled_event_log_index, Some(7));

        let mut disabled = tx();
        disabled.trading_disabled_events.push(TradingDisabledEvent {
            token_address: address!("2222222222222222222222222222222222222222"),
            block_number: 101,
            log_index: 8,
        });
        status_manager
            .update_from_processed_transaction(&disabled, 0.0, 18)
            .unwrap();

        assert!(!status_manager.trading_enabled);
        assert_eq!(status_manager.trading_disabled_event_log_index, Some(8));
    }

    #[test]
    fn hidden_mint_marks_scam_when_transfer_mints_exceed_supply_threshold() {
        let tx = tx();
        let mut status_manager = TokenStatusManager::new("100000000000000000000");

        status_manager
            .update_from_processed_transaction(&tx, 102.0, 18)
            .unwrap();

        assert!(status_manager.is_scam);
        assert_eq!(status_manager.scam_label.as_deref(), Some("hidden_mint"));
        assert_eq!(status_manager.scam_block, Some(100));
    }
}
