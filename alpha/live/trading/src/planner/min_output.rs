use alloy_primitives::U256;

use super::LivePrioritySellPlannerError;

pub fn derive_min_output_from_expected_output(
    amount: U256,
    max_slippage_bps: u32,
) -> Result<U256, LivePrioritySellPlannerError> {
    if max_slippage_bps >= 10_000 {
        return Err(LivePrioritySellPlannerError::InvalidInput(format!(
            "max_slippage_bps must be below 10000 for production min-output; got {max_slippage_bps}"
        )));
    }

    let keep_bps = 10_000u32 - max_slippage_bps;
    amount
        .checked_mul(U256::from(keep_bps))
        .map(|value| value / U256::from(10_000u64))
        .ok_or_else(|| {
            LivePrioritySellPlannerError::InvalidInput(
                "min-output overflow while applying slippage".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_min_output_from_exact_output_and_slippage() {
        let min_output =
            derive_min_output_from_expected_output(U256::from(10_000_000_000_000_000u128), 500)
                .unwrap();

        assert_eq!(min_output, U256::from(9_500_000_000_000_000u128));
    }

    #[test]
    fn rejects_full_slippage() {
        let error = derive_min_output_from_expected_output(U256::from(100u64), 10_000)
            .expect_err("full slippage should reject");

        assert!(error.to_string().contains("max_slippage_bps"));
    }
}
