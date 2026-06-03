use std::collections::BTreeMap;

use alloy_primitives::{Address, U256};
use tx_processor::ProcessedTransaction;

use crate::pnl::common::{address_string, parse_address, ZERO_ADDRESS};
use crate::pnl::tx_ledger::{
    AddressReconciliationSummary, AssetFamily, ReconciledTxLedger, TxAsset, TxLedger,
    TxLedgerContext, TxMovement,
};

use super::{AddressPoolPosition, PoolPnlEntry, PoolPnlEntryKind, PoolPnlTracker};

#[derive(Default)]
struct AddressPoolTxDelta {
    token_in: U256,
    token_out: U256,
    denom_in: U256,
    denom_out: U256,
    native_fee: U256,
    native_priority_fee: U256,
    movement_count: u64,
    entries: Vec<PoolPnlEntry>,
}

impl AddressPoolTxDelta {
    fn net_token_abs(&self) -> U256 {
        abs_delta(self.token_in, self.token_out)
    }

    fn net_denom_abs(&self) -> U256 {
        abs_delta(self.denom_in, self.denom_out)
    }

    fn has_position_signal(&self) -> bool {
        !self.net_token_abs().is_zero() || !self.net_denom_abs().is_zero()
    }

    fn apply_to_position(&self, position: &mut AddressPoolPosition, block_number: u64) {
        if !self.token_in.is_zero() {
            position.record_token_in(self.token_in, block_number);
        }
        if !self.token_out.is_zero() {
            position.record_token_out(self.token_out, block_number);
        }
        if !self.denom_in.is_zero() {
            position.record_denom_in(self.denom_in, block_number);
        }
        if !self.denom_out.is_zero() {
            position.record_denom_out(self.denom_out, block_number);
        }
        if !self.native_fee.is_zero() {
            position.record_native_fee(self.native_fee, block_number);
        }
        if !self.native_priority_fee.is_zero() {
            position.record_native_priority_fee(self.native_priority_fee, block_number);
        }

        let recorded_by_helpers = nonzero_count([
            self.token_in,
            self.token_out,
            self.denom_in,
            self.denom_out,
            self.native_fee,
            self.native_priority_fee,
        ]);
        if self.movement_count > recorded_by_helpers {
            position.movement_count = position
                .movement_count
                .saturating_add(self.movement_count - recorded_by_helpers);
        }
    }
}

impl PoolPnlTracker {
    pub fn record_transaction(&mut self, transaction: &ProcessedTransaction) {
        let Some(context) = self.ledger_context() else {
            return;
        };
        let ledger = TxLedger::from_processed_transaction(context, transaction);
        let reconciled = ledger.reconcile();
        let mut deltas: BTreeMap<String, AddressPoolTxDelta> = BTreeMap::new();
        let mut matched_movement_count = 0u64;

        for movement in &ledger.movements {
            if self.record_ledger_movement(&mut deltas, context, movement, transaction) {
                matched_movement_count = matched_movement_count.saturating_add(1);
            }
        }

        if matched_movement_count == 0
            && transaction.fees.tx_fee.is_zero()
            && transaction.bribe_amount.is_zero()
        {
            return;
        }

        self.record_native_costs(&mut deltas, transaction);
        self.commit_ledger_deltas(deltas, &reconciled, transaction.block_number);
        self.tx_count = self.tx_count.saturating_add(1);
        self.latest_block_number = Some(transaction.block_number);
        self.latest_block_timestamp = Some(transaction.block_timestamp);
    }

    fn ledger_context(&self) -> Option<TxLedgerContext> {
        Some(TxLedgerContext::new(
            parse_address(&self.token_address)?,
            parse_address(&self.denom_address)?,
        ))
    }

