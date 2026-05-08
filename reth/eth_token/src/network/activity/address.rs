//! Address-level activity counters.

use serde::{Deserialize, Serialize};

use crate::network::{
    activity::{
        movement::{AddressMovement, AddressMovementTotals},
        pnl::{AddressPnlInput, AddressPnlProxy},
    },
    model::{normalize_network_address, NetworkNodeId, NetworkObservation, ObservationRange},
};

pub const DEFAULT_ADDRESS_ACTIVITY_HISTORY_LIMIT: usize = 1000;

/// Transaction-level cost observed for an address.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressCostRecord {
    pub observation: NetworkObservation,
    pub tx_fee: f64,
    pub bribe_amount: f64,
}

impl AddressCostRecord {
    pub fn new(observation: NetworkObservation, tx_fee: f64, bribe_amount: f64) -> Self {
        Self {
            observation,
            tx_fee: finite_non_negative(tx_fee),
            bribe_amount: finite_non_negative(bribe_amount),
        }
    }
}

/// Serializable summary for one address in a token network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressActivitySummary {
    pub address: String,
    pub node_id: NetworkNodeId,
    pub entry_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub token_balance: f64,
    pub denom_balance: f64,
    pub total_token_bought: f64,
    pub total_token_sold: f64,
    pub total_denom_spent: f64,
    pub total_denom_received: f64,
    pub denom_received_spent_ratio: Option<f64>,
    pub token_sell_buy_ratio: Option<f64>,
    pub token_holdings_ratio: Option<f64>,
    pub token_holdings_to_total_supply_ratio: Option<f64>,
    pub num_buys: u64,
    pub num_sells: u64,
    pub num_tx: u64,
    pub mean_buy: Option<f64>,
    pub mean_sell: Option<f64>,
    pub bribe_amount: f64,
    pub total_tx_fees: f64,
    pub is_fee_source: bool,
    pub fee_source: Option<String>,
    pub pnl: AddressPnlProxy,
}

/// Per-address activity state for one tracked token.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressActivity {
    pub address: String,
    pub node_id: NetworkNodeId,
    pub history_limit: usize,
    pub observed: ObservationRange,
    pub is_fee_source: bool,
    pub fee_source: Option<String>,
    pub movements: Vec<AddressMovement>,
    pub costs: Vec<AddressCostRecord>,
    pub totals: AddressMovementTotals,
    pub total_tx_fees: f64,
    pub bribe_amount: f64,
}

impl AddressActivity {
    pub fn new(address: impl AsRef<str>) -> Self {
        Self::with_history_limit(address, DEFAULT_ADDRESS_ACTIVITY_HISTORY_LIMIT)
    }

    pub fn with_history_limit(address: impl AsRef<str>, history_limit: usize) -> Self {
        let address = normalize_network_address(address);
        Self {
            node_id: NetworkNodeId::address(&address),
            address,
            history_limit,
            observed: ObservationRange::default(),
            is_fee_source: false,
            fee_source: None,
            movements: Vec::new(),
            costs: Vec::new(),
            totals: AddressMovementTotals::default(),
            total_tx_fees: 0.0,
            bribe_amount: 0.0,
        }
    }

    pub fn mark_fee_source(&mut self) {
        self.is_fee_source = true;
    }

    pub fn set_fee_source(&mut self, fee_source: impl AsRef<str>) {
        self.fee_source = Some(normalize_network_address(fee_source));
    }

    pub fn record_movement(&mut self, movement: AddressMovement) {
        self.observed.record(movement.observation.clone());
        self.totals.record(&movement);
        append_with_history_limit(&mut self.movements, movement, self.history_limit);
    }

    pub fn record_cost(&mut self, cost: AddressCostRecord) {
        self.observed.record(cost.observation.clone());
        self.total_tx_fees += cost.tx_fee;
        self.bribe_amount += cost.bribe_amount;
        append_with_history_limit(&mut self.costs, cost, self.history_limit);
    }

    pub fn record_tx_fee(&mut self, observation: NetworkObservation, tx_fee: f64) {
        self.record_cost(AddressCostRecord::new(observation, tx_fee, 0.0));
    }

    pub fn record_bribe(&mut self, observation: NetworkObservation, bribe_amount: f64) {
        self.record_cost(AddressCostRecord::new(observation, 0.0, bribe_amount));
    }

    pub fn total_token_bought(&self) -> f64 {
        self.totals.token_in
    }

    pub fn total_token_sold(&self) -> f64 {
        self.totals.token_out
    }

    pub fn token_balance(&self) -> f64 {
        self.totals.token_balance()
    }

    pub fn total_denom_spent(&self) -> f64 {
        self.totals.denom_out
    }

    pub fn total_denom_received(&self) -> f64 {
        self.totals.denom_in
    }

    pub fn denom_balance(&self) -> f64 {
        self.totals.denom_balance()
    }

    pub fn num_buys(&self) -> u64 {
        self.totals.buy_count()
    }

    pub fn num_sells(&self) -> u64 {
        self.totals.sell_count()
    }

    pub fn num_tx_proxy(&self) -> u64 {
        self.totals.tx_count_proxy()
    }

    pub fn mean_buy(&self) -> Option<f64> {
        ratio_if_positive(self.total_denom_spent(), self.num_buys() as f64)
    }

    pub fn mean_sell(&self) -> Option<f64> {
        ratio_if_positive(self.total_denom_received(), self.num_sells() as f64)
    }

