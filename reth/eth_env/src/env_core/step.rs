use serde::{Deserialize, Serialize};

/// Generic step output that wraps the scenario-specific observation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput<O> {
    pub observation: O,
    pub reward: f64,
    pub done: bool,
    pub info: serde_json::Value,
}

impl<O> StepOutput<O> {
    pub fn terminal(observation: O, reward: f64) -> Self {
        Self {
            observation,
            reward,
            done: true,
            info: serde_json::Value::Null,
        }
    }
}
