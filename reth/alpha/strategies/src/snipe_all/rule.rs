#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleDecision {
    Enter { rule: &'static str },
    Exit { rule: &'static str },
    Hold { rule: &'static str, reason: String },
}

impl RuleDecision {
    pub fn hold(rule: &'static str, reason: impl Into<String>) -> Self {
        Self::Hold {
            rule,
            reason: reason.into(),
        }
    }
}