    pub fn denom_received_spent_ratio(&self) -> Option<f64> {
        ratio_if_positive(self.total_denom_received(), self.total_denom_spent())
    }

    pub fn token_sell_buy_ratio(&self) -> Option<f64> {
        ratio_if_positive(self.total_token_sold(), self.total_token_bought())
    }

    pub fn token_holdings_ratio(&self) -> Option<f64> {
        ratio_if_positive(self.token_balance(), self.total_token_bought())
    }

    pub fn token_holdings_to_total_supply_ratio(&self, total_supply: Option<f64>) -> Option<f64> {
        ratio_if_positive(self.token_balance(), total_supply?)
    }

    pub fn pnl_proxy(&self, token_latest_price: Option<f64>) -> AddressPnlProxy {
        AddressPnlProxy::from_input(AddressPnlInput {
            token_balance: self.token_balance(),
            token_latest_price,
            total_denom_spent: self.total_denom_spent(),
            total_denom_received: self.total_denom_received(),
            bribe_amount: self.bribe_amount,
            tx_fee_amount: self.total_tx_fees,
        })
    }

    pub fn summary(
        &self,
        token_latest_price: Option<f64>,
        total_supply: Option<f64>,
    ) -> AddressActivitySummary {
        AddressActivitySummary {
            address: self.address.clone(),
            node_id: self.node_id.clone(),
            entry_block: self
                .observed
                .first_seen
                .as_ref()
                .map(|observation| observation.block_number),
            latest_block: self
                .observed
                .last_seen
                .as_ref()
                .map(|observation| observation.block_number),
            token_balance: self.token_balance(),
            denom_balance: self.denom_balance(),
            total_token_bought: self.total_token_bought(),
            total_token_sold: self.total_token_sold(),
            total_denom_spent: self.total_denom_spent(),
            total_denom_received: self.total_denom_received(),
            denom_received_spent_ratio: self.denom_received_spent_ratio(),
            token_sell_buy_ratio: self.token_sell_buy_ratio(),
            token_holdings_ratio: self.token_holdings_ratio(),
            token_holdings_to_total_supply_ratio: self
                .token_holdings_to_total_supply_ratio(total_supply),
            num_buys: self.num_buys(),
            num_sells: self.num_sells(),
            num_tx: self.num_tx_proxy(),
            mean_buy: self.mean_buy(),
            mean_sell: self.mean_sell(),
            bribe_amount: self.bribe_amount,
            total_tx_fees: self.total_tx_fees,
            is_fee_source: self.is_fee_source,
            fee_source: self.fee_source.clone(),
            pnl: self.pnl_proxy(token_latest_price),
        }
    }
}

fn append_with_history_limit<T>(items: &mut Vec<T>, entry: T, limit: usize) {
    items.push(entry);
    if limit == 0 {
        items.clear();
    } else if items.len() > limit {
        let excess = items.len() - limit;
        items.drain(0..excess);
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn ratio_if_positive(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator.is_finite() && denominator > 0.0 && numerator.is_finite() {
        Some(numerator / denominator)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::activity::movement::AddressMovement;

    fn obs(block_number: u64) -> NetworkObservation {
        NetworkObservation::new(block_number)
    }

    #[test]
    fn address_activity_tracks_python_style_features() {
        let mut activity = AddressActivity::new("0xABC");
        activity.mark_fee_source();
        activity.record_movement(AddressMovement::token_in(100.0, obs(10)));
        activity.record_movement(AddressMovement::denom_out(1.0, obs(10)));
        activity.record_movement(AddressMovement::token_out(25.0, obs(11)));
        activity.record_movement(AddressMovement::denom_in(0.5, obs(11)));
        activity.record_tx_fee(obs(10), 0.01);
        activity.record_bribe(obs(10), 0.02);

        let summary = activity.summary(Some(0.02), Some(1_000.0));

        assert_eq!(summary.address, "0xabc");
        assert_eq!(summary.entry_block, Some(10));
        assert_eq!(summary.latest_block, Some(11));
        assert_eq!(summary.token_balance, 75.0);
        assert_eq!(summary.denom_balance, -0.5);
        assert_eq!(summary.num_buys, 1);
        assert_eq!(summary.num_sells, 1);
        assert_eq!(summary.total_tx_fees, 0.01);
        assert_eq!(summary.bribe_amount, 0.02);
        assert_eq!(summary.pnl.realized_profit, -0.53);
        assert_eq!(summary.pnl.unrealized_profit, 1.5);
        assert_eq!(summary.token_holdings_to_total_supply_ratio, Some(0.075));
    }

    #[test]
    fn history_limit_bounds_movements_and_costs() {
        let mut activity = AddressActivity::with_history_limit("0xABC", 1);
        activity.record_movement(AddressMovement::token_in(1.0, obs(1)));
        activity.record_movement(AddressMovement::token_in(2.0, obs(2)));
        activity.record_tx_fee(obs(1), 0.1);
        activity.record_tx_fee(obs(2), 0.2);

        assert_eq!(activity.movements.len(), 1);
        assert_eq!(activity.movements[0].amount, 2.0);
        assert_eq!(activity.costs.len(), 1);
        assert_eq!(activity.costs[0].tx_fee, 0.2);
        assert_eq!(activity.total_token_bought(), 3.0);
        assert_eq!(activity.total_tx_fees, 0.30000000000000004);
    }
}