    fn record_ledger_movement(
        &mut self,
        deltas: &mut BTreeMap<String, AddressPoolTxDelta>,
        context: TxLedgerContext,
        movement: &TxMovement,
        transaction: &ProcessedTransaction,
    ) -> bool {
        let family = context.asset_family(movement.asset);
        let Some((out_kind, in_kind)) = entry_kinds(family, movement.asset) else {
            return false;
        };

        self.record_conservation(family, movement);

        let pool_direct =
            self.is_pool_address(&movement.from) || self.is_pool_address(&movement.to);
        let log_index = movement.source.log_index;
        let from = address_string(&movement.from);
        let to = address_string(&movement.to);

        {
            let delta = deltas.entry(from.clone()).or_default();
            delta.movement_count = delta.movement_count.saturating_add(1);
            match family {
                AssetFamily::Token => {
                    delta.token_out = delta.token_out.saturating_add(movement.amount)
                }
                AssetFamily::Denom => {
                    delta.denom_out = delta.denom_out.saturating_add(movement.amount)
                }
                _ => {}
            }
            delta.entries.push(PoolPnlEntry::new(
                transaction,
                log_index,
                from,
                out_kind,
                U256::ZERO,
                token_out_raw(family, movement.amount),
                U256::ZERO,
                denom_out_raw(family, movement.amount),
                U256::ZERO,
                U256::ZERO,
                pool_direct,
            ));
        }

        {
            let delta = deltas.entry(to.clone()).or_default();
            delta.movement_count = delta.movement_count.saturating_add(1);
            match family {
                AssetFamily::Token => {
                    delta.token_in = delta.token_in.saturating_add(movement.amount)
                }
                AssetFamily::Denom => {
                    delta.denom_in = delta.denom_in.saturating_add(movement.amount)
                }
                _ => {}
            }
            delta.entries.push(PoolPnlEntry::new(
                transaction,
                log_index,
                to,
                in_kind,
                token_in_raw(family, movement.amount),
                U256::ZERO,
                denom_in_raw(family, movement.amount),
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                pool_direct,
            ));
        }

        true
    }

    fn record_conservation(&mut self, family: AssetFamily, movement: &TxMovement) {
        match family {
            AssetFamily::Token => {
                self.conservation.token_transfer_count =
                    self.conservation.token_transfer_count.saturating_add(1);
                self.conservation.token_in_raw = self
                    .conservation
                    .token_in_raw
                    .saturating_add(movement.amount);
                self.conservation.token_out_raw = self
                    .conservation
                    .token_out_raw
                    .saturating_add(movement.amount);
                if self.is_pool_address(&movement.to) {
                    self.conservation.pool_token_in_raw = self
                        .conservation
                        .pool_token_in_raw
                        .saturating_add(movement.amount);
                }
                if self.is_pool_address(&movement.from) {
                    self.conservation.pool_token_out_raw = self
                        .conservation
                        .pool_token_out_raw
                        .saturating_add(movement.amount);
                }
            }
            AssetFamily::Denom => {
                self.conservation.denom_transfer_count =
                    self.conservation.denom_transfer_count.saturating_add(1);
                self.conservation.denom_in_raw = self
                    .conservation
                    .denom_in_raw
                    .saturating_add(movement.amount);
                self.conservation.denom_out_raw = self
                    .conservation
                    .denom_out_raw
                    .saturating_add(movement.amount);
                if self.is_pool_address(&movement.to) {
                    self.conservation.pool_denom_in_raw = self
                        .conservation
                        .pool_denom_in_raw
                        .saturating_add(movement.amount);
                }
                if self.is_pool_address(&movement.from) {
                    self.conservation.pool_denom_out_raw = self
                        .conservation
                        .pool_denom_out_raw
                        .saturating_add(movement.amount);
                }
            }
            _ => {}
        }
    }

