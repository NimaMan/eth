use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{TokenBlockActivity, TokenTransactionActivity};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenActivityTracker {
    pub transactions_by_hash: BTreeMap<String, TokenTransactionActivity>,
    pub blocks: BTreeMap<u64, TokenBlockActivity>,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
}

impl TokenActivityTracker {
    pub fn record_transaction(
        &mut self,
        tx_hash: impl AsRef<str>,
        maker: Option<&str>,
        block_number: u64,
        timestamp: Option<u64>,
    ) {
        let tx_hash = normalize_key(tx_hash);
        let maker = maker.map(normalize_key);
        let inserted = !self.transactions_by_hash.contains_key(&tx_hash);

        self.transactions_by_hash
            .entry(tx_hash.clone())
            .and_modify(|activity| activity.refresh_context(timestamp, maker.clone()))
            .or_insert_with(|| {
                TokenTransactionActivity::new(tx_hash, block_number, timestamp, maker)
            });

        let block = self.block_mut(block_number, timestamp);
        if inserted {
            block.num_tx = block.num_tx.saturating_add(1);
        }
        self.record_latest(block_number, timestamp);
    }

    pub fn record_buy(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        denom: impl AsRef<str>,
        amount: f64,
    ) {
        self.record_volume(
            tx_hash,
            block_number,
            timestamp,
            denom,
            amount,
            VolumeSide::Buy,
        );
    }

    pub fn record_sell(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        denom: impl AsRef<str>,
        amount: f64,
    ) {
        self.record_volume(
            tx_hash,
            block_number,
            timestamp,
            denom,
            amount,
            VolumeSide::Sell,
        );
    }

    pub fn record_bribe_eth(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        amount_eth: f64,
    ) {
        let amount_eth = finite_non_negative(amount_eth);
        if amount_eth == 0.0 {
            return;
        }

        self.record_transaction(tx_hash.as_ref(), None, block_number, timestamp);

        let tx_hash = normalize_key(tx_hash);
        let delta = if let Some(tx) = self.transactions_by_hash.get_mut(&tx_hash) {
            let delta = (amount_eth - tx.total_bribe_eth).max(0.0);
            tx.total_bribe_eth += delta;
            delta
        } else {
            0.0
        };
        if delta == 0.0 {
            return;
        }
        self.block_mut(block_number, timestamp).total_bribe_eth += delta;
        self.record_latest(block_number, timestamp);
    }

    pub fn record_token_transfer(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        self.record_transaction(tx_hash.as_ref(), None, block_number, timestamp);
        let tx_hash = normalize_key(tx_hash);
        if let Some(tx) = self.transactions_by_hash.get_mut(&tx_hash) {
            tx.token_transfer_count = tx.token_transfer_count.saturating_add(count);
        }
        self.block_mut(block_number, timestamp).token_transfer_count = self
            .block_mut(block_number, timestamp)
            .token_transfer_count
            .saturating_add(count);
        self.record_latest(block_number, timestamp);
    }

    pub fn record_denom_transfer(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        self.record_transaction(tx_hash.as_ref(), None, block_number, timestamp);
        let tx_hash = normalize_key(tx_hash);
        if let Some(tx) = self.transactions_by_hash.get_mut(&tx_hash) {
            tx.denom_transfer_count = tx.denom_transfer_count.saturating_add(count);
        }
        self.block_mut(block_number, timestamp).denom_transfer_count = self
            .block_mut(block_number, timestamp)
            .denom_transfer_count
            .saturating_add(count);
        self.record_latest(block_number, timestamp);
    }

    pub fn recent_blocks(&self, limit: usize) -> Vec<TokenBlockActivity> {
        if limit == 0 {
            return Vec::new();
        }

        let mut blocks = self
            .blocks
            .values()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        blocks.reverse();
        blocks
    }

    pub fn total_buy_volume_by_denom(&self) -> BTreeMap<String, f64> {
        self.aggregate_volume(|block| &block.buy_volume_by_denom)
    }

