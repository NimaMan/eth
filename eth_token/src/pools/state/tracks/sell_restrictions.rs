use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::sell_restrictions::SellRestrictionObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SellRestrictionSignal {
    MaxTx,
    MaxWallet,
    Cooldown,
    OneSellPerBlock,
    LowSellLimit,
}

impl SellRestrictionSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaxTx => "max_tx",
            Self::MaxWallet => "max_wallet",
            Self::Cooldown => "cooldown",
            Self::OneSellPerBlock => "one_sell_per_block",
            Self::LowSellLimit => "low_sell_limit",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MaxTx => "restriction:max_tx",
            Self::MaxWallet => "restriction:max_wallet",
            Self::Cooldown => "restriction:cooldown",
            Self::OneSellPerBlock => "restriction:one_sell_per_block",
            Self::LowSellLimit => "restriction:low_sell_limit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SellRestrictionState {
    pub signal: SellRestrictionSignal,
    pub present: bool,
    pub limit_tokens: Option<f64>,
    pub limit_denom: Option<f64>,
    pub limit_bps_of_supply: Option<f64>,
    pub cooldown_seconds: Option<u64>,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SellRestrictionsTrack {
    pub max_tx: bool,
    pub max_wallet: bool,
    pub cooldown: bool,
    pub one_sell_per_block: bool,
    pub low_sell_limit: bool,
    pub restrictions: Vec<SellRestrictionState>,
    pub evidence: Vec<EvidenceRef>,
}

impl SellRestrictionsTrack {
    pub fn from_observations(observations: &[SellRestrictionObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &SellRestrictionObservation) {
        if observation.present {
            match observation.signal {
                SellRestrictionSignal::MaxTx => self.max_tx = true,
                SellRestrictionSignal::MaxWallet => self.max_wallet = true,
                SellRestrictionSignal::Cooldown => self.cooldown = true,
                SellRestrictionSignal::OneSellPerBlock => self.one_sell_per_block = true,
                SellRestrictionSignal::LowSellLimit => self.low_sell_limit = true,
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.restrictions.push(SellRestrictionState {
            signal: observation.signal,
            present: observation.present,
            limit_tokens: observation.limit_tokens,
            limit_denom: observation.limit_denom,
            limit_bps_of_supply: observation.limit_bps_of_supply,
            cooldown_seconds: observation.cooldown_seconds,
            actor: observation.actor.clone(),
            subject: observation.subject.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.max_tx {
            labels.push(SellRestrictionSignal::MaxTx.label());
        }
        if self.max_wallet {
            labels.push(SellRestrictionSignal::MaxWallet.label());
        }
        if self.cooldown {
            labels.push(SellRestrictionSignal::Cooldown.label());
        }
        if self.one_sell_per_block {
            labels.push(SellRestrictionSignal::OneSellPerBlock.label());
        }
        if self.low_sell_limit {
            labels.push(SellRestrictionSignal::LowSellLimit.label());
        }
        labels
    }

    pub fn live_blocker_labels(&self) -> Vec<&'static str> {
        self.active_labels()
    }
}