    fn record_native_costs(
        &mut self,
        deltas: &mut BTreeMap<String, AddressPoolTxDelta>,
        transaction: &ProcessedTransaction,
    ) {
        let address = address_string(&transaction.from_address);
        if !transaction.fees.tx_fee.is_zero() {
            self.conservation.native_fee_raw = self
                .conservation
                .native_fee_raw
                .saturating_add(transaction.fees.tx_fee);
            let delta = deltas.entry(address.clone()).or_default();
            delta.native_fee = delta.native_fee.saturating_add(transaction.fees.tx_fee);
            delta.movement_count = delta.movement_count.saturating_add(1);
            delta.entries.push(PoolPnlEntry::new(
                transaction,
                None,
                address.clone(),
                PoolPnlEntryKind::NativeFee,
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                transaction.fees.tx_fee,
                U256::ZERO,
                false,
            ));
        }
        if !transaction.bribe_amount.is_zero() {
            self.conservation.native_priority_fee_raw = self
                .conservation
                .native_priority_fee_raw
                .saturating_add(transaction.bribe_amount);
            let delta = deltas.entry(address.clone()).or_default();
            delta.native_priority_fee = delta
                .native_priority_fee
                .saturating_add(transaction.bribe_amount);
            delta.movement_count = delta.movement_count.saturating_add(1);
            delta.entries.push(PoolPnlEntry::new(
                transaction,
                None,
                address,
                PoolPnlEntryKind::NativePriorityFee,
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                U256::ZERO,
                transaction.bribe_amount,
                false,
            ));
        }
    }

    fn commit_ledger_deltas(
        &mut self,
        deltas: BTreeMap<String, AddressPoolTxDelta>,
        reconciled: &ReconciledTxLedger,
        block_number: u64,
    ) {
        for (address, delta) in deltas {
            if self.should_exclude_address(&address) {
                continue;
            }
            if !delta.has_position_signal() {
                continue;
            }
            if reconciled_summary(&address, reconciled)
                .map(AddressReconciliationSummary::is_pass_through)
                .unwrap_or(false)
            {
                continue;
            }

            let position = self
                .positions
                .entry(address.clone())
                .or_insert_with(|| AddressPoolPosition::new(address));
            delta.apply_to_position(position, block_number);
            for entry in delta.entries {
                self.push_entry(entry);
            }
        }
    }

    fn should_exclude_address(&self, address: &str) -> bool {
        address == self.pool_address
            || address == self.token_address
            || address == self.denom_address
            || address == ZERO_ADDRESS
    }

    fn is_pool_address(&self, address: &Address) -> bool {
        address_string(address) == self.pool_address
    }
}

fn entry_kinds(
    family: AssetFamily,
    asset: TxAsset,
) -> Option<(PoolPnlEntryKind, PoolPnlEntryKind)> {
    match family {
        AssetFamily::Token => Some((PoolPnlEntryKind::TokenOut, PoolPnlEntryKind::TokenIn)),
        AssetFamily::Denom if asset == TxAsset::native_eth() => Some((
            PoolPnlEntryKind::NativeDenomOut,
            PoolPnlEntryKind::NativeDenomIn,
        )),
        AssetFamily::Denom => Some((PoolPnlEntryKind::DenomOut, PoolPnlEntryKind::DenomIn)),
        _ => None,
    }
}

fn reconciled_summary<'a>(
    address: &str,
    reconciled: &'a ReconciledTxLedger,
) -> Option<&'a AddressReconciliationSummary> {
    parse_address(address).and_then(|address| reconciled.address(&address))
}

fn abs_delta(incoming: U256, outgoing: U256) -> U256 {
    if incoming >= outgoing {
        incoming - outgoing
    } else {
        outgoing - incoming
    }
}

fn nonzero_count(values: [U256; 6]) -> u64 {
    values.into_iter().filter(|value| !value.is_zero()).count() as u64
}

const fn token_in_raw(family: AssetFamily, amount: U256) -> U256 {
    match family {
        AssetFamily::Token => amount,
        _ => U256::ZERO,
    }
}

const fn token_out_raw(family: AssetFamily, amount: U256) -> U256 {
    match family {
        AssetFamily::Token => amount,
        _ => U256::ZERO,
    }
}

const fn denom_in_raw(family: AssetFamily, amount: U256) -> U256 {
    match family {
        AssetFamily::Denom => amount,
        _ => U256::ZERO,
    }
}

const fn denom_out_raw(family: AssetFamily, amount: U256) -> U256 {
    match family {
        AssetFamily::Denom => amount,
        _ => U256::ZERO,
    }
}
