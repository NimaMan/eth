-- Creator/control network seed rows around token launches.
-- This is a review extract, not an as-of model feature yet.
SELECT
    o.run_id,
    lower(o.token_address) AS token_address,
    lower(o.pool_address) AS pool_address,
    o.protocol,
    min(o.block_number) AS first_observation_block,
    max(o.block_number) AS last_observation_block,
    bool_or(o.pair_balance_backdoor_signal_seen_as_of) AS pair_balance_backdoor_signal_seen,
    bool_or(o.control_transfer_from_holder_to_burn_seen_as_of) AS holder_to_burn_signal_seen,
    bool_or(o.control_transfer_from_pair_seen_as_of) AS pair_transfer_from_signal_seen,
    max(o.pair_token_to_control_to_pool_reserve_ratio) AS max_pair_token_to_control_to_pool_reserve_ratio
FROM public.risk_atlas_observations o
WHERE o.run_id = %(run_id)s
  AND UPPER(o.protocol) = %(protocol)s
GROUP BY o.run_id, o.token_address, o.pool_address, o.protocol
ORDER BY pair_balance_backdoor_signal_seen DESC, first_observation_block;
