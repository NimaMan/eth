use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::tax_policy::TaxPolicyObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaxPolicySignal {
    ExtremeSellTax,
    AsymmetricBuySellTax,
    ModifiableTax,
    PersonalizedTax,
    HighSellGas,
}

impl TaxPolicySignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExtremeSellTax => "extreme_sell_tax",
            Self::AsymmetricBuySellTax => "asymmetric_buy_sell_tax",
            Self::ModifiableTax => "modifiable_tax",
            Self::PersonalizedTax => "personalized_tax",
            Self::HighSellGas => "high_sell_gas",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ExtremeSellTax => "tax:extreme_sell_tax",
            Self::AsymmetricBuySellTax => "tax:asymmetric_buy_sell_tax",
            Self::ModifiableTax => "tax:modifiable",
            Self::PersonalizedTax => "tax:personalized",
            Self::HighSellGas => "tax:high_sell_gas",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaxPolicyState {
    pub signal: TaxPolicySignal,
    pub present: bool,
    pub buy_tax_percent: Option<f64>,
    pub sell_tax_percent: Option<f64>,
    pub gas_used: Option<u64>,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TaxPolicyTrack {
    pub extreme_sell_tax: bool,
    pub asymmetric_buy_sell_tax: bool,
    pub modifiable_tax: bool,
    pub personalized_tax: bool,
    pub high_sell_gas: bool,
    pub policies: Vec<TaxPolicyState>,
    pub evidence: Vec<EvidenceRef>,
}

impl TaxPolicyTrack {
    pub fn from_observations(observations: &[TaxPolicyObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &TaxPolicyObservation) {
        if observation.present {
            match observation.signal {
                TaxPolicySignal::ExtremeSellTax => self.extreme_sell_tax = true,
                TaxPolicySignal::AsymmetricBuySellTax => self.asymmetric_buy_sell_tax = true,
                TaxPolicySignal::ModifiableTax => self.modifiable_tax = true,
                TaxPolicySignal::PersonalizedTax => self.personalized_tax = true,
                TaxPolicySignal::HighSellGas => self.high_sell_gas = true,
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.policies.push(TaxPolicyState {
            signal: observation.signal,
            present: observation.present,
            buy_tax_percent: observation.buy_tax_percent,
            sell_tax_percent: observation.sell_tax_percent,
            gas_used: observation.gas_used,
            actor: observation.actor.clone(),
            subject: observation.subject.clone(),
            selector: observation.selector.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.extreme_sell_tax {
            labels.push(TaxPolicySignal::ExtremeSellTax.label());
        }
        if self.asymmetric_buy_sell_tax {
            labels.push(TaxPolicySignal::AsymmetricBuySellTax.label());
        }
        if self.modifiable_tax {
            labels.push(TaxPolicySignal::ModifiableTax.label());
        }
        if self.personalized_tax {
            labels.push(TaxPolicySignal::PersonalizedTax.label());
        }
        if self.high_sell_gas {
            labels.push(TaxPolicySignal::HighSellGas.label());
        }
        labels
    }

    pub fn live_blocker_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.extreme_sell_tax {
            labels.push(TaxPolicySignal::ExtremeSellTax.label());
        }
        if self.high_sell_gas {
            labels.push(TaxPolicySignal::HighSellGas.label());
        }
        labels
    }
}
