pub mod action;
pub mod dataset;
pub mod error;
pub mod observation;
pub mod reward;
pub mod step;

/// Basic trait implemented by every environment scenario.
#[async_trait::async_trait]
pub trait Environment {
    type Action;
    type Observation;

    async fn reset(&mut self) -> Result<Self::Observation, error::EnvError>;
    async fn step(
        &mut self,
        action: Self::Action,
    ) -> Result<step::StepOutput<Self::Observation>, error::EnvError>;
}

pub use action::PredictionAction;
pub use dataset::{LabeledObservation, ObservationDataset};
pub use error::EnvError;
pub use observation::Observation;
pub use reward::{binary_payoff, negative_absolute_error};
pub use step::StepOutput;
