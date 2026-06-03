use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::behavioral_outcomes::BehavioralOutcomeObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralOutcomeSignal {
    HighSellFailRate,
    SiphonRateHigh,
    SellsAbsentAfterBuys,
    SniperBlacklistCluster,
    ReusedConfiscationPattern,
}

impl BehavioralOutcomeSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HighSellFailRate => "high_sell_fail_rate",
            Self::SiphonRateHigh => "siphon_rate_high",
            Self::SellsAbsentAfterBuys => "sells_absent_after_buys",
            Self::SniperBlacklistCluster => "sniper_blacklist_cluster",
            Self::ReusedConfiscationPattern => "reused_confiscation_pattern",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::HighSellFailRate => "behavior:high_sell_fail_rate",
            Self::SiphonRateHigh => "behavior:siphon_rate_high",
            Self::SellsAbsentAfterBuys => "behavior:sells_absent_after_buys",
            Self::SniperBlacklistCluster => "behavior:sniper_blacklist_cluster",
            Self::ReusedConfiscationPattern => "behavior:reused_confiscation_pattern",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehavioralOutcomeState {
    pub signal: BehavioralOutcomeSignal,
    pub present: bool,
    pub rate: Option<f64>,
    pub sample_size: Option<u64>,
    pub buy_count: Option<u64>,
    pub sell_count: Option<u64>,
    pub pattern_id: Option<String>,
    pub signature: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BehavioralOutcomesTrack {
    pub high_sell_fail_rate: bool,
    pub siphon_rate_high: bool,
    pub sells_absent_after_buys: bool,
    pub sniper_blacklist_cluster: bool,
    pub reused_confiscation_pattern: bool,
    pub outcomes: Vec<BehavioralOutcomeState>,
    pub evidence: Vec<EvidenceRef>,
}

impl BehavioralOutcomesTrack {
    pub fn from_observations(observations: &[BehavioralOutcomeObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &BehavioralOutcomeObservation) {
        if observation.present {
            match observation.signal {
                BehavioralOutcomeSignal::HighSellFailRate => self.high_sell_fail_rate = true,
                BehavioralOutcomeSignal::SiphonRateHigh => self.siphon_rate_high = true,
                BehavioralOutcomeSignal::SellsAbsentAfterBuys => {
                    self.sells_absent_after_buys = true
                }
                BehavioralOutcomeSignal::SniperBlacklistCluster => {
                    self.sniper_blacklist_cluster = true
                }
                BehavioralOutcomeSignal::ReusedConfiscationPattern => {
                    self.reused_confiscation_pattern = true
                }
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.outcomes.push(BehavioralOutcomeState {
            signal: observation.signal,
            present: observation.present,
            rate: observation.rate,
            sample_size: observation.sample_size,
            buy_count: observation.buy_count,
            sell_count: observation.sell_count,
            pattern_id: observation.pattern_id.clone(),
            signature: observation.signature.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.high_sell_fail_rate {
            labels.push(BehavioralOutcomeSignal::HighSellFailRate.label());
        }
        if self.siphon_rate_high {
            labels.push(BehavioralOutcomeSignal::SiphonRateHigh.label());
        }
        if self.sells_absent_after_buys {
            labels.push(BehavioralOutcomeSignal::SellsAbsentAfterBuys.label());
        }
        if self.sniper_blacklist_cluster {
            labels.push(BehavioralOutcomeSignal::SniperBlacklistCluster.label());
        }
        if self.reused_confiscation_pattern {
            labels.push(BehavioralOutcomeSignal::ReusedConfiscationPattern.label());
        }
        labels
    }
}
