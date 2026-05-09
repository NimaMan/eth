//! Transfer-derived network update extraction.

use alloy_primitives::Address;
use reth_chain_query::common_addresses::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS};
use tx_processor::ProcessedTransaction;

use crate::network::{
    activity::{AddressMovement, MovementAssetKind, MovementDirection},
    ingest::transaction::{
        address_string, edge_with_evidence, scale_u256, NetworkIngestBatch,
        TransactionIngestContext,
    },
    model::{NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkNodeId},
};

pub const ETH_SYMBOL: &str = "ETH";
pub const WETH_SYMBOL: &str = "WETH";

/// Extract token and denomination movements for the tracked token network.
pub fn extract_transfer_updates(
    tx: &ProcessedTransaction,
    tracked_token_address: impl AsRef<str>,
    token_decimals: u8,
) -> NetworkIngestBatch {
    let mut batch = NetworkIngestBatch::default();
    if !tx.status {
        return batch;
    }

    let context = TransactionIngestContext::from_transaction(tx);
    let tracked_token_address = tracked_token_address.as_ref().trim().to_ascii_lowercase();

    for transfer in &tx.erc20_transfers {
        let token_address = address_string(&transfer.token_address);
        if token_address == tracked_token_address {
            let amount = scale_u256(transfer.amount, token_decimals);
            if amount > 0.0 {
                push_address_pair_movements(
                    &mut batch,
                    &context,
                    &transfer.from_address,
                    &transfer.to_address,
                    MovementAssetKind::Token,
                    Some(&tracked_token_address),
                    None,
                    transfer.amount.to_string(),
                    amount,
                    Some(transfer.log_index),
                    NetworkEdgeKind::TokenTransfer,
                    NetworkEvidenceSource::TokenTransfer,
                );
            }
            continue;
        }

        if let Some(symbol) = DENOM_ADDRESSES.get(&transfer.token_address).copied() {
            let decimals = ERC20_TOKEN_DECIMALS.get(symbol).copied().unwrap_or(18);
            let amount = scale_u256(transfer.amount, decimals);
            if amount > 0.0 {
                push_address_pair_movements(
                    &mut batch,
                    &context,
                    &transfer.from_address,
                    &transfer.to_address,
                    denom_asset_kind(symbol),
                    Some(&token_address),
                    Some(symbol),
                    transfer.amount.to_string(),
                    amount,
                    Some(transfer.log_index),
                    NetworkEdgeKind::DenomTransfer,
                    NetworkEvidenceSource::DenomTransfer,
                );
            }
        }
    }

    for transfer in &tx.eth_transfers {
        let amount = scale_u256(transfer.amount, 18);
        if amount > 0.0 {
            push_address_pair_movements(
                &mut batch,
                &context,
                &transfer.from_address,
                &transfer.to_address,
                MovementAssetKind::Native,
                None,
                Some(ETH_SYMBOL),
                transfer.amount.to_string(),
                amount,
                None,
                NetworkEdgeKind::DenomTransfer,
                NetworkEvidenceSource::DenomTransfer,
            );
        }
    }

    for transfer in &tx.internal_transactions {
        let Some(to_address) = transfer.to_address else {
            continue;
        };
        if transfer.error.is_some() {
            continue;
        }
        let amount = scale_u256(transfer.value, 18);
        if amount > 0.0 {
            push_address_pair_movements(
                &mut batch,
                &context,
                &transfer.from_address,
                &to_address,
                MovementAssetKind::Native,
                None,
                Some(ETH_SYMBOL),
                transfer.value.to_string(),
                amount,
                None,
                NetworkEdgeKind::DenomTransfer,
                NetworkEvidenceSource::DenomTransfer,
            );
        }
    }

    batch
}

