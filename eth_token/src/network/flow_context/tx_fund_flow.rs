//! Adapter from `tx_fund_flow` processed-transaction extraction into token flow context.

use std::collections::BTreeSet;

use alloy_primitives::{Address, U256};
use eyre::Result;
use reth_chain_query::common_addresses::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS};
use tx_fund_flow_core_types::{EthMovement, EthMovementType, TokenMovement};
use tx_fund_flow_fundflownetwork::extract_fund_flows_from_processed_tx;
use tx_processor::{ProcessedBlock, ProcessedTransaction};

use crate::network::{
    ingest::transaction::{address_string, hash_string, scale_u256},
    model::{normalize_network_address, NetworkObservation},
};

use super::extractor::{FlowContextObservation, FlowObservationExtractor};

const ETH_SYMBOL: &str = "ETH";
const ETH_DECIMALS: u8 = 18;

/// Controls which `tx_fund_flow` movements become second-order context observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxFundFlowObservationConfig {
    /// Include the transaction's direct native ETH value transfer.
    pub include_direct_eth: bool,
    /// Include native ETH movements discovered in internal calls.
    pub include_internal_eth: bool,
    /// Include gas payment movements. Disabled by default because they point at
    /// validator-like sinks and are usually high-noise for token coordination.
    pub include_gas_eth: bool,
    /// Include ERC-20 movements only when the token is a known denomination
    /// asset such as WETH or a stablecoin.
    pub include_known_denom_tokens: bool,
    /// Include movements from failed transactions. Disabled by default because
    /// logs and balance effects from failed transactions are not reliable signal.
    pub include_failed_transactions: bool,
    /// Keep zero-address endpoints. Disabled by default to suppress mint, burn,
    /// contract-creation, and gas-placeholder edges from the context layer.
    pub include_zero_address_flows: bool,
}

impl Default for TxFundFlowObservationConfig {
    fn default() -> Self {
        Self {
            include_direct_eth: true,
            include_internal_eth: true,
            include_gas_eth: false,
            include_known_denom_tokens: true,
            include_failed_transactions: false,
            include_zero_address_flows: false,
        }
    }
}

/// Concrete [`FlowObservationExtractor`] backed by `tx_fund_flow`.
///
/// The token network selects seed addresses and block windows. The builder then
/// loads processed blocks for those windows and passes them here. This adapter
/// emits only ETH/WETH/stable denomination flows that touch at least one seed,
/// leaving shared-funder/shared-sink inference to the flow-context aggregator.
#[derive(Clone, Debug, Default)]
pub struct TxFundFlowObservationExtractor {
    pub config: TxFundFlowObservationConfig,
}

impl TxFundFlowObservationExtractor {
    pub fn new(config: TxFundFlowObservationConfig) -> Self {
        Self { config }
    }

    fn extract_tx_observations(
        &self,
        tx: &ProcessedTransaction,
        seed_addresses: &BTreeSet<String>,
        observations: &mut Vec<FlowContextObservation>,
    ) -> Result<()> {
        if !tx.status && !self.config.include_failed_transactions {
            return Ok(());
        }

        let flows = extract_fund_flows_from_processed_tx(tx)?;
        let mut successful_internal_eth = SuccessfulInternalEthLookup::from_transaction(tx);
        let mut token_log_indexes = TokenLogIndexLookup::from_transaction(tx);

        for movement in &flows.eth_movements {
            if !self.include_eth_movement(movement) {
                continue;
            }
            if movement.movement_type == EthMovementType::Internal
                && !successful_internal_eth.take(movement)
            {
                continue;
            }
            self.push_eth_observation(tx, movement, seed_addresses, observations);
        }

        if self.config.include_known_denom_tokens {
            for movement in &flows.token_movements {
                let log_index = token_log_indexes.take(movement);
                self.push_token_observation(tx, movement, log_index, seed_addresses, observations);
            }
        }

        Ok(())
    }

