-- Token-control signal rows for reviewing pair-balance backdoor candidates.
SELECT
    o.run_id,
    lower(o.token_address) AS token_address,
    lower(o.pool_address) AS pool_address,
    o.protocol,
    o.block_number,
    o.active_observation_index,
    o.control_transfer_from_after_renounce_seen_as_of,
    o.control_transfer_from_after_renounce_in_block,
    o.control_transfer_from_holder_to_burn_seen_as_of,
    o.control_transfer_from_holder_to_burn_in_block,
    o.control_transfer_from_pair_seen_as_of,
    o.control_transfer_from_pair_in_block,
    o.control_transfer_from_without_transfer_log_seen_as_of,
    o.control_transfer_from_without_transfer_log_in_block,
    o.pair_token_to_control_seen_as_of,
    o.pair_token_to_control_in_block,
    o.pair_token_to_control_to_pool_reserve_ratio,
    o.pair_balance_backdoor_signal_seen_as_of,
    o.pair_balance_backdoor_signal_in_block,
    o.last_pair_balance_backdoor_signal_to_as_of_chain_block_delta
FROM public.risk_atlas_observations o
WHERE o.run_id = %(run_id)s
  AND UPPER(o.protocol) = %(protocol)s
  AND (
      o.control_transfer_from_after_renounce_seen_as_of
      OR o.control_transfer_from_holder_to_burn_seen_as_of
      OR o.control_transfer_from_pair_seen_as_of
      OR o.control_transfer_from_without_transfer_log_seen_as_of
      OR o.pair_token_to_control_seen_as_of
      OR o.pair_balance_backdoor_signal_seen_as_of
  )
ORDER BY o.block_number, o.active_observation_index;
