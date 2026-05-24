"""Token-control feature names used by backdoor-horizon experiments."""

TOKEN_CONTROL_FEATURES = [
    "control_transfer_from_after_renounce_seen_as_of",
    "control_transfer_from_after_renounce_in_block",
    "control_transfer_from_holder_to_burn_seen_as_of",
    "control_transfer_from_holder_to_burn_in_block",
    "control_transfer_from_pair_seen_as_of",
    "control_transfer_from_pair_in_block",
    "control_transfer_from_without_transfer_log_seen_as_of",
    "control_transfer_from_without_transfer_log_in_block",
    "pair_token_to_control_seen_as_of",
    "pair_token_to_control_in_block",
    "pair_token_to_control_to_pool_reserve_ratio",
    "pair_balance_backdoor_signal_seen_as_of",
    "pair_balance_backdoor_signal_in_block",
    "last_pair_balance_backdoor_signal_to_as_of_chain_block_delta",
]