    fn include_eth_movement(&self, movement: &EthMovement) -> bool {
        match movement.movement_type {
            EthMovementType::Direct => self.config.include_direct_eth,
            EthMovementType::Internal | EthMovementType::Refund | EthMovementType::SelfDestruct => {
                self.config.include_internal_eth
            }
            EthMovementType::Gas => self.config.include_gas_eth,
        }
    }

    fn push_eth_observation(
        &self,
        tx: &ProcessedTransaction,
        movement: &EthMovement,
        seed_addresses: &BTreeSet<String>,
        observations: &mut Vec<FlowContextObservation>,
    ) {
        if movement.amount.is_zero() || self.should_skip_endpoint_pair(movement.from, movement.to) {
            return;
        }

        let from = normalized_address(&movement.from);
        let to = normalized_address(&movement.to);
        if !touches_seed(&from, &to, seed_addresses) {
            return;
        }

        observations.push(
            FlowContextObservation::new(&from, &to, tx_observation(tx, None)).with_amount(
                None,
                Some(ETH_SYMBOL.to_string()),
                Some(movement.amount.to_string()),
                Some(scale_u256(movement.amount, ETH_DECIMALS)),
            ),
        );
    }

    fn push_token_observation(
        &self,
        tx: &ProcessedTransaction,
        movement: &TokenMovement,
        log_index: Option<u64>,
        seed_addresses: &BTreeSet<String>,
        observations: &mut Vec<FlowContextObservation>,
    ) {
        if movement.amount.is_zero() || self.should_skip_endpoint_pair(movement.from, movement.to) {
            return;
        }

        let Some(symbol) = known_denom_symbol(movement) else {
            return;
        };

        let from = normalized_address(&movement.from);
        let to = normalized_address(&movement.to);
        if !touches_seed(&from, &to, seed_addresses) {
            return;
        }

        let decimals = movement
            .token_decimals
            .or_else(|| ERC20_TOKEN_DECIMALS.get(symbol).copied())
            .unwrap_or(ETH_DECIMALS);
        observations.push(
            FlowContextObservation::new(&from, &to, tx_observation(tx, log_index)).with_amount(
                Some(normalized_address(&movement.token_address)),
                Some(symbol.to_string()),
                Some(movement.amount.to_string()),
                Some(scale_u256(movement.amount, decimals)),
            ),
        );
    }

    fn should_skip_endpoint_pair(&self, from: Address, to: Address) -> bool {
        !self.config.include_zero_address_flows && (from == Address::ZERO || to == Address::ZERO)
    }
}

impl FlowObservationExtractor for TxFundFlowObservationExtractor {
    fn extract_observations(
        &self,
        blocks: &[ProcessedBlock],
        seed_addresses: &BTreeSet<String>,
    ) -> Result<Vec<FlowContextObservation>> {
        let seed_addresses = seed_addresses
            .iter()
            .map(|address| normalize_network_address(address))
            .collect::<BTreeSet<_>>();
        let mut observations = Vec::new();

        for block in blocks {
            for tx in &block.transactions {
                self.extract_tx_observations(&tx.processed, &seed_addresses, &mut observations)?;
            }
        }

        Ok(observations)
    }
}

#[derive(Clone, Debug)]
struct TokenLogIndexLookup {
    entries: Vec<(Address, Address, Address, U256, u64)>,
}

#[derive(Clone, Debug)]
struct SuccessfulInternalEthLookup {
    entries: Vec<(Address, Address, U256)>,
}

impl SuccessfulInternalEthLookup {
    fn from_transaction(tx: &ProcessedTransaction) -> Self {
        Self {
            entries: tx
                .internal_transactions
                .iter()
                .filter(|internal| internal.error.is_none() && !internal.value.is_zero())
                .filter_map(|internal| {
                    Some((internal.from_address, internal.to_address?, internal.value))
                })
                .collect(),
        }
    }

