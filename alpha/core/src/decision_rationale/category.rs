use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCategory {
    Entry,
    Exit,
    Hold,
    RiskPolicy,
    Execution,
    Unknown,
}

impl ReasonCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Exit => "exit",
            Self::Hold => "hold",
            Self::RiskPolicy => "risk_policy",
            Self::Execution => "execution",
            Self::Unknown => "unknown",
        }
    }
}
