pub(crate) const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

pub(crate) fn valid_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

pub(crate) fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

pub(crate) fn ratio_to_initial(current: f64, initial: Option<f64>) -> Option<f64> {
    let initial = initial?;
    if current.is_finite() && current >= 0.0 && initial > 0.0 {
        Some(current / initial)
    } else {
        None
    }
}

pub(crate) fn ratio_if_positive(numerator: f64, denominator: Option<f64>) -> Option<f64> {
    let denominator = denominator?;
    if numerator.is_finite() && numerator >= 0.0 && denominator > 0.0 && denominator.is_finite() {
        Some(numerator / denominator)
    } else {
        None
    }
}

pub(crate) fn signed_block_delta(start: Option<u64>, end: Option<u64>) -> Option<i64> {
    Some(end? as i64 - start? as i64)
}

pub(crate) fn signed_volume_imbalance(buy: f64, sell: f64) -> Option<f64> {
    let total = buy + sell;
    if total > 0.0 && total.is_finite() {
        Some((buy - sell) / total)
    } else {
        None
    }
}

pub(crate) fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
