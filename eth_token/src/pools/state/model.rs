use serde::{Deserialize, Serialize};

use crate::custody::CustodyFinding;
use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::labels::labels_for_state;
use crate::pools::state::tracks::{
    ActivityTrack, BehavioralOutcomesTrack, ContractPostureTrack, CounterpartyRolesTrack,
    CustodyTrack, EligibilityTrack, EvidenceQualityTrack, IdentityTrack, LifecycleTrack,
    LiquidityTrack, LpControlTrack, MarketStructureTrack, RiskTrack, RouteabilityTrack,
    SellRestrictionsTrack, SupplyControlTrack, TaxPolicyTrack, TransferPolicyTrack, ValuationTrack,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolTrackedState {
    pub identity: IdentityTrack,
    pub market_structure: MarketStructureTrack,
    pub lifecycle: LifecycleTrack,
    pub liquidity: LiquidityTrack,
    pub routeability: RouteabilityTrack,
    pub valuation: ValuationTrack,
    pub activity: ActivityTrack,
    pub lp_control: LpControlTrack,
    pub custody: CustodyTrack,
    #[serde(default)]
    pub transfer_policy: TransferPolicyTrack,
    #[serde(default)]
    pub sell_restrictions: SellRestrictionsTrack,
    #[serde(default)]
    pub tax_policy: TaxPolicyTrack,
    #[serde(default)]
    pub contract_posture: ContractPostureTrack,
    #[serde(default)]
    pub supply_control: SupplyControlTrack,
    #[serde(default)]
    pub behavioral_outcomes: BehavioralOutcomesTrack,
    pub risk: RiskTrack,
    pub roles: CounterpartyRolesTrack,
    pub eligibility: EligibilityTrack,
    pub quality: EvidenceQualityTrack,
    pub labels: Vec<String>,
}

impl PoolTrackedState {
    pub fn from_base_pool(base: &BasePool, custody_findings: &[CustodyFinding]) -> Self {
        let flags = PoolStateFlags::from_base_and_custody_findings(base, custody_findings);
        Self::from_base_pool_and_flags(base, custody_findings, &flags)
    }

    pub fn from_base_pool_and_flags(
        base: &BasePool,
        custody_findings: &[CustodyFinding],
        flags: &PoolStateFlags,
    ) -> Self {
        let mut state = Self {
            identity: IdentityTrack::from_base_pool(base),
            market_structure: MarketStructureTrack::from_base_pool(base),
            lifecycle: LifecycleTrack::from_lifecycle(base.state.lifecycle, flags),
            liquidity: LiquidityTrack::from_flags(flags),
            routeability: RouteabilityTrack::from_base_and_flags(base, flags),
            valuation: ValuationTrack::from_base_and_flags(base, flags),
            activity: ActivityTrack::from_base_pool(base),
            lp_control: LpControlTrack::from_base_and_flags(base, flags),
            custody: CustodyTrack::from_base_flags_and_findings(base, flags, custody_findings),
            transfer_policy: TransferPolicyTrack::default(),
            sell_restrictions: SellRestrictionsTrack::default(),
            tax_policy: TaxPolicyTrack::default(),
            contract_posture: ContractPostureTrack::default(),
            supply_control: SupplyControlTrack::default(),
            behavioral_outcomes: BehavioralOutcomesTrack::default(),
            risk: RiskTrack::from_base_and_flags(base, flags),
            roles: CounterpartyRolesTrack::from_base_pool(base),
            eligibility: EligibilityTrack::from_base_pool(base),
            quality: EvidenceQualityTrack::default(),
            labels: Vec::new(),
        };
        state.refresh_derived();
        state
    }

    pub fn refresh_derived(&mut self) {
        self.quality = EvidenceQualityTrack::from_evidence_refs(self.track_evidence());
        self.labels = labels_for_state(self);
    }

    pub fn track_evidence(&self) -> Vec<EvidenceRef> {
        let mut evidence = Vec::new();
        evidence.extend(self.identity.evidence.clone());
        evidence.extend(self.market_structure.evidence.clone());
        evidence.extend(self.lifecycle.evidence.clone());
        evidence.extend(self.liquidity.evidence.clone());
        evidence.extend(self.routeability.evidence.clone());
        evidence.extend(self.valuation.evidence.clone());
        evidence.extend(self.activity.evidence.clone());
        evidence.extend(self.lp_control.evidence.clone());
        evidence.extend(self.custody.evidence.clone());
        for capability in &self.custody.capabilities {
            evidence.extend(capability.evidence.clone());
        }
        evidence.extend(self.transfer_policy.evidence.clone());
        evidence.extend(self.sell_restrictions.evidence.clone());
        evidence.extend(self.tax_policy.evidence.clone());
        evidence.extend(self.contract_posture.evidence.clone());
        evidence.extend(self.supply_control.evidence.clone());
        evidence.extend(self.behavioral_outcomes.evidence.clone());
        evidence.extend(self.risk.evidence.clone());
        for role in &self.roles.roles {
            evidence.extend(role.evidence.clone());
        }
        evidence.extend(self.eligibility.evidence.clone());
        evidence
    }
}
