/// Transaction Router Module
///
/// Routes incoming mempool transactions into lanes that describe how they can
/// affect currently tracked tokens and pools before those transactions mine.
mod classify;
mod lanes;
mod metrics;
mod protocol;
mod router;
pub mod types;

pub use metrics::{LpApprovalRouterStats, RouteOrigin};
pub use router::TransactionRouter;
pub use types::{ClassificationResult, RouteLane, SimulationPriority, TransactionCategory};
// CreatorFunctionType now exported from function_detector module
pub use crate::function_detector::CreatorFunctionType;
pub use classify::{ContractCreationRouter, CreatorTransactionRouter};
