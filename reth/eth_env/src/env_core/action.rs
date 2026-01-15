use serde::{Deserialize, Serialize};

/// Minimal action space for prediction-style environments.
///
/// The default scenario accepts either a concrete price prediction or a hold
/// instruction. Additional variants can be layered on by scenario crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionAction {
    /// Do nothing for this step (useful for burn-in/observation windows).
    Hold,
    /// Emit a raw ETH/USD prediction that will be scored when the next
    /// observation arrives.
    Predict { price: f64 },
}

impl PredictionAction {
    pub fn predicted_price(&self) -> Option<f64> {
        match self {
            PredictionAction::Hold => None,
            PredictionAction::Predict { price } => Some(*price),
        }
    }
}
