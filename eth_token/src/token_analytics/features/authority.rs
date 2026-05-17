use serde::{Deserialize, Serialize};

use super::utils::{normalize_address, ZERO_ADDRESS};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenAuthorityFeatures {
    pub creator_address: Option<String>,
    pub current_owner: Option<String>,
    pub current_owner_is_zero_address: Option<bool>,
    pub current_owner_is_creator: Option<bool>,
    pub ownership_renounced: bool,
    pub renouncement_block: Option<u64>,
    pub control_address_count: Option<u32>,
    pub control_address_tx_count_in_block: Option<u32>,
}

impl TokenAuthorityFeatures {
    pub fn with_owner_context(
        creator_address: Option<String>,
        current_owner: Option<String>,
        ownership_renounced: bool,
    ) -> Self {
        let current_owner_is_creator = creator_address
            .as_deref()
            .zip(current_owner.as_deref())
            .map(|(creator, owner)| creator.eq_ignore_ascii_case(owner));
        let current_owner_is_zero_address = current_owner
            .as_deref()
            .map(|owner| normalize_address(owner) == ZERO_ADDRESS);

        Self {
            creator_address,
            current_owner,
            current_owner_is_zero_address,
            current_owner_is_creator,
            ownership_renounced,
            ..Default::default()
        }
    }
}
