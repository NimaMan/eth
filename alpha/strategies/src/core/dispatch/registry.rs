use super::spec::{LiveStrategySpec, LiveStrategySpecOptions};
use super::strategy_sets;
use crate::alpha11::live::specs as alpha11_specs;

pub fn strategy_set_specs(
    set_name: &str,
    options: &LiveStrategySpecOptions,
) -> Result<Vec<LiveStrategySpec>, String> {
    match set_name {
        strategy_sets::mempool_exit_diagnostics::SET_NAME => {
            Ok(strategy_sets::mempool_exit_diagnostics::specs(options))
        }
        crate::core::rules::lp_approval_warning_exit::SUITE_NAME
        | crate::core::rules::lp_approval_warning_exit::STRATEGY_NAME => {
            Ok(vec![strategy_sets::lp_approval_warning::spec(options)])
        }
        strategy_sets::risk_atlas::LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME => Ok(vec![
            strategy_sets::risk_atlas::lp_gate_hold15_buy_confirm_lp_maxhold_spec(options),
        ]),
        alpha11_specs::SET_NAME => Ok(alpha11_specs::specs(options)),
        crate::alpha11::HOLD15_STRATEGY_NAME => Ok(vec![alpha11_specs::hold15_spec(options)]),
        crate::alpha11::HOLD16_STRATEGY_NAME => Ok(vec![alpha11_specs::hold16_spec(options)]),
        crate::alpha11::HOLD16_ALL_POOLS_STRATEGY_NAME => {
            Ok(vec![alpha11_specs::hold16_all_pools_spec(options)])
        }
        crate::alpha11::HOLD3_VALIDATION_STRATEGY_NAME => {
            Ok(vec![alpha11_specs::hold3_validation_spec(options)])
        }
        other => Err(format!("unsupported strategy set: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_known_strategy_sets() {
        let options = LiveStrategySpecOptions::default();

        assert_eq!(
            strategy_set_specs(strategy_sets::mempool_exit_diagnostics::SET_NAME, &options)
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            strategy_set_specs(
                crate::core::rules::lp_approval_warning_exit::SUITE_NAME,
                &options
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            strategy_set_specs(
                strategy_sets::risk_atlas::LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME,
                &options
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            strategy_set_specs(alpha11_specs::SET_NAME, &options)
                .unwrap()
                .len(),
            10
        );
        let hold15 = strategy_set_specs(crate::alpha11::HOLD15_STRATEGY_NAME, &options).unwrap();
        assert_eq!(hold15.len(), 1);
        assert_eq!(
            hold15[0].strategy_name,
            crate::alpha11::HOLD15_STRATEGY_NAME
        );
        assert_eq!(
            hold15[0].entry_init_policy.max_age_blocks,
            Some(crate::alpha11::ENTRY_INIT_MAX_AGE_BLOCKS)
        );
        assert_eq!(
            hold15[0]
                .entry_init_policy
                .max_price_ratio_to_initial
                .as_deref(),
            Some(crate::alpha11::ENTRY_INIT_MAX_PRICE_RATIO_TO_INITIAL)
        );

        let hold16 = strategy_set_specs(crate::alpha11::HOLD16_STRATEGY_NAME, &options).unwrap();
        assert_eq!(hold16.len(), 1);
        assert_eq!(
            hold16[0].strategy_name,
            crate::alpha11::HOLD16_STRATEGY_NAME
        );
        assert_eq!(hold16[0].max_hold_blocks, Some(16));
        assert_eq!(
            hold16[0].entry_bankroll_eth.as_deref(),
            Some(crate::alpha11::INITIAL_ENTRY_BANKROLL_ETH)
        );

        let hold16_all_pools =
            strategy_set_specs(crate::alpha11::HOLD16_ALL_POOLS_STRATEGY_NAME, &options).unwrap();
        assert_eq!(hold16_all_pools.len(), 1);
        assert_eq!(
            hold16_all_pools[0].strategy_name,
            crate::alpha11::HOLD16_ALL_POOLS_STRATEGY_NAME
        );
        assert_eq!(hold16_all_pools[0].max_hold_blocks, Some(16));
        assert!(hold16_all_pools[0].allowed_protocols.is_empty());
        assert_eq!(
            hold16_all_pools[0].entry_bankroll_eth.as_deref(),
            Some(crate::alpha11::INITIAL_ENTRY_BANKROLL_ETH)
        );

        let hold3_validation =
            strategy_set_specs(crate::alpha11::HOLD3_VALIDATION_STRATEGY_NAME, &options).unwrap();
        assert_eq!(hold3_validation.len(), 1);
        assert_eq!(
            hold3_validation[0].strategy_name,
            crate::alpha11::HOLD3_VALIDATION_STRATEGY_NAME
        );
        assert_eq!(hold3_validation[0].max_hold_blocks, Some(3));
        assert_eq!(
            hold3_validation[0].entry_bankroll_eth.as_deref(),
            Some("0.01")
        );
    }
}
