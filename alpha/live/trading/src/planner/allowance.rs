use async_trait::async_trait;
use eth_alpha_core::amount::Amount;
use serde::{Deserialize, Serialize};

use crate::PreparedSellRoute;

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowanceMode {
    PreApproved,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AllowanceDecision {
    pub mode: AllowanceMode,
    pub spender: String,
    pub token_amount: Amount,
    #[serde(default)]
    pub detail: Option<String>,
}

impl AllowanceDecision {
    pub fn pre_approved(spender: impl Into<String>, token_amount: Amount) -> Self {
        Self {
            mode: AllowanceMode::PreApproved,
            spender: spender.into(),
            token_amount,
            detail: None,
        }
    }

    pub fn missing(
        spender: impl Into<String>,
        token_amount: Amount,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            mode: AllowanceMode::Missing,
            spender: spender.into(),
            token_amount,
            detail: Some(detail.into()),
        }
    }

    pub fn ensure_preapproved(&self) -> Result<(), LivePrioritySellPlannerError> {
        match self.mode {
            AllowanceMode::PreApproved => Ok(()),
            AllowanceMode::Missing => Err(LivePrioritySellPlannerError::Allowance(
                self.detail
                    .clone()
                    .unwrap_or_else(|| "sell token allowance is missing".to_string()),
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllowanceCheck {
    pub route: PreparedSellRoute,
    pub decision: AllowanceDecision,
}

#[async_trait]
pub trait AllowanceChecker: Send + Sync {
    async fn check_allowance(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: PreparedSellRoute,
    ) -> Result<AllowanceCheck, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug)]
pub struct StaticAllowanceChecker {
    decision: AllowanceMode,
}

impl StaticAllowanceChecker {
    pub fn pre_approved() -> Self {
        Self {
            decision: AllowanceMode::PreApproved,
        }
    }

    pub fn missing() -> Self {
        Self {
            decision: AllowanceMode::Missing,
        }
    }
}

#[async_trait]
impl AllowanceChecker for StaticAllowanceChecker {
    async fn check_allowance(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: PreparedSellRoute,
    ) -> Result<AllowanceCheck, LivePrioritySellPlannerError> {
        let token_amount = input.intent.amount.clone();
        let spender = route.router_address.clone();
        let decision = match self.decision {
            AllowanceMode::PreApproved => AllowanceDecision::pre_approved(spender, token_amount),
            AllowanceMode::Missing => AllowanceDecision::missing(
                spender,
                token_amount,
                "pre-existing token allowance is required for v1 priority sells",
            ),
        };
        Ok(AllowanceCheck { route, decision })
    }
}

#[derive(Clone, Debug, Default)]
pub struct VaultInternalAllowanceChecker;

#[async_trait]
impl AllowanceChecker for VaultInternalAllowanceChecker {
    async fn check_allowance(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: PreparedSellRoute,
    ) -> Result<AllowanceCheck, LivePrioritySellPlannerError> {
        let mut decision = AllowanceDecision::pre_approved(
            route.router_address.clone(),
            input.intent.amount.clone(),
        );
        decision.detail =
            Some("Baygus vault emergency sell approves the router internally".to_string());
        Ok(AllowanceCheck { route, decision })
    }
}
