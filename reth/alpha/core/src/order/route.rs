use crate::ids::PoolAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteHint {
    pub pools: Vec<PoolAddress>,
}
