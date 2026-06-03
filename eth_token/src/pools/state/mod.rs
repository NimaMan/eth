//! Orthogonal pool-state tracking tracks and consumer projections.

pub mod evidence;
pub mod fixtures;
pub mod labels;
pub mod model;
pub mod observations;
pub mod tracks;
pub mod update;
pub mod views;

#[cfg(test)]
mod tests;

pub use evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};
pub use labels::labels_for_state;
pub use model::PoolTrackedState;
pub use views::{LiveTradingPoolView, PoolPnlStateView, RiskAtlasPoolView};
