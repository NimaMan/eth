use std::collections::HashMap;

use crate::{
    ids::{PortfolioId, PositionId},
    position::Position,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PortfolioState {
    pub id: Option<PortfolioId>,
    pub positions: HashMap<PositionId, Position>,
}

impl PortfolioState {
    pub fn active_position_count(&self) -> usize {
        self.positions
            .values()
            .filter(|position| !position.state.is_terminal())
            .count()
    }
}
