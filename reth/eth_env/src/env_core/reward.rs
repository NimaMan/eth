/// Reward helpers shared by scenarios.
///
/// Negative absolute error is the default for supervised forecasting because it
/// is scale aware yet easy to optimise. Additional payoff models (e.g.,
/// Polymarket style binary predictions) can be layered on top of the same
/// observation stream.
pub fn negative_absolute_error(predicted: f64, realized: f64) -> f64 {
    -(predicted - realized).abs()
}

/// Binary payoff for prediction-market style contracts (e.g., Polymarket).
/// Returns 1.0 when the realised price exceeds the strike and the contract
/// settles "yes", otherwise 0.0. The caller can convert this into a reward by
/// subtracting the entry price paid for the contract.
pub fn binary_payoff(realized: f64, strike: f64) -> f64 {
    if realized >= strike {
        1.0
    } else {
        0.0
    }
}
