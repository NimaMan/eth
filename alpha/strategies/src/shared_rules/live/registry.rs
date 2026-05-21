use super::spec::{LiveStrategySpec, LiveStrategySpecOptions};
use super::strategy_sets;

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
        strategy_sets::alpha11_hold_sweep::SET_NAME => {
            Ok(strategy_sets::alpha11_hold_sweep::specs(options))
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
            strategy_set_specs(strategy_sets::alpha11_hold_sweep::SET_NAME, &options)
                .unwrap()
                .len(),
            3
        );
    }
}
