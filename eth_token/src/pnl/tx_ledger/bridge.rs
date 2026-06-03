use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use super::model::{MovementSource, RawSourceKind, TxAsset, TxMovement, WETH_ADDRESS};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WethBridgeKind {
    Deposit,
    Withdraw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WethBridge {
    pub kind: WethBridgeKind,
    pub account: Address,
    pub amount: U256,
    pub weth_address: Address,
    pub log_index: Option<u64>,
}

impl WethBridge {
    pub fn wrapped_movement(&self) -> TxMovement {
        match self.kind {
            WethBridgeKind::Deposit => TxMovement::new(
                self.weth_address,
                self.account,
                TxAsset::erc20(self.weth_address),
                self.amount,
                MovementSource::new(RawSourceKind::WethDepositEvent).with_log_index(self.log_index),
            ),
            WethBridgeKind::Withdraw => TxMovement::new(
                self.account,
                self.weth_address,
                TxAsset::erc20(self.weth_address),
                self.amount,
                MovementSource::new(RawSourceKind::WethWithdrawEvent)
                    .with_log_index(self.log_index),
            ),
        }
    }
}

pub fn detect_weth_bridges(transaction: &ProcessedTransaction) -> Vec<WethBridge> {
    let mut bridges = Vec::new();

    for event in &transaction.deposit_events {
        let is_weth = event.pair_address == Some(WETH_ADDRESS);
        let Some(account) = event.sender else {
            continue;
        };
        let Some(amount) = event.amount else {
            continue;
        };
        if !is_weth || amount.is_zero() {
            continue;
        }
        bridges.push(WethBridge {
            kind: WethBridgeKind::Deposit,
            account,
            amount,
            weth_address: WETH_ADDRESS,
            log_index: event.log_index,
        });
    }

    for event in &transaction.withdraw_events {
        let Some(account) = event.sender else {
            continue;
        };
        if event.pair_address != WETH_ADDRESS || event.amount.is_zero() {
            continue;
        }
        bridges.push(WethBridge {
            kind: WethBridgeKind::Withdraw,
            account,
            amount: event.amount,
            weth_address: WETH_ADDRESS,
            log_index: Some(event.log_index),
        });
    }

    bridges
}

pub fn bridge_movements(bridges: &[WethBridge]) -> Vec<TxMovement> {
    bridges.iter().map(WethBridge::wrapped_movement).collect()
}