    fn take(&mut self, movement: &EthMovement) -> bool {
        let Some(index) = self.entries.iter().position(|entry| {
            entry.0 == movement.from && entry.1 == movement.to && entry.2 == movement.amount
        }) else {
            return false;
        };
        self.entries.remove(index);
        true
    }
}

impl TokenLogIndexLookup {
    fn from_transaction(tx: &ProcessedTransaction) -> Self {
        Self {
            entries: tx
                .erc20_transfers
                .iter()
                .map(|transfer| {
                    (
                        transfer.token_address,
                        transfer.from_address,
                        transfer.to_address,
                        transfer.amount,
                        transfer.log_index,
                    )
                })
                .collect(),
        }
    }

    fn take(&mut self, movement: &TokenMovement) -> Option<u64> {
        let index = self.entries.iter().position(|entry| {
            entry.0 == movement.token_address
                && entry.1 == movement.from
                && entry.2 == movement.to
                && entry.3 == movement.amount
        })?;
        Some(self.entries.remove(index).4)
    }
}

fn known_denom_symbol(movement: &TokenMovement) -> Option<&'static str> {
    DENOM_ADDRESSES.get(&movement.token_address).copied()
}

fn normalized_address(address: &Address) -> String {
    normalize_network_address(address_string(address))
}

fn touches_seed(from: &str, to: &str, seed_addresses: &BTreeSet<String>) -> bool {
    seed_addresses.contains(from) || seed_addresses.contains(to)
}