#[allow(clippy::too_many_arguments)]
fn push_address_pair_movements(
    batch: &mut NetworkIngestBatch,
    context: &TransactionIngestContext,
    from_address: &Address,
    to_address: &Address,
    asset_kind: MovementAssetKind,
    asset: Option<&str>,
    symbol: Option<&str>,
    raw_amount: String,
    amount: f64,
    log_index: Option<u64>,
    edge_kind: NetworkEdgeKind,
    evidence_source: NetworkEvidenceSource,
) {
    let from = address_string(from_address);
    let to = address_string(to_address);
    let observation = context.observation(log_index);

    let mut out_movement = AddressMovement::new(
        MovementDirection::Out,
        asset_kind.clone(),
        amount,
        observation.clone(),
    )
    .with_raw_amount(raw_amount.clone());
    let mut in_movement = AddressMovement::new(
        MovementDirection::In,
        asset_kind,
        amount,
        observation.clone(),
    )
    .with_raw_amount(raw_amount.clone());

    if let Some(asset) = asset {
        out_movement = out_movement.with_asset(asset);
        in_movement = in_movement.with_asset(asset);
    }
    if let Some(symbol) = symbol {
        out_movement = out_movement.with_symbol(symbol);
        in_movement = in_movement.with_symbol(symbol);
    }

    batch.push_movement(&from, out_movement);
    batch.push_movement(&to, in_movement);

    let mut evidence = NetworkEvidence::new(evidence_source)
        .with_observation(observation)
        .with_description("address-to-address value movement");
    evidence
        .attributes
        .insert("amount".to_string(), amount.to_string());
    evidence
        .attributes
        .insert("raw_amount".to_string(), raw_amount);
    if let Some(symbol) = symbol {
        evidence
            .attributes
            .insert("symbol".to_string(), symbol.to_string());
    }

    let edge = edge_with_evidence(
        NetworkNodeId::address(from),
        NetworkNodeId::address(to),
        edge_kind,
        evidence,
    );
    batch.push_edge(edge);
}

fn denom_asset_kind(symbol: &str) -> MovementAssetKind {
    match symbol {
        WETH_SYMBOL => MovementAssetKind::WrappedNative,
        "USDC" | "USDT" | "DAI" | "USDe" | "sUSDe" | "USDS" => MovementAssetKind::Stable,
        _ => MovementAssetKind::KnownDenom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, U256};
    use tx_processor::tx_processor::data_models::{tx_models::ETHTransfer, ERC20TransferEvent};

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            address!("1111111111111111111111111111111111111111"),
            None,
            U256::ZERO,
            true,
            7,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn transfer_ingest_extracts_tracked_token_movements() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut tx = tx();
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: token,
            from_address: address!("1111111111111111111111111111111111111111"),
            to_address: address!("2222222222222222222222222222222222222222"),
            amount: U256::from(1_500_000_u64),
            log_index: 9,
        });

        let batch = extract_transfer_updates(&tx, address_string(&token), 6);

        assert_eq!(batch.address_movements.len(), 2);
        assert_eq!(batch.address_movements[0].movement.amount, 1.5);
        assert_eq!(
            batch.address_movements[0].movement.direction,
            MovementDirection::Out
        );
        assert_eq!(
            batch.address_movements[1].movement.direction,
            MovementDirection::In
        );
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, NetworkEdgeKind::TokenTransfer);
    }

    #[test]
    fn transfer_ingest_extracts_native_eth_movements() {
        let mut tx = tx();
        tx.eth_transfers.push(ETHTransfer {
            from_address: address!("1111111111111111111111111111111111111111"),
            to_address: address!("2222222222222222222222222222222222222222"),
            amount: U256::from(2_000_000_000_000_000_000_u128),
        });

        let batch = extract_transfer_updates(&tx, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 18);

        assert_eq!(batch.address_movements.len(), 2);
        assert_eq!(batch.address_movements[0].movement.amount, 2.0);
        assert_eq!(
            batch.address_movements[0].movement.asset_kind,
            MovementAssetKind::Native
        );
        assert_eq!(batch.edges[0].kind, NetworkEdgeKind::DenomTransfer);
    }
}
