use std::collections::BTreeMap;

use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

pub mod conservation;
pub mod export;
pub mod model;

pub use conservation::PoolPnlConservationCheck;
pub use model::{
    PnlAddressPositionExport, PnlConservationExport, PnlMovementExport, PnlPoolExport,
};

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPnlTracker {
    #[serde(default)]
    pools: BTreeMap<String, PoolPnlTracker>,
}

impl TokenPnlTracker {
    pub fn register_pool(
        &mut self,
        pool_address: impl AsRef<str>,
        token_address: impl AsRef<str>,
        denom_address: impl AsRef<str>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
    ) {
        let pool_address = normalize_address(pool_address);
        let token_address = normalize_address(token_address);
        let denom_address = normalize_address(denom_address);
        self.pools
            .entry(pool_address.clone())
            .and_modify(|pool| {
                pool.token_address = token_address.clone();
                pool.denom_address = denom_address.clone();
                pool.token_decimals = token_decimals;
                pool.denom_decimals = denom_decimals;
                pool.history_limit = history_limit;
                pool.prune_history();
            })
            .or_insert_with(|| {
                PoolPnlTracker::new(
                    pool_address,
                    token_address,
                    denom_address,
                    token_decimals,
                    denom_decimals,
                    history_limit,
                )
            });
    }

