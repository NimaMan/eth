use tx_processor::tx_processor::data_models::InternalTransaction;
use tx_processor::ProcessedTransaction;

use super::model::{MovementSource, RawSourceKind, TxAsset, TxMovement};
use super::native::movement_from_internal_trace;

pub fn movements_from_processed_transaction(transaction: &ProcessedTransaction) -> Vec<TxMovement> {
    let mut movements = Vec::new();

    for (index, transfer) in transaction.eth_transfers.iter().enumerate() {
        if transfer.amount.is_zero() {
            continue;
        }
        movements.push(TxMovement::new(
            transfer.from_address,
            transfer.to_address,
            TxAsset::native_eth(),
            transfer.amount,
            MovementSource::new(RawSourceKind::ProcessedEthTransfer).with_row_index(index),
        ));
    }

    for (index, row) in transaction.internal_transactions.iter().enumerate() {
        if duplicates_processed_eth_transfer(transaction, row) {
            continue;
        }
        if let Some(movement) = movement_from_internal_trace(index, row) {
            movements.push(movement);
        }
    }

    for (index, transfer) in transaction.erc20_transfers.iter().enumerate() {
        if transfer.amount.is_zero() {
            continue;
        }
        movements.push(TxMovement::new(
            transfer.from_address,
            transfer.to_address,
            TxAsset::erc20(transfer.token_address),
            transfer.amount,
            MovementSource::new(RawSourceKind::Erc20TransferLog)
                .with_row_index(index)
                .with_log_index(Some(transfer.log_index)),
        ));
    }

    for (index, transfer) in transaction.internal_erc20_transfers.iter().enumerate() {
        if transfer.amount.is_zero() {
            continue;
        }
        movements.push(TxMovement::new(
            transfer.from_address,
            transfer.to_address,
            TxAsset::erc20(transfer.token_address),
            transfer.amount,
            MovementSource::new(RawSourceKind::InternalErc20Transfer).with_row_index(index),
        ));
    }

    movements
}

fn duplicates_processed_eth_transfer(
    transaction: &ProcessedTransaction,
    row: &InternalTransaction,
) -> bool {
    row.depth == 0
        && transaction.eth_transfers.iter().any(|transfer| {
            transfer.from_address == row.from_address
                && Some(transfer.to_address) == row.to_address
                && transfer.amount == row.value
        })
}
