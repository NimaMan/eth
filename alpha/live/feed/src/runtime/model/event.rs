use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTokenEvent {
    WarmupStarted {
        id: String,
        start_block: u64,
        end_block: u64,
    },
    BlockApplied {
        block_number: u64,
        block_hash: String,
        updated_tokens: Vec<String>,
        updated_v2_pools: Vec<String>,
        updated_v3_pools: Vec<String>,
        updated_v4_pools: Vec<String>,
    },
    RuntimeLive {
        id: String,
        current_block: Option<u64>,
    },
    RuntimeStopped {
        id: Option<String>,
        current_block: Option<u64>,
    },
    RuntimeFailed {
        id: Option<String>,
        block_number: Option<u64>,
        message: String,
    },
}
