use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::observations::transfer_policy::TransferPolicyObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferPolicySignal {
    Blacklist,
    Whitelist,
    Pause,
    TradingPause,
    SniperBlacklist,
}

impl TransferPolicySignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blacklist => "blacklist",
            Self::Whitelist => "whitelist",
            Self::Pause => "pause",
            Self::TradingPause => "trading_pause",
            Self::SniperBlacklist => "sniper_blacklist",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Blacklist => "policy:blacklist_present",
            Self::Whitelist => "policy:whitelist_present",
            Self::Pause => "policy:pause_present",
            Self::TradingPause => "policy:trading_pause_present",
            Self::SniperBlacklist => "policy:sniper_blacklist_present",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferPolicySignalState {
    pub signal: TransferPolicySignal,
    pub present: bool,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub function: Option<String>,
    pub selector: Option<String>,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TransferPolicyTrack {
    pub blacklist_present: bool,
    pub whitelist_present: bool,
    pub pause_present: bool,
    pub trading_pause_present: bool,
    pub sniper_blacklist_present: bool,
    pub signals: Vec<TransferPolicySignalState>,
    pub evidence: Vec<EvidenceRef>,
}

impl TransferPolicyTrack {
    pub fn from_observations(observations: &[TransferPolicyObservation]) -> Self {
        let mut track = Self::default();
        for observation in observations {
            track.add_observation(observation);
        }
        track
    }

    pub fn add_observation(&mut self, observation: &TransferPolicyObservation) {
        if observation.present {
            match observation.signal {
                TransferPolicySignal::Blacklist => self.blacklist_present = true,
                TransferPolicySignal::Whitelist => self.whitelist_present = true,
                TransferPolicySignal::Pause => self.pause_present = true,
                TransferPolicySignal::TradingPause => self.trading_pause_present = true,
                TransferPolicySignal::SniperBlacklist => self.sniper_blacklist_present = true,
            }
        }

        self.evidence.push(observation.evidence.clone());
        self.signals.push(TransferPolicySignalState {
            signal: observation.signal,
            present: observation.present,
            actor: observation.actor.clone(),
            subject: observation.subject.clone(),
            function: observation.function.clone(),
            selector: observation.selector.clone(),
            block_number: observation.block_number,
            tx_hash: observation.tx_hash.clone(),
            evidence: vec![observation.evidence.clone()],
        });
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut labels = Vec::new();
        if self.blacklist_present {
            labels.push(TransferPolicySignal::Blacklist.label());
        }
        if self.whitelist_present {
            labels.push(TransferPolicySignal::Whitelist.label());
        }
        if self.pause_present {
            labels.push(TransferPolicySignal::Pause.label());
        }
        if self.trading_pause_present {
            labels.push(TransferPolicySignal::TradingPause.label());
        }
        if self.sniper_blacklist_present {
            labels.push(TransferPolicySignal::SniperBlacklist.label());
        }
        labels
    }

    pub fn live_blocker_labels(&self) -> Vec<&'static str> {
        self.active_labels()
    }
}
