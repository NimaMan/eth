use std::collections::{HashMap, HashSet};

use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use indexmap::IndexMap;
use reth_chain_query::common_addresses::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS};
use serde::{Deserialize, Serialize};
use tx_processor::{Erc20CallKind, ProcessedTransaction};

use crate::utils::{append_to_index_map_history, append_with_history_limit};

pub const WETH_SYMBOL: &str = "WETH";
pub const ETH_SYMBOL: &str = "ETH";
pub const ZERO_ADDRESS: Address = Address::ZERO;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenTransferRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub tx_index: u64,
    pub log_index: u64,
    pub from_address: String,
    pub to_address: String,
    pub amount: f64,
    pub token_address: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InternalEthTransferRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub tx_index: u64,
    pub depth: u32,
    pub from_address: String,
    pub to_address: Option<String>,
    pub amount: f64,
    pub token_address: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub tx_index: u64,
    pub log_index: u64,
    pub owner: String,
    pub spender: String,
    pub token_address: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenTransferFromCallRecord {
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub caller: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: f64,
    pub token_address: String,
    pub emitted_transfer_count: u32,
    pub emitted_approval_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenTransferTracker {
    pub contract_address: String,
    pub decimals: u8,
    pub history_limit: usize,
    pub erc20_transfers: IndexMap<String, Vec<TokenTransferRecord>>,
    pub eth_transfers: IndexMap<String, Vec<InternalEthTransferRecord>>,
    pub weth_transfers: IndexMap<String, Vec<TokenTransferRecord>>,
    pub other_denom_transfers: IndexMap<String, Vec<TokenTransferRecord>>,
    pub approvals: Vec<ApprovalRecord>,
    #[serde(default)]
    pub transfer_from_calls: Vec<TokenTransferFromCallRecord>,
    pub approved_addresses: HashSet<String>,
    pub other_currencies: HashMap<String, u64>,
    pub address_tx_counter: HashMap<String, u64>,
    pub total_supply_from_transfers: f64,
    pub total_bribe_amount: f64,
    pub bribe_amounts_by_tx: HashMap<String, f64>,
}

impl TokenTransferTracker {
    pub fn new(contract_address: impl Into<String>, decimals: u8, history_limit: usize) -> Self {
        Self {
            contract_address: normalize_address_string(contract_address),
            decimals,
            history_limit,
            erc20_transfers: IndexMap::new(),
            eth_transfers: IndexMap::new(),
            weth_transfers: IndexMap::new(),
            other_denom_transfers: IndexMap::new(),
            approvals: Vec::new(),
            transfer_from_calls: Vec::new(),
            approved_addresses: HashSet::new(),
            other_currencies: HashMap::new(),
            address_tx_counter: HashMap::new(),
            total_supply_from_transfers: 0.0,
            total_bribe_amount: 0.0,
            bribe_amounts_by_tx: HashMap::new(),
        }
    }

    pub fn update_from_processed_transaction(&mut self, tx: &ProcessedTransaction) -> Result<()> {
        self.add_transfers(tx)?;
        self.add_internal_eth_transfers(tx)?;
        self.add_approvals(tx);
        self.add_transfer_from_call(tx)?;
        self.update_address_tx_counter(tx.unique_addresses.iter().copied());
        self.update_bribe_amount(tx)?;
        Ok(())
    }

    pub fn add_transfers(&mut self, tx: &ProcessedTransaction) -> Result<()> {
        let tx_hash = hash_string(&tx.hash);
        for transfer in &tx.erc20_transfers {
            if same_address_str(transfer.token_address, &self.contract_address) {
                let amount = scale_amount(transfer.amount, self.decimals)?;
                let record = TokenTransferRecord {
                    tx_hash: tx_hash.clone(),
                    block_number: tx.block_number,
                    tx_index: tx.tx_index,
                    log_index: transfer.log_index,
                    from_address: address_string(&transfer.from_address),
                    to_address: address_string(&transfer.to_address),
                    amount,
                    token_address: address_string(&transfer.token_address),
                };
                append_to_index_map_history(
                    &mut self.erc20_transfers,
                    tx_hash.clone(),
                    record,
                    self.history_limit,
                );
                if transfer.from_address == ZERO_ADDRESS {
                    self.total_supply_from_transfers += amount;
                }
                continue;
            }

            let Some(symbol) = DENOM_ADDRESSES.get(&transfer.token_address).copied() else {
                continue;
            };
            let decimals = ERC20_TOKEN_DECIMALS.get(symbol).copied().unwrap_or(18);
            let record = TokenTransferRecord {
                tx_hash: tx_hash.clone(),
                block_number: tx.block_number,
                tx_index: tx.tx_index,
                log_index: transfer.log_index,
                from_address: address_string(&transfer.from_address),
                to_address: address_string(&transfer.to_address),
                amount: scale_amount(transfer.amount, decimals)?,
                token_address: if symbol == WETH_SYMBOL {
                    WETH_SYMBOL.to_string()
                } else {
                    address_string(&transfer.token_address)
                },
            };

            if symbol == WETH_SYMBOL {
                append_to_index_map_history(
                    &mut self.weth_transfers,
                    tx_hash.clone(),
                    record,
                    self.history_limit,
                );
            } else {
                *self.other_currencies.entry(symbol.to_string()).or_insert(0) += 1;
                append_to_index_map_history(
                    &mut self.other_denom_transfers,
                    tx_hash.clone(),
                    record,
                    self.history_limit,
                );
            }
        }
        Ok(())
    }

    pub fn add_internal_eth_transfers(&mut self, tx: &ProcessedTransaction) -> Result<()> {
        let tx_hash = hash_string(&tx.hash);
        for transfer in &tx.internal_transactions {
            let record = InternalEthTransferRecord {
                tx_hash: tx_hash.clone(),
                block_number: tx.block_number,
                tx_index: tx.tx_index,
                depth: transfer.depth,
                from_address: address_string(&transfer.from_address),
                to_address: transfer.to_address.map(|address| address_string(&address)),
                amount: scale_amount(transfer.value, 18)?,
                token_address: ETH_SYMBOL.to_string(),
            };
            append_to_index_map_history(
                &mut self.eth_transfers,
                tx_hash.clone(),
                record,
                self.history_limit,
            );
        }
        Ok(())
    }

    pub fn add_approvals(&mut self, tx: &ProcessedTransaction) {
        let tx_hash = hash_string(&tx.hash);
        for approval in &tx.erc20_approval_events {
            let spender = address_string(&approval.spender);
            append_with_history_limit(
                &mut self.approvals,
                ApprovalRecord {
                    tx_hash: tx_hash.clone(),
                    block_number: tx.block_number,
                    tx_index: tx.tx_index,
                    log_index: approval.log_index,
                    owner: address_string(&approval.owner),
                    spender: spender.clone(),
                    token_address: address_string(&approval.token_address),
                },
                self.history_limit,
            );
            self.approved_addresses.insert(spender);
        }
    }

    // Records a direct (top-level) `transferFrom` to this token. The source is
    // the trace-derived `internal_erc20_calls` (depth 0 = the tx's own call),
    // NOT raw calldata: the processed-block disk cache (v3) no longer stores
    // `input`, but `internal_erc20_calls` carries the same (from, to, amount)
    // and is independent of `input`. See tx_processor block/README.md.
    pub fn add_transfer_from_call(&mut self, tx: &ProcessedTransaction) -> Result<()> {
        let Some(to_address) = tx.to_address else {
            return Ok(());
        };
        if !same_address_str(to_address, &self.contract_address) {
            return Ok(());
        }
        let Some(call) = tx.internal_erc20_calls.iter().find(|call| {
            call.depth == 0
                && matches!(call.kind, Erc20CallKind::TransferFrom)
                && same_address_str(call.token_address, &self.contract_address)
        }) else {
            return Ok(());
        };
        let (from_address, transfer_to, amount) = (call.from_address, call.to_address, call.amount);

        let emitted_transfer_count = tx
            .erc20_transfers
            .iter()
            .filter(|transfer| same_address_str(transfer.token_address, &self.contract_address))
            .count() as u32;
        let emitted_approval_count = tx
            .erc20_approval_events
            .iter()
            .filter(|approval| same_address_str(approval.token_address, &self.contract_address))
            .count() as u32;

        let record = TokenTransferFromCallRecord {
            tx_hash: hash_string(&tx.hash),
            block_number: tx.block_number,
            block_timestamp: tx.block_timestamp,
            tx_index: tx.tx_index,
            caller: address_string(&tx.from_address),
            from_address: address_string(&from_address),
            to_address: address_string(&transfer_to),
            amount: scale_amount(amount, self.decimals)?,
            token_address: self.contract_address.clone(),
            emitted_transfer_count,
            emitted_approval_count,
        };
        append_with_history_limit(&mut self.transfer_from_calls, record, self.history_limit);
        Ok(())
    }

    pub fn update_address_tx_counter(&mut self, addresses: impl IntoIterator<Item = Address>) {
        for address in addresses {
            *self
                .address_tx_counter
                .entry(address_string(&address))
                .or_insert(0) += 1;
        }
    }

    pub fn update_bribe_amount(&mut self, tx: &ProcessedTransaction) -> Result<()> {
        if tx.bribe_amount.is_zero() {
            return Ok(());
        }
        let amount = scale_amount(tx.bribe_amount, 18)?;
        let from_address = address_string(&tx.from_address);
        self.bribe_amounts_by_tx.insert(from_address, amount);
        self.total_bribe_amount += amount;
        Ok(())
    }
}

fn scale_amount(value: U256, decimals: u8) -> Result<f64> {
    let raw = value.to_string().parse::<f64>()?;
    Ok(raw / 10_f64.powi(i32::from(decimals)))
}


fn same_address_str(address: Address, value: &str) -> bool {
    address_string(&address) == normalize_address(value)
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: impl Into<String>) -> String {
    normalize_address(value.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use tx_processor::tx_processor::data_models::{
        ERC20ApprovalEvent, ERC20TransferEvent, InternalTransaction,
    };

    fn tx() -> ProcessedTransaction {
        let mut tx = ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            address!("1111111111111111111111111111111111111111"),
            None,
            U256::ZERO,
            true,
            1,
            2,
            Vec::new(),
        );
        tx.unique_addresses
            .insert(address!("2222222222222222222222222222222222222222"));
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: address!("3333333333333333333333333333333333333333"),
            from_address: Address::ZERO,
            to_address: address!("2222222222222222222222222222222222222222"),
            amount: U256::from(1_500_000_000_000_000_000_u128),
            log_index: 7,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
            from_address: address!("2222222222222222222222222222222222222222"),
            to_address: address!("4444444444444444444444444444444444444444"),
            amount: U256::from(2_000_000_000_000_000_000_u128),
            log_index: 8,
        });
        tx.erc20_approval_events.push(ERC20ApprovalEvent {
            token_address: address!("3333333333333333333333333333333333333333"),
            owner: address!("2222222222222222222222222222222222222222"),
            spender: address!("5555555555555555555555555555555555555555"),
            amount: U256::from(10_u64),
            log_index: 9,
        });
        tx.internal_transactions.push(InternalTransaction {
            from_address: address!("2222222222222222222222222222222222222222"),
            to_address: Some(address!("4444444444444444444444444444444444444444")),
            value: U256::from(3_000_000_000_000_000_000_u128),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("call".to_string()),
            depth: 1,
            error: None,
        });
        tx.bribe_amount = U256::from(4_000_000_000_000_000_000_u128);
        tx
    }

    #[test]
    fn tracks_token_transfers_denoms_internal_eth_approvals_and_bribes() {
        let mut tracker =
            TokenTransferTracker::new("0x3333333333333333333333333333333333333333", 18, 10);

        tracker.update_from_processed_transaction(&tx()).unwrap();

        assert_eq!(tracker.erc20_transfers.len(), 1);
        assert_eq!(
            tracker.erc20_transfers.values().next().unwrap()[0].amount,
            1.5
        );
        assert_eq!(tracker.total_supply_from_transfers, 1.5);
        assert_eq!(
            tracker.weth_transfers.values().next().unwrap()[0].token_address,
            WETH_SYMBOL
        );
        assert_eq!(
            tracker.eth_transfers.values().next().unwrap()[0].amount,
            3.0
        );
        assert_eq!(tracker.approvals.len(), 1);
        assert!(tracker
            .approved_addresses
            .contains("0x5555555555555555555555555555555555555555"));
        assert_eq!(
            tracker.address_tx_counter["0x2222222222222222222222222222222222222222"],
            1
        );
        assert_eq!(tracker.total_bribe_amount, 4.0);
    }

    #[test]
    fn tracks_transfer_from_calldata_for_token_contract_calls() {
        let mut tx = tx();
        tx.input = transfer_from_input(
            address!("2222222222222222222222222222222222222222"),
            address!("000000000000000000000000000000000000dEaD"),
            U256::from(2_500_000_000_000_000_000_u128),
        );
        tx.to_address = Some(address!("3333333333333333333333333333333333333333"));

        let mut tracker =
            TokenTransferTracker::new("0x3333333333333333333333333333333333333333", 18, 10);

        tracker.update_from_processed_transaction(&tx).unwrap();

        let call = tracker.transfer_from_calls.first().unwrap();
        assert_eq!(
            call.from_address,
            "0x2222222222222222222222222222222222222222"
        );
        assert_eq!(
            call.to_address,
            "0x000000000000000000000000000000000000dead"
        );
        assert_eq!(call.amount, 2.5);
        assert_eq!(call.emitted_transfer_count, 1);
        assert_eq!(call.emitted_approval_count, 1);
    }

    fn transfer_from_input(from: Address, to: Address, amount: U256) -> Vec<u8> {
        let mut input = vec![0x23, 0xb8, 0x72, 0xdd];
        input.extend_from_slice(from.into_word().as_slice());
        input.extend_from_slice(to.into_word().as_slice());
        input.extend_from_slice(amount.to_be_bytes::<32>().as_slice());
        input
    }
}
