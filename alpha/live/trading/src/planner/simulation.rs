use async_trait::async_trait;

use crate::{PreSubmitSimulation, PreparedSellRoute};

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

#[async_trait]
pub trait PreSubmitSimulator: Send + Sync {
    async fn simulate(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug)]
pub struct FixedPreSubmitSimulator {
    simulation: PreSubmitSimulation,
}

impl FixedPreSubmitSimulator {
    pub fn new(simulation: PreSubmitSimulation) -> Self {
        Self { simulation }
    }
}

#[async_trait]
impl PreSubmitSimulator for FixedPreSubmitSimulator {
    async fn simulate(
        &self,
        _input: &LivePrioritySellPlannerInput,
        _route: &PreparedSellRoute,
    ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError> {
        Ok(self.simulation.clone())
    }
}
