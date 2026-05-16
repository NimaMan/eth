pub mod report;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskAtlasSourceKind {
    CurrentArtifacts,
    TokenAnalyticsRows,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasSourceLayout {
    pub kind: RiskAtlasSourceKind,
    pub root: PathBuf,
    pub distribution_report: Option<PathBuf>,
}

impl RiskAtlasSourceLayout {
    pub fn current_100k(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            kind: RiskAtlasSourceKind::CurrentArtifacts,
            distribution_report: Some(
                root.join("artifacts/reports/scam_100k_25007276_25107275_distribution.md"),
            ),
            root,
        }
    }
}