    pub fn record_v2_pool_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        token_address: impl AsRef<str>,
        denom_address: impl AsRef<str>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
        transaction: &ProcessedTransaction,
    ) {
        self.register_pool(
            pool_address.as_ref(),
            token_address.as_ref(),
            denom_address.as_ref(),
            token_decimals,
            denom_decimals,
            history_limit,
        );
        if let Some(pool) = self.pool_mut(pool_address.as_ref()) {
            pool.record_transaction(transaction);
        }
    }

    pub fn pool(&self, pool_address: impl AsRef<str>) -> Option<&PoolPnlTracker> {
        self.pools.get(&normalize_address(pool_address))
    }

    pub fn pool_mut(&mut self, pool_address: impl AsRef<str>) -> Option<&mut PoolPnlTracker> {
        self.pools.get_mut(&normalize_address(pool_address))
    }

    pub fn pools(&self) -> impl Iterator<Item = (&String, &PoolPnlTracker)> {
        self.pools.iter()
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlTracker {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub history_limit: usize,
    #[serde(default)]
    pub positions: BTreeMap<String, AddressPoolPosition>,
    #[serde(default)]
    pub conservation: PoolPnlConservationTotals,
    #[serde(default)]
    pub recent_entries: Vec<PoolPnlEntry>,
    #[serde(default)]
    pub tx_count: u64,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
}

impl PoolPnlTracker {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
    ) -> Self {
        Self {
            pool_address: normalize_address_string(pool_address.into()),
            token_address: normalize_address_string(token_address.into()),
            denom_address: normalize_address_string(denom_address.into()),
            token_decimals,
            denom_decimals,
            history_limit,
            positions: BTreeMap::new(),
            conservation: PoolPnlConservationTotals::default(),
            recent_entries: Vec::new(),
            tx_count: 0,
            latest_block_number: None,
            latest_block_timestamp: None,
        }
    }

    pub fn record_transaction(&mut self, transaction: &ProcessedTransaction) {
        let mut matched_transfer_count = 0u32;
        for transfer in &transaction.erc20_transfers {
            let token_address = address_string(&transfer.token_address);
            if token_address == self.token_address {
                matched_transfer_count += 1;
                let pool_direct = self.is_pool_side(&transfer.from_address)
                    || self.is_pool_side(&transfer.to_address);
                self.record_token_transfer(
                    &transfer.from_address,
                    &transfer.to_address,
                    transfer.amount,
                    Some(transfer.log_index),
                    pool_direct,
                    transaction,
                );
            } else if token_address == self.denom_address {
                matched_transfer_count += 1;
                let pool_direct = self.is_pool_side(&transfer.from_address)
                    || self.is_pool_side(&transfer.to_address);
                self.record_denom_transfer(
                    &transfer.from_address,
                    &transfer.to_address,
                    transfer.amount,
                    Some(transfer.log_index),
                    pool_direct,
                    transaction,
                );
            }
        }
        if self.denom_tracks_native_eth() {
            matched_transfer_count = matched_transfer_count
                .saturating_add(self.record_native_denom_transfers(transaction));
        }

        if matched_transfer_count == 0
            && transaction.fees.tx_fee.is_zero()
            && transaction.bribe_amount.is_zero()
        {
            return;
        }

        if !transaction.fees.tx_fee.is_zero() {
            self.record_native_fee(
                &transaction.from_address,
                transaction.fees.tx_fee,
                transaction,
            );
        }
        if !transaction.bribe_amount.is_zero() {
            self.record_native_bribe(
                &transaction.from_address,
                transaction.bribe_amount,
                transaction,
            );
        }

        self.tx_count = self.tx_count.saturating_add(1);
        self.latest_block_number = Some(transaction.block_number);
        self.latest_block_timestamp = Some(transaction.block_timestamp);
    }

    pub fn position(&self, address: impl AsRef<str>) -> Option<&AddressPoolPosition> {
        self.positions.get(&normalize_address(address))
    }

    pub fn address_summaries(
        &self,
        mark_price_denom_per_token: Option<f64>,
    ) -> Vec<AddressPoolPnlSummary> {
        self.positions
            .values()
            .map(|position| self.summary_for_position(position, mark_price_denom_per_token))
            .collect()
    }

    pub fn top_positions_by_denom_volume(
        &self,
        limit: usize,
        include_pool_and_zero: bool,
        mark_price_denom_per_token: Option<f64>,
    ) -> Vec<AddressPoolPnlSummary> {
        let mut positions = self
            .positions
            .values()
            .filter(|position| {
                include_pool_and_zero
                    || (position.address != self.pool_address && position.address != ZERO_ADDRESS)
            })
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| {
            let left_volume = left.denom_in_raw.saturating_add(left.denom_out_raw);
            let right_volume = right.denom_in_raw.saturating_add(right.denom_out_raw);
            right_volume.cmp(&left_volume)
        });
        positions
            .into_iter()
            .take(limit)
            .map(|position| self.summary_for_position(position, mark_price_denom_per_token))
            .collect()
    }

    pub fn conservation_summary(&self) -> PoolPnlConservationSummary {
        PoolPnlConservationSummary {
            token_delta_raw: signed_raw_string(
                self.conservation.token_in_raw,
                self.conservation.token_out_raw,
            ),
            denom_delta_raw: signed_raw_string(
                self.conservation.denom_in_raw,
                self.conservation.denom_out_raw,
            ),
            token_delta: scaled_signed_balance(
                self.conservation.token_in_raw,
                self.conservation.token_out_raw,
                self.token_decimals,
            ),
            denom_delta: scaled_signed_balance(
                self.conservation.denom_in_raw,
                self.conservation.denom_out_raw,
                self.denom_decimals,
            ),
            token_is_conserved: self.conservation.token_in_raw == self.conservation.token_out_raw,
            denom_is_conserved: self.conservation.denom_in_raw == self.conservation.denom_out_raw,
            token_transfer_count: self.conservation.token_transfer_count,
            denom_transfer_count: self.conservation.denom_transfer_count,
            pool_token_delta_raw: signed_raw_string(
                self.conservation.pool_token_in_raw,
                self.conservation.pool_token_out_raw,
            ),
            pool_denom_delta_raw: signed_raw_string(
                self.conservation.pool_denom_in_raw,
                self.conservation.pool_denom_out_raw,
            ),
            pool_token_delta: scaled_signed_balance(
                self.conservation.pool_token_in_raw,
                self.conservation.pool_token_out_raw,
                self.token_decimals,
            ),
            pool_denom_delta: scaled_signed_balance(
                self.conservation.pool_denom_in_raw,
                self.conservation.pool_denom_out_raw,
                self.denom_decimals,
            ),
        }
    }

    fn record_token_transfer(
        &mut self,
        from_address: &Address,
        to_address: &Address,
        amount: U256,
        log_index: Option<u64>,
        pool_direct: bool,
        transaction: &ProcessedTransaction,
    ) {
        let from = address_string(from_address);
        let to = address_string(to_address);
        self.position_mut(&from)
            .record_token_out(amount, transaction.block_number);
        self.position_mut(&to)
            .record_token_in(amount, transaction.block_number);
        self.conservation.token_transfer_count =
            self.conservation.token_transfer_count.saturating_add(1);
        self.conservation.token_in_raw = self.conservation.token_in_raw.saturating_add(amount);
        self.conservation.token_out_raw = self.conservation.token_out_raw.saturating_add(amount);

        if to == self.pool_address {
            self.conservation.pool_token_in_raw =
                self.conservation.pool_token_in_raw.saturating_add(amount);
        }
        if from == self.pool_address {
            self.conservation.pool_token_out_raw =
                self.conservation.pool_token_out_raw.saturating_add(amount);
        }

        self.push_entry(PoolPnlEntry::new(
            transaction,
            log_index,
            from,
            PoolPnlEntryKind::TokenOut,
            U256::ZERO,
            amount,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
        self.push_entry(PoolPnlEntry::new(
            transaction,
            log_index,
            to,
            PoolPnlEntryKind::TokenIn,
            amount,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
    }

    fn record_denom_transfer(
        &mut self,
        from_address: &Address,
        to_address: &Address,
        amount: U256,
        log_index: Option<u64>,
        pool_direct: bool,
        transaction: &ProcessedTransaction,
    ) {
        let from = address_string(from_address);
        let to = address_string(to_address);
        self.position_mut(&from)
            .record_denom_out(amount, transaction.block_number);
        self.position_mut(&to)
            .record_denom_in(amount, transaction.block_number);
        self.conservation.denom_transfer_count =
            self.conservation.denom_transfer_count.saturating_add(1);
        self.conservation.denom_in_raw = self.conservation.denom_in_raw.saturating_add(amount);
        self.conservation.denom_out_raw = self.conservation.denom_out_raw.saturating_add(amount);

        if to == self.pool_address {
            self.conservation.pool_denom_in_raw =
                self.conservation.pool_denom_in_raw.saturating_add(amount);
        }
        if from == self.pool_address {
            self.conservation.pool_denom_out_raw =
                self.conservation.pool_denom_out_raw.saturating_add(amount);
        }

        self.push_entry(PoolPnlEntry::new(
            transaction,
            log_index,
            from,
            PoolPnlEntryKind::DenomOut,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            amount,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
        self.push_entry(PoolPnlEntry::new(
            transaction,
            log_index,
            to,
            PoolPnlEntryKind::DenomIn,
            U256::ZERO,
            U256::ZERO,
            amount,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
    }

    fn record_native_denom_transfers(&mut self, transaction: &ProcessedTransaction) -> u32 {
        let mut transfer_count = 0u32;
        for transfer in &transaction.eth_transfers {
            if transfer.amount.is_zero() {
                continue;
            }
            self.record_native_denom_transfer(
                &transfer.from_address,
                &transfer.to_address,
                transfer.amount,
                transaction,
            );
            transfer_count = transfer_count.saturating_add(1);
        }

        for transfer in &transaction.internal_transactions {
            if transfer.value.is_zero() || transfer.error.is_some() {
                continue;
            }
            let Some(to_address) = transfer.to_address else {
                continue;
            };
            self.record_native_denom_transfer(
                &transfer.from_address,
                &to_address,
                transfer.value,
                transaction,
            );
            transfer_count = transfer_count.saturating_add(1);
        }

        transfer_count
    }

    fn record_native_denom_transfer(
        &mut self,
        from_address: &Address,
        to_address: &Address,
        amount: U256,
        transaction: &ProcessedTransaction,
    ) {
        let from = address_string(from_address);
        let to = address_string(to_address);
        self.position_mut(&from)
            .record_denom_out(amount, transaction.block_number);
        self.position_mut(&to)
            .record_denom_in(amount, transaction.block_number);
        self.conservation.denom_transfer_count =
            self.conservation.denom_transfer_count.saturating_add(1);
        self.conservation.denom_in_raw = self.conservation.denom_in_raw.saturating_add(amount);
        self.conservation.denom_out_raw = self.conservation.denom_out_raw.saturating_add(amount);

        if to == self.pool_address {
            self.conservation.pool_denom_in_raw =
                self.conservation.pool_denom_in_raw.saturating_add(amount);
        }
        if from == self.pool_address {
            self.conservation.pool_denom_out_raw =
                self.conservation.pool_denom_out_raw.saturating_add(amount);
        }

        let pool_direct = from == self.pool_address || to == self.pool_address;
        self.push_entry(PoolPnlEntry::new(
            transaction,
            None,
            from,
            PoolPnlEntryKind::NativeDenomOut,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            amount,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
        self.push_entry(PoolPnlEntry::new(
            transaction,
            None,
            to,
            PoolPnlEntryKind::NativeDenomIn,
            U256::ZERO,
            U256::ZERO,
            amount,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            pool_direct,
        ));
    }

    fn record_native_fee(
        &mut self,
        address: &Address,
        amount: U256,
        transaction: &ProcessedTransaction,
    ) {
        let address = address_string(address);
        self.position_mut(&address)
            .record_native_fee(amount, transaction.block_number);
        self.conservation.native_fee_raw = self.conservation.native_fee_raw.saturating_add(amount);
        self.push_entry(PoolPnlEntry::new(
            transaction,
            None,
            address,
            PoolPnlEntryKind::NativeFee,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            amount,
            U256::ZERO,
            false,
        ));
    }

    fn record_native_bribe(
        &mut self,
        address: &Address,
        amount: U256,
        transaction: &ProcessedTransaction,
    ) {
        let address = address_string(address);
        self.position_mut(&address)
            .record_native_bribe(amount, transaction.block_number);
        self.conservation.native_bribe_raw =
            self.conservation.native_bribe_raw.saturating_add(amount);
        self.push_entry(PoolPnlEntry::new(
            transaction,
            None,
            address,
            PoolPnlEntryKind::NativeBribe,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            U256::ZERO,
            amount,
            false,
        ));
    }

    fn position_mut(&mut self, address: &str) -> &mut AddressPoolPosition {
        let address = normalize_address(address);
        self.positions
            .entry(address.clone())
            .or_insert_with(|| AddressPoolPosition::new(address))
    }

    fn push_entry(&mut self, entry: PoolPnlEntry) {
        self.recent_entries.push(entry);
        self.prune_history();
    }

    fn prune_history(&mut self) {
        if self.history_limit == 0 || self.recent_entries.len() <= self.history_limit {
            return;
        }
        let excess = self.recent_entries.len() - self.history_limit;
        self.recent_entries.drain(0..excess);
    }

    fn is_pool_side(&self, address: &Address) -> bool {
        address_string(address) == self.pool_address
    }

    fn denom_tracks_native_eth(&self) -> bool {
        self.denom_address == WETH_ADDRESS
    }

    fn summary_for_position(
        &self,
        position: &AddressPoolPosition,
        mark_price_denom_per_token: Option<f64>,
    ) -> AddressPoolPnlSummary {
        let token_balance = scaled_signed_balance(
            position.token_in_raw,
            position.token_out_raw,
            self.token_decimals,
        );
        let denom_cashflow = scaled_signed_balance(
            position.denom_in_raw,
            position.denom_out_raw,
            self.denom_decimals,
        );
        let native_fee = scaled_units(position.native_fee_raw, 18);
        let native_bribe = scaled_units(position.native_bribe_raw, 18);
        let marked_token_value_denom =
            mark_price_denom_per_token.map(|price| token_balance * price);
        let native_costs = if self.denom_tracks_native_eth() {
            native_fee + native_bribe
        } else {
            0.0
        };
        let pnl_proxy_denom =
            marked_token_value_denom.map(|marked| denom_cashflow + marked - native_costs);

        AddressPoolPnlSummary {
            address: position.address.clone(),
            token_balance_raw: signed_raw_string(position.token_in_raw, position.token_out_raw),
            denom_cashflow_raw: signed_raw_string(position.denom_in_raw, position.denom_out_raw),
            native_fee_raw: position.native_fee_raw.to_string(),
            native_bribe_raw: position.native_bribe_raw.to_string(),
            token_balance,
            denom_cashflow,
            native_fee,
            native_bribe,
            marked_token_value_denom,
            pnl_proxy_denom,
            token_in_raw: position.token_in_raw.to_string(),
            token_out_raw: position.token_out_raw.to_string(),
            denom_in_raw: position.denom_in_raw.to_string(),
            denom_out_raw: position.denom_out_raw.to_string(),
            first_block: position.first_block,
            latest_block: position.latest_block,
            movement_count: position.movement_count,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlConservationTotals {
    pub token_in_raw: U256,
    pub token_out_raw: U256,
    pub denom_in_raw: U256,
    pub denom_out_raw: U256,
    pub pool_token_in_raw: U256,
    pub pool_token_out_raw: U256,
    pub pool_denom_in_raw: U256,
    pub pool_denom_out_raw: U256,
    pub native_fee_raw: U256,
    pub native_bribe_raw: U256,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlConservationSummary {
    pub token_delta_raw: String,
    pub denom_delta_raw: String,
    pub token_delta: f64,
    pub denom_delta: f64,
    pub token_is_conserved: bool,
    pub denom_is_conserved: bool,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
    pub pool_token_delta_raw: String,
    pub pool_denom_delta_raw: String,
    pub pool_token_delta: f64,
    pub pool_denom_delta: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressPoolPosition {
    pub address: String,
    pub token_in_raw: U256,
    pub token_out_raw: U256,
    pub denom_in_raw: U256,
    pub denom_out_raw: U256,
    pub native_fee_raw: U256,
    pub native_bribe_raw: U256,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
}

impl AddressPoolPosition {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: normalize_address_string(address.into()),
            token_in_raw: U256::ZERO,
            token_out_raw: U256::ZERO,
            denom_in_raw: U256::ZERO,
            denom_out_raw: U256::ZERO,
            native_fee_raw: U256::ZERO,
            native_bribe_raw: U256::ZERO,
            first_block: None,
            latest_block: None,
            movement_count: 0,
        }
    }

    fn record_token_in(&mut self, amount: U256, block_number: u64) {
        self.token_in_raw = self.token_in_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn record_token_out(&mut self, amount: U256, block_number: u64) {
        self.token_out_raw = self.token_out_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn record_denom_in(&mut self, amount: U256, block_number: u64) {
        self.denom_in_raw = self.denom_in_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn record_denom_out(&mut self, amount: U256, block_number: u64) {
        self.denom_out_raw = self.denom_out_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn record_native_fee(&mut self, amount: U256, block_number: u64) {
        self.native_fee_raw = self.native_fee_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn record_native_bribe(&mut self, amount: U256, block_number: u64) {
        self.native_bribe_raw = self.native_bribe_raw.saturating_add(amount);
        self.touch(block_number);
    }

    fn touch(&mut self, block_number: u64) {
        if self.first_block.is_none() {
            self.first_block = Some(block_number);
        }
        self.latest_block = Some(block_number);
        self.movement_count = self.movement_count.saturating_add(1);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressPoolPnlSummary {
    pub address: String,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub token_balance: f64,
    pub denom_cashflow: f64,
    pub native_fee: f64,
    pub native_bribe: f64,
    pub marked_token_value_denom: Option<f64>,
    pub pnl_proxy_denom: Option<f64>,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlEntry {
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub log_index: Option<u64>,
    pub address: String,
    pub kind: PoolPnlEntryKind,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub pool_direct: bool,
}

impl PoolPnlEntry {
    fn new(
        transaction: &ProcessedTransaction,
        log_index: Option<u64>,
        address: String,
        kind: PoolPnlEntryKind,
        token_in_raw: U256,
        token_out_raw: U256,
        denom_in_raw: U256,
        denom_out_raw: U256,
        native_fee_raw: U256,
        native_bribe_raw: U256,
        pool_direct: bool,
    ) -> Self {
        Self {
            tx_hash: hash_string(&transaction.hash),
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_index: transaction.tx_index,
            log_index,
            address,
            kind,
            token_in_raw: token_in_raw.to_string(),
            token_out_raw: token_out_raw.to_string(),
            denom_in_raw: denom_in_raw.to_string(),
            denom_out_raw: denom_out_raw.to_string(),
            native_fee_raw: native_fee_raw.to_string(),
            native_bribe_raw: native_bribe_raw.to_string(),
            pool_direct,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolPnlEntryKind {
    TokenIn,
    TokenOut,
    DenomIn,
    DenomOut,
    NativeDenomIn,
    NativeDenomOut,
    NativeFee,
    NativeBribe,
}

impl PoolPnlEntryKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TokenIn => "token_in",
            Self::TokenOut => "token_out",
            Self::DenomIn => "denom_in",
            Self::DenomOut => "denom_out",
            Self::NativeDenomIn => "native_denom_in",
            Self::NativeDenomOut => "native_denom_out",
            Self::NativeFee => "native_fee",
            Self::NativeBribe => "native_bribe",
        }
    }
}

fn signed_raw_string(incoming: U256, outgoing: U256) -> String {
    if incoming >= outgoing {
        (incoming - outgoing).to_string()
    } else {
        format!("-{}", outgoing - incoming)
    }
}

fn scaled_signed_balance(incoming: U256, outgoing: U256, decimals: u8) -> f64 {
    let amount = if incoming >= outgoing {
        scaled_units(incoming - outgoing, decimals)
    } else {
        -scaled_units(outgoing - incoming, decimals)
    };
    if amount == -0.0 {
        0.0
    } else {
        amount
    }
}

fn scaled_units(value: U256, decimals: u8) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(i32::from(decimals))
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn normalize_address_string(value: String) -> String {
    value.trim().to_ascii_lowercase()
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn hash_string(hash: &B256) -> String {
    format!("{hash:#x}")
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256};
    use tx_processor::tx_processor::data_models::trace_models::InternalTransaction;
    use tx_processor::tx_processor::data_models::ERC20TransferEvent;
    use tx_processor::tx_processor::data_models::TransactionFees;

    use super::*;

    const TOKEN: Address = address!("1111111111111111111111111111111111111111");
    const DENOM: Address = address!("2222222222222222222222222222222222222222");
    const WETH: Address = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    const POOL: Address = address!("3333333333333333333333333333333333333333");
    const FEE_PAYER: Address = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    const DENOM_PAYER: Address = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    const TOKEN_RECEIVER: Address = address!("cccccccccccccccccccccccccccccccccccccccc");

    #[test]
    fn records_split_actor_pool_trade_and_conservation() {
        let mut tracker = TokenPnlTracker::default();
        let mut tx = tx();
        tx.fees = TransactionFees::new(U256::from(10), 21_000, 21_000);
        tx.bribe_amount = U256::from(7);
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: DENOM,
            from_address: DENOM_PAYER,
            to_address: POOL,
            amount: U256::from(1_000),
            log_index: 1,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: TOKEN,
            from_address: POOL,
            to_address: TOKEN_RECEIVER,
            amount: U256::from(50_000),
            log_index: 2,
        });

        tracker.record_v2_pool_transaction(
            address_string(&POOL),
            address_string(&TOKEN),
            address_string(&DENOM),
            9,
            18,
            100,
            &tx,
        );

        let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
        let conservation = pool.conservation_summary();
        assert!(conservation.token_is_conserved);
        assert!(conservation.denom_is_conserved);
        assert_eq!(conservation.pool_token_delta_raw, "-50000");
        assert_eq!(conservation.pool_denom_delta_raw, "1000");

        let fee_payer = pool
            .position(address_string(&FEE_PAYER))
            .expect("fee payer");
        assert_eq!(fee_payer.native_fee_raw, U256::from(210_000));
        assert_eq!(fee_payer.native_bribe_raw, U256::from(7));

        let denom_payer = pool
            .position(address_string(&DENOM_PAYER))
            .expect("denom payer");
        assert_eq!(denom_payer.denom_out_raw, U256::from(1_000));
        assert_eq!(denom_payer.token_in_raw, U256::ZERO);

        let token_receiver = pool
            .position(address_string(&TOKEN_RECEIVER))
            .expect("token receiver");
        assert_eq!(token_receiver.token_in_raw, U256::from(50_000));
        assert_eq!(token_receiver.denom_out_raw, U256::ZERO);
    }

    #[test]
    fn keeps_transaction_scoped_router_movements_and_marks_pool_direct_entries() {
        let mut tracker = TokenPnlTracker::default();
        let router = address!("dddddddddddddddddddddddddddddddddddddddd");
        let mut tx = tx();
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: DENOM,
            from_address: DENOM_PAYER,
            to_address: router,
            amount: U256::from(1_000),
            log_index: 1,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: DENOM,
            from_address: router,
            to_address: POOL,
            amount: U256::from(1_000),
            log_index: 2,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: TOKEN,
            from_address: POOL,
            to_address: router,
            amount: U256::from(5_000),
            log_index: 3,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: TOKEN,
            from_address: router,
            to_address: TOKEN_RECEIVER,
            amount: U256::from(5_000),
            log_index: 4,
        });

        tracker.record_v2_pool_transaction(
            address_string(&POOL),
            address_string(&TOKEN),
            address_string(&DENOM),
            9,
            18,
            100,
            &tx,
        );

        let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
        assert_eq!(
            pool.position(address_string(&DENOM_PAYER))
                .expect("payer")
                .denom_out_raw,
            U256::from(1_000)
        );
        assert_eq!(
            pool.position(address_string(&TOKEN_RECEIVER))
                .expect("receiver")
                .token_in_raw,
            U256::from(5_000)
        );
        assert_eq!(
            pool.recent_entries
                .iter()
                .filter(|entry| entry.pool_direct)
                .count(),
            4
        );
        assert!(pool.conservation_summary().token_is_conserved);
        assert!(pool.conservation_summary().denom_is_conserved);
    }

    #[test]
    fn treats_native_eth_transfers_as_denom_for_weth_pools() {
        let mut tracker = TokenPnlTracker::default();
        let router = address!("dddddddddddddddddddddddddddddddddddddddd");
        let mut tx = tx();
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: WETH,
            from_address: POOL,
            to_address: router,
            amount: U256::from(1_000),
            log_index: 1,
        });
        tx.internal_transactions.push(InternalTransaction {
            from_address: router,
            to_address: Some(TOKEN_RECEIVER),
            value: U256::from(1_000),
            gas: 0,
            gas_used: 0,
            trace_type: "call".to_string(),
            call_type: Some("call".to_string()),
            depth: 1,
            error: None,
        });

        tracker.record_v2_pool_transaction(
            address_string(&POOL),
            address_string(&TOKEN),
            address_string(&WETH),
            9,
            18,
            100,
            &tx,
        );

        let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
        assert_eq!(
            signed_raw_string(
                pool.position(address_string(&router))
                    .expect("router")
                    .denom_in_raw,
                pool.position(address_string(&router))
                    .expect("router")
                    .denom_out_raw,
            ),
            "0"
        );
        assert_eq!(
            pool.position(address_string(&TOKEN_RECEIVER))
                .expect("receiver")
                .denom_in_raw,
            U256::from(1_000)
        );
        assert!(pool.conservation_summary().denom_is_conserved);
    }

    #[test]
    fn weth_pool_pnl_proxy_subtracts_native_fees_and_bribes() {
        let mut tracker = TokenPnlTracker::default();
        let mut tx = tx();
        tx.fees = TransactionFees::new(U256::from(10), 21_000, 21_000);
        tx.bribe_amount = U256::from(7);
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: WETH,
            from_address: FEE_PAYER,
            to_address: POOL,
            amount: U256::from(1_000_000_000_000_000_000_u64),
            log_index: 1,
        });

        tracker.record_v2_pool_transaction(
            address_string(&POOL),
            address_string(&TOKEN),
            address_string(&WETH),
            9,
            18,
            100,
            &tx,
        );

        let pool = tracker.pool(address_string(&POOL)).expect("pool pnl");
        let summary = pool
            .address_summaries(Some(0.0))
            .into_iter()
            .find(|summary| summary.address == address_string(&FEE_PAYER))
            .expect("fee payer summary");

        assert_eq!(summary.denom_cashflow, -1.0);
        assert!((summary.native_fee - 0.00000000000021).abs() < 1e-18);
        assert!((summary.native_bribe - 0.000000000000000007).abs() < 1e-21);
        assert!((summary.pnl_proxy_denom.expect("pnl proxy") - -1.00000000000021).abs() < 1e-15);
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0101010101010101010101010101010101010101010101010101010101010101"),
            100,
            1_700,
            1,
            FEE_PAYER,
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }
}
