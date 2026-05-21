use super::spec::{LiveStrategySpec, LiveStrategySpecOptions};

pub mod alpha11;
mod lp_approval_warning;
pub mod mempool_exit_diagnostics;
pub mod risk_atlas;

pub fn suite_specs(
    suite_name: &str,
    options: &LiveStrategySpecOptions,
) -> Result<Vec<LiveStrategySpec>, String> {
    match suite_name {
        mempool_exit_diagnostics::SUITE_NAME => Ok(mempool_exit_diagnostics::specs(options)),
        crate::shared_rules::lp_approval_warning_exit::SUITE_NAME
        | crate::shared_rules::lp_approval_warning_exit::STRATEGY_NAME => {
            Ok(vec![lp_approval_warning::spec(options)])
        }
        risk_atlas::LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME => Ok(vec![
            risk_atlas::lp_gate_hold15_buy_confirm_lp_maxhold_spec(options),
        ]),
        alpha11::SUITE_NAME => Ok(alpha11::specs(options)),
        other => Err(format!("unsupported strategy suite: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_known_suites() {
        let options = LiveStrategySpecOptions::default();

        assert_eq!(
            suite_specs(mempool_exit_diagnostics::SUITE_NAME, &options)
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            suite_specs(
                crate::shared_rules::lp_approval_warning_exit::SUITE_NAME,
                &options
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            suite_specs(
                risk_atlas::LP_GATE_HOLD15_BUY_CONFIRM_LP_MAXHOLD_STRATEGY_NAME,
                &options
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(suite_specs(alpha11::SUITE_NAME, &options).unwrap().len(), 3);
    }
}
