//! Integration with tx_processor for converting ProcessedTransaction to fund flows

use alloy_primitives::{Address, U256};
use eyre::Result;
use std::str::FromStr;
use tx_fund_flow_core_types::{CompleteFundFlows, EthMovement, EthMovementType, TokenMovement};
use tx_processor::tx_processor::data_models::ProcessedTransaction;

/// Convert a ProcessedTransaction into CompleteFundFlows for analysis
pub fn extract_fund_flows_from_processed_tx(
    tx: &ProcessedTransaction,
) -> Result<CompleteFundFlows> {
    let mut eth_movements = Vec::new();
    let mut token_movements = Vec::new();

    // 1. Main ETH transfer (if any)
    if tx.value > U256::ZERO {
        eth_movements.push(EthMovement {
            from: tx.from_address,
            to: tx.to_address.unwrap_or(Address::ZERO),
            amount: tx.value,
            movement_type: EthMovementType::Direct,
        });
    }

    // 2. Gas payment
    let gas_cost = tx.fees.gas_price * U256::from(tx.fees.gas_used);
    if gas_cost > U256::ZERO {
        eth_movements.push(EthMovement {
            from: tx.from_address,
            to: Address::ZERO, // Miners/validators
            amount: gas_cost,
            movement_type: EthMovementType::Gas,
        });
    }

    // 3. Internal transactions (ETH transfers)
    for internal in &tx.internal_transactions {
        if internal.value > U256::ZERO {
            eth_movements.push(EthMovement {
                from: internal.from_address,
                to: internal.to_address.unwrap_or(Address::ZERO),
                amount: internal.value,
                movement_type: EthMovementType::Internal,
            });
        }
    }

    // 4. ERC20 transfers
    for transfer in &tx.erc20_transfers {
        token_movements.push(TokenMovement {
            token_address: transfer.token_address,
            from: transfer.from_address,
            to: transfer.to_address,
            amount: transfer.amount,
            token_symbol: None, // Could be enriched from token metadata
            token_decimals: None,
        });
    }

    // 5. ERC721 transfers (treat each NFT as 1 unit)
    for transfer in &tx.erc721_transfers {
        token_movements.push(TokenMovement {
            token_address: transfer.token_address,
            from: transfer.from_address,
            to: transfer.to_address,
            amount: U256::from(1), // NFT = 1 unit
            token_symbol: None,
            token_decimals: Some(0),
        });
    }

    // 6. ERC1155 transfers. The current core flow type has no token_id field, so
    // each id/amount pair becomes one movement for the collection contract.
    for transfer in &tx.erc1155_transfers {
        for amount in &transfer.amounts {
            token_movements.push(TokenMovement {
                token_address: transfer.token_address,
                from: transfer.from_address,
                to: transfer.to_address,
                amount: *amount,
                token_symbol: None,
                token_decimals: Some(0),
            });
        }
    }

    Ok(CompleteFundFlows {
        tx_hash: tx.hash.to_string(),
        block_number: tx.block_number,
        from_address: tx.from_address,
        to_address: tx.to_address,
        eth_movements,
        token_movements,
    })
}

/// Converter for batch processing
pub struct ProcessedTxConverter;

impl ProcessedTxConverter {
    /// Convert a batch of ProcessedTransactions to CompleteFundFlows
    pub fn convert_batch(txs: &[ProcessedTransaction]) -> Result<Vec<CompleteFundFlows>> {
        txs.iter()
            .map(extract_fund_flows_from_processed_tx)
            .collect()
    }

    /// Extract all unique addresses from a ProcessedTransaction
    pub fn extract_addresses(tx: &ProcessedTransaction) -> Vec<Address> {
        let mut addresses = vec![tx.from_address];

        if let Some(to) = tx.to_address {
            addresses.push(to);
        }

        // From internal transactions
        for internal in &tx.internal_transactions {
            addresses.push(internal.from_address);
            if let Some(to) = internal.to_address {
                addresses.push(to);
            }
        }

        // From token transfers
        for transfer in &tx.erc20_transfers {
            addresses.push(transfer.from_address);
            addresses.push(transfer.to_address);
            addresses.push(transfer.token_address);
        }

        for transfer in &tx.erc721_transfers {
            addresses.push(transfer.from_address);
            addresses.push(transfer.to_address);
            addresses.push(transfer.token_address);
        }

        for transfer in &tx.erc1155_transfers {
            addresses.push(transfer.from_address);
            addresses.push(transfer.to_address);
            addresses.push(transfer.token_address);
        }

        // Deduplicate
        addresses.sort();
        addresses.dedup();
        addresses
    }

    /// Check if address is WETH
    pub fn is_weth(address: Address) -> bool {
        // Mainnet WETH
        address
            == Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
                .unwrap_or(Address::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::B256;

    #[test]
    fn test_extract_fund_flows() {
        // Create a test ProcessedTransaction
        let mut tx = ProcessedTransaction::new(
            B256::ZERO,
            0,
            0,
            0,
            Address::from([1u8; 20]),
            Some(Address::from([2u8; 20])),
            U256::from(1_000_000_000_000_000_000u128), // 1 ETH
            true,
            0,
            0,
            Vec::new(),
        );

        // Add gas fees
        tx.fees.gas_price = U256::from(20_000_000_000u64); // 20 gwei
        tx.fees.gas_used = 21000;

        // Extract fund flows
        let flows = extract_fund_flows_from_processed_tx(&tx).unwrap();

        // Should have 2 ETH movements: main transfer + gas
        assert_eq!(flows.eth_movements.len(), 2);
        assert_eq!(
            flows.eth_movements[0].movement_type,
            EthMovementType::Direct
        );
        assert_eq!(flows.eth_movements[1].movement_type, EthMovementType::Gas);
    }
}