fn tx_observation(tx: &ProcessedTransaction, log_index: Option<u64>) -> NetworkObservation {
    NetworkObservation::with_transaction(
        tx.block_number,
        Some(tx.block_timestamp),
        Some(hash_string(&tx.hash)),
        Some(tx.tx_index),
        log_index,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use alloy_primitives::{address, Address, Bytes, B256, U256};
    use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
    use tx_processor::{
        tx_processor::data_models::{ERC20TransferEvent, InternalTransaction},
        ProcessedBlockTransactions,
    };

    use super::*;
    use crate::network::flow_context::extractor::FlowObservationExtractor;

    #[test]
    fn emits_seed_touching_eth_observations_from_tx_fund_flow() {
        let seed = test_address(1);
        let funder = test_address(9);
        let other = test_address(8);
        let seed_addresses = BTreeSet::from([normalized_address(&seed)]);

        let direct = processed_tx(
            1,
            funder,
            Some(seed),
            U256::from(1_000_000_000_000_000_000u128),
            true,
        );
        let mut internal = processed_tx(2, other, None, U256::ZERO, true);
        internal.internal_transactions.push(InternalTransaction {
            from_address: funder,
            to_address: Some(seed),
            value: U256::from(500_000_000_000_000_000u128),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("call".to_string()),
            depth: 1,
            error: None,
        });
        internal.internal_transactions.push(InternalTransaction {
            from_address: funder,
            to_address: Some(seed),
            value: U256::from(250_000_000_000_000_000u128),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("call".to_string()),
            depth: 1,
            error: Some("revert".to_string()),
        });
        let non_seed = processed_tx(
            3,
            funder,
            Some(other),
            U256::from(1_000_000_000_000_000_000u128),
            true,
        );
        let failed = processed_tx(
            4,
            funder,
            Some(seed),
            U256::from(2_000_000_000_000_000_000u128),
            false,
        );

        let extractor = TxFundFlowObservationExtractor::default();
        let observations = extractor
            .extract_observations(
                &[processed_block(vec![direct, internal, non_seed, failed])],
                &seed_addresses,
            )
            .expect("extracts observations");

        assert_eq!(observations.len(), 2);
        assert!(observations
            .iter()
            .any(|observation| observation.symbol.as_deref() == Some("ETH")
                && observation.scaled_amount == Some(1.0)));
        assert!(observations
            .iter()
            .any(|observation| observation.symbol.as_deref() == Some("ETH")
                && observation.scaled_amount == Some(0.5)));
        assert!(!observations
            .iter()
            .any(|observation| observation.scaled_amount == Some(0.25)));
        assert!(observations.iter().all(|observation| observation.from
            == normalized_address(&funder)
            && observation.to == normalized_address(&seed)));
    }

    #[test]
    fn emits_known_denom_token_observation_with_log_index() {
        let seed = test_address(1);
        let funder = test_address(9);
        let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
        let unknown_token = test_address(7);
        let seed_addresses = BTreeSet::from([normalized_address(&seed)]);

        let mut tx = processed_tx(5, funder, Some(seed), U256::ZERO, true);
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: weth,
            from_address: funder,
            to_address: seed,
            amount: U256::from(2_000_000_000_000_000_000u128),
            log_index: 11,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: unknown_token,
            from_address: funder,
            to_address: seed,
            amount: U256::from(1_000_000u64),
            log_index: 12,
        });

        let extractor = TxFundFlowObservationExtractor::default();
        let observations = extractor
            .extract_observations(&[processed_block(vec![tx])], &seed_addresses)
            .expect("extracts observations");

        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(
            observation.asset.as_deref(),
            Some(normalized_address(&weth).as_str())
        );
        assert_eq!(observation.symbol.as_deref(), Some("WETH"));
        assert_eq!(
            observation.raw_amount.as_deref(),
            Some("2000000000000000000")
        );
        assert_eq!(observation.scaled_amount, Some(2.0));
        assert_eq!(observation.observation.log_index, Some(11));
    }

    fn processed_tx(
        hash_byte: u8,
        from: Address,
        to: Option<Address>,
        value: U256,
        status: bool,
    ) -> ProcessedTransaction {
        ProcessedTransaction::new(
            B256::from([hash_byte; 32]),
            100,
            1_700,
            u64::from(hash_byte),
            from,
            to,
            value,
            status,
            0,
            2,
            Vec::new(),
        )
    }

    fn processed_block(transactions: Vec<ProcessedTransaction>) -> ProcessedBlock {
        ProcessedBlock {
            header: BlockHeader {
                number: 100,
                hash: B256::from([0x99; 32]),
                parent_hash: B256::from([0x88; 32]),
                timestamp: 1_700,
                gas_limit: 30_000_000,
                gas_used: 21_000,
                base_fee_per_gas: Some(1),
                withdrawals_root: None,
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
                requests_hash: None,
                block_access_list_hash: None,
                slot_number: None,
            },
            transactions: transactions.into_iter().map(block_transaction).collect(),
        }
    }

    fn block_transaction(processed: ProcessedTransaction) -> ProcessedBlockTransactions {
        ProcessedBlockTransactions {
            metadata: TransactionData {
                hash: processed.hash,
                block_number: processed.block_number,
                block_timestamp: processed.block_timestamp,
                tx_index: processed.tx_index,
                tx_number: processed.tx_index,
                from: processed.from_address,
                to: processed.to_address,
                value: processed.value,
                input: Bytes::from(processed.input.clone()),
                gas_price: U256::ZERO,
                gas_limit: 21_000,
                nonce: processed.nonce,
                transaction_type: processed.raw_tx_type,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                access_list: Vec::new(),
                blob_versioned_hashes: Vec::new(),
                max_fee_per_blob_gas: None,
                signed_authorizations: Vec::new(),
            },
            receipt: TransactionReceipt {
                tx_hash: processed.hash,
                status: processed.status,
                gas_used: 21_000,
                logs: Vec::new(),
                cumulative_gas_used: 21_000,
                effective_gas_price: U256::ZERO,
                contract_address: processed.contract_address,
                blob_gas_used: None,
            },
            processed,
            trace: None,
            processing_error: None,
        }
    }

    fn test_address(index: u8) -> Address {
        Address::from([index; 20])
    }
}
