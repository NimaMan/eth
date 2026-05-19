use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};

use super::gwei_per_eth;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityFeeBudgetInput {
    /// Expected ETH value if the priority sell lands before the scam path.
    pub protected_exit_value_eth: DecimalAmount,
    /// Expected ETH value if we are late and only recover dust.
    pub expected_late_recovery_eth: DecimalAmount,
    /// Conservative buffer for slippage, stale quotes, and model error.
    pub safety_buffer_eth: DecimalAmount,
    /// Predicted next-block base fee. This is mandatory fee, not bribe, but it
    /// still consumes the trade's total allowed execution cost.
    pub predicted_base_fee_gwei: DecimalAmount,
    pub estimated_gas_used: u64,
    /// Operator hard cap for total gas cost on this exit.
    pub max_total_fee_eth: DecimalAmount,
    /// Operator hard cap for priority fee per gas.
    pub configured_max_priority_fee_gwei: DecimalAmount,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityFeeBudget {
    pub avoidable_loss_eth: DecimalAmount,
    pub max_total_fee_eth: DecimalAmount,
    pub predicted_base_fee_gwei: DecimalAmount,
    pub estimated_base_fee_cost_eth: DecimalAmount,
    pub max_priority_spend_eth: DecimalAmount,
    pub max_priority_fee_gwei: DecimalAmount,
    pub max_fee_per_gas_gwei: DecimalAmount,
    pub estimated_gas_used: u64,
}

impl PriorityFeeBudget {
    pub fn from_input(input: &PriorityFeeBudgetInput) -> Self {
        let avoidable_loss_eth = positive_or_zero(
            input.protected_exit_value_eth
                - input.expected_late_recovery_eth
                - input.safety_buffer_eth,
        );
        let max_total_fee_eth = min_decimal(input.max_total_fee_eth, avoidable_loss_eth);
        let estimated_base_fee_cost_eth =
            estimate_eth_cost_from_gwei(input.predicted_base_fee_gwei, input.estimated_gas_used);
        let max_priority_spend_eth =
            positive_or_zero(max_total_fee_eth - estimated_base_fee_cost_eth);
        let value_capped_priority_fee_gwei =
            priority_fee_gwei_for_spend(max_priority_spend_eth, input.estimated_gas_used);
        let max_priority_fee_gwei = min_decimal(
            value_capped_priority_fee_gwei,
            input.configured_max_priority_fee_gwei,
        );
        let max_fee_per_gas_gwei = input.predicted_base_fee_gwei + max_priority_fee_gwei;

        Self {
            avoidable_loss_eth,
            max_total_fee_eth,
            predicted_base_fee_gwei: input.predicted_base_fee_gwei,
            estimated_base_fee_cost_eth,
            max_priority_spend_eth,
            max_priority_fee_gwei,
            max_fee_per_gas_gwei,
            estimated_gas_used: input.estimated_gas_used,
        }
    }

    pub fn allows_priority_fee(&self, priority_fee_gwei: DecimalAmount, gas_used: u64) -> bool {
        if priority_fee_gwei > self.max_priority_fee_gwei {
            return false;
        }

        estimate_eth_cost_from_gwei(priority_fee_gwei, gas_used) <= self.max_priority_spend_eth
    }

    pub fn allows_total_fee(&self, max_fee_per_gas_gwei: DecimalAmount, gas_used: u64) -> bool {
        estimate_eth_cost_from_gwei(max_fee_per_gas_gwei, gas_used) <= self.max_total_fee_eth
    }
}

pub fn estimate_eth_cost_from_gwei(fee_gwei: DecimalAmount, gas_used: u64) -> DecimalAmount {
    fee_gwei * DecimalAmount::from(gas_used) / gwei_per_eth()
}

fn priority_fee_gwei_for_spend(spend_eth: DecimalAmount, gas_used: u64) -> DecimalAmount {
    if gas_used == 0 {
        return DecimalAmount::ZERO;
    }

    spend_eth * gwei_per_eth() / DecimalAmount::from(gas_used)
}

pub(crate) fn positive_or_zero(value: DecimalAmount) -> DecimalAmount {
    value.max(DecimalAmount::ZERO)
}

pub(crate) fn min_decimal(left: DecimalAmount, right: DecimalAmount) -> DecimalAmount {
    if left <= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_priority_spend_by_avoidable_loss() {
        let budget = PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
            protected_exit_value_eth: DecimalAmount::new(10, 3),
            expected_late_recovery_eth: DecimalAmount::new(5, 3),
            safety_buffer_eth: DecimalAmount::new(1, 3),
            predicted_base_fee_gwei: DecimalAmount::from(10),
            estimated_gas_used: 100_000,
            max_total_fee_eth: DecimalAmount::new(2, 2),
            configured_max_priority_fee_gwei: DecimalAmount::from(100_000),
        });

        assert_eq!(budget.avoidable_loss_eth, DecimalAmount::new(4, 3));
        assert_eq!(budget.estimated_base_fee_cost_eth, DecimalAmount::new(1, 3));
        assert_eq!(budget.max_priority_spend_eth, DecimalAmount::new(3, 3));
        assert_eq!(budget.max_priority_fee_gwei, DecimalAmount::from(30));
    }

    #[test]
    fn zeroes_priority_budget_when_base_fee_exhausts_value_cap() {
        let budget = PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
            protected_exit_value_eth: DecimalAmount::new(10, 4),
            expected_late_recovery_eth: DecimalAmount::ZERO,
            safety_buffer_eth: DecimalAmount::ZERO,
            predicted_base_fee_gwei: DecimalAmount::from(50),
            estimated_gas_used: 100_000,
            max_total_fee_eth: DecimalAmount::new(10, 4),
            configured_max_priority_fee_gwei: DecimalAmount::from(100),
        });

        assert_eq!(budget.max_priority_spend_eth, DecimalAmount::ZERO);
        assert_eq!(budget.max_priority_fee_gwei, DecimalAmount::ZERO);
    }
}