    pub fn total_sell_volume_by_denom(&self) -> BTreeMap<String, f64> {
        self.aggregate_volume(|block| &block.sell_volume_by_denom)
    }

    pub fn total_bribe_eth(&self) -> f64 {
        self.blocks
            .values()
            .map(|block| block.total_bribe_eth)
            .sum()
    }

    fn record_volume(
        &mut self,
        tx_hash: impl AsRef<str>,
        block_number: u64,
        timestamp: Option<u64>,
        denom: impl AsRef<str>,
        amount: f64,
        side: VolumeSide,
    ) {
        let amount = finite_non_negative(amount);
        if amount == 0.0 {
            return;
        }

        self.record_transaction(tx_hash.as_ref(), None, block_number, timestamp);

        let tx_hash = normalize_key(tx_hash);
        let denom = normalize_key(denom);
        match side {
            VolumeSide::Buy => {
                {
                    let tx = self
                        .transactions_by_hash
                        .get_mut(&tx_hash)
                        .expect("transaction was recorded");
                    *tx.buy_volume_by_denom.entry(denom.clone()).or_insert(0.0) += amount;
                }
                *self
                    .block_mut(block_number, timestamp)
                    .buy_volume_by_denom
                    .entry(denom)
                    .or_insert(0.0) += amount;
            }
            VolumeSide::Sell => {
                {
                    let tx = self
                        .transactions_by_hash
                        .get_mut(&tx_hash)
                        .expect("transaction was recorded");
                    *tx.sell_volume_by_denom.entry(denom.clone()).or_insert(0.0) += amount;
                }
                *self
                    .block_mut(block_number, timestamp)
                    .sell_volume_by_denom
                    .entry(denom)
                    .or_insert(0.0) += amount;
            }
        }
        self.record_latest(block_number, timestamp);
    }

    fn block_mut(&mut self, block_number: u64, timestamp: Option<u64>) -> &mut TokenBlockActivity {
        self.blocks
            .entry(block_number)
            .and_modify(|block| block.refresh_timestamp(timestamp))
            .or_insert_with(|| TokenBlockActivity::new(block_number, timestamp))
    }

    fn record_latest(&mut self, block_number: u64, timestamp: Option<u64>) {
        if self
            .latest_block_number
            .map_or(true, |latest| block_number >= latest)
        {
            self.latest_block_number = Some(block_number);
            if timestamp.is_some() {
                self.latest_block_timestamp = timestamp;
            }
        }
    }

    fn aggregate_volume<F>(&self, selector: F) -> BTreeMap<String, f64>
    where
        F: Fn(&TokenBlockActivity) -> &BTreeMap<String, f64>,
    {
        let mut totals = BTreeMap::new();
        for block in self.blocks.values() {
            for (denom, amount) in selector(block) {
                *totals.entry(denom.clone()).or_insert(0.0) += amount;
            }
        }
        totals
    }
}

#[derive(Clone, Copy)]
enum VolumeSide {
    Buy,
    Sell,
}

fn normalize_key(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_num_tx_once_per_hash_per_block() {
        let mut tracker = TokenActivityTracker::default();

        tracker.record_transaction("0xA", Some("0xMaker"), 10, Some(1000));
        tracker.record_transaction("0xa", Some("0xMaker"), 10, Some(1000));
        tracker.record_buy("0xA", 10, Some(1000), "ETH", 1.5);

        let block = tracker.blocks.get(&10).unwrap();
        assert_eq!(block.num_tx, 1);
        assert_eq!(block.buy_volume_by_denom["eth"], 1.5);
    }

    #[test]
    fn records_eth_bribe_on_block_and_transaction() {
        let mut tracker = TokenActivityTracker::default();

        tracker.record_bribe_eth("0xA", 11, Some(1001), 0.02);

        assert_eq!(tracker.blocks[&11].total_bribe_eth, 0.02);
        assert_eq!(tracker.transactions_by_hash["0xa"].total_bribe_eth, 0.02);
        assert_eq!(tracker.total_bribe_eth(), 0.02);
    }
}
