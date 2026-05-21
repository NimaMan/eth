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
        crate::shared_rules::lp_approval_warning_exit::SUITE_NAME
        | crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME => {
            Ok(vec![strategy_sets::lp_approval_warning::spec(options)])
        }
        strategy_sets::risk_atlas::LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME => Ok(vec![
            strategy_sets::risk_atlas::lp_gate_hold15_buy_confirm_lp_maxhold_spec(options),
        ]),
        alpha11_specs::SET_NAME => Ok(alpha11_specs::specs(options)),
        alpha11_specs::DEPLOY_SET_NAME => Ok(alpha11_specs::deploy_specs(options)),
        crate::alpha11::HOLD15_STRATEGY_NAME => Ok(vec![alpha11_specs::hold15_spec(options)]),
        crate::alpha11::DEPLOY_HOLD15_STRATEGY_NAME => {
            Ok(vec![alpha11_specs::deploy_hold15_spec(options)])
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
                crate::shared_rules::lp_approval_warning_exit::SUITE_NAME,
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
            3
        );
        let hold15 = strategy_set_specs(crate::alpha11::HOLD15_STRATEGY_NAME, &options).unwrap();
        assert_eq!(hold15.len(), 1);
        assert_eq!(
            hold15[0].strategy_name,
            crate::alpha11::HOLD15_STRATEGY_NAME
        );
        assert!(hold15[0].max_entry_price_ratio_to_initial.is_none());
        let deploy_hold15 =
            strategy_set_specs(crate::alpha11::DEPLOY_HOLD15_STRATEGY_NAME, &options).unwrap();
        assert_eq!(deploy_hold15.len(), 1);
        assert_eq!(
            deploy_hold15[0].strategy_name,
            crate::alpha11::DEPLOY_HOLD15_STRATEGY_NAME
        );
        assert_eq!(
            deploy_hold15[0].max_entry_price_ratio_to_initial.as_deref(),
            Some(crate::alpha11::MAX_ENTRY_PRICE_RATIO_TO_INITIAL)
        );
    }
}
