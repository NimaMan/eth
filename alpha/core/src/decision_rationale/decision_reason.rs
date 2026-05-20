use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{category::ReasonCategory, code, source};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionReason {
    pub code: String,
    pub category: ReasonCategory,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(default)]
    pub details: Value,
}

impl DecisionReason {
    pub fn from_parts(
        raw_reason: Option<&str>,
        event_source: Option<&str>,
        action: Option<&str>,
    ) -> Option<Self> {
        let raw = raw_reason?.trim();
        if raw.is_empty() {
            return None;
        }

        let normalized = code::normalize_reason(raw, action);
        Some(Self {
            code: normalized.code,
            category: normalized.category,
            label: normalized.label,
            source: source::normalize_source(event_source),
            raw: Some(raw.to_string()),
            details: normalized.details,
        })
    }

    pub fn category_key(&self) -> &'static str {
        self.category.as_str()
    }
}
