use super::spec::{LiveStrategySpec, LiveStrategySpecOptions, StrategyId};
use crate::strategies::alpha11::factory as alpha11_specs;
use crate::strategies::diagnostics as strategy_sets;

/// Resolve a [`StrategyId`] into its complete resolved [`super::spec::StrategySpec`]s.
///
/// This is the single dispatch entry point all runtimes (live and backtest)
/// share, so an id produces an identical spec set in every mode.
pub fn resolve(
    id: &StrategyId,
    options: &LiveStrategySpecOptions,
) -> Result<Vec<LiveStrategySpec>, String> {
    strategy_set_specs(id.as_str(), options)
}

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

    /// Live and backtest resolve a strategy id through this same registry/factory,
    /// so a given id must resolve to an identical StrategySpec in every mode.
    /// This asserts the single-id registry resolutions match the alpha11 factory
    /// set entries that the backtest suite consumes — i.e. one source of truth.
    #[test]
    fn strategy_ids_resolve_identically_across_modes() {
        let options = LiveStrategySpecOptions::default();

        // Backtest source: the full alpha11 factory set.
        let factory_set = alpha11_specs::specs(&options);
        // Live source: resolving the sweep set id through the registry.
        let registry_set = resolve(&StrategyId::new(alpha11_specs::SET_NAME), &options).unwrap();
        assert_eq!(registry_set, factory_set, "sweep set must be identical");

        // Each single-strategy id resolves to the same spec the set contains.
        for id in [
            crate::strategies::alpha11::HOLD15_STRATEGY_NAME,
            crate::strategies::alpha11::HOLD16_STRATEGY_NAME,
            crate::strategies::alpha11::HOLD16_ALL_POOLS_STRATEGY_NAME,
            crate::strategies::alpha11::HOLD3_VALIDATION_STRATEGY_NAME,
        ] {
            let resolved = resolve(&StrategyId::new(id), &options).unwrap();
            assert_eq!(resolved.len(), 1, "{id} resolves to a single spec");
            // hold3-validation is not part of the sweep set, so only assert
            // set-membership for the ids that are in it.
            if let Some(in_set) = factory_set.iter().find(|s| s.strategy_name == id) {
                assert_eq!(&resolved[0], in_set, "{id} live spec == backtest spec");
            }
        }
    }

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
