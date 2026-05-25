"""SQL text used by Risk Atlas modeling extracts."""

OBSERVATIONS_SQL = """
SELECT
    o.run_id,
    lower(o.token_address) AS token_address,
    lower(o.pool_address) AS pool_address,
    o.protocol,
    o.block_number,
    o.timestamp,
    o.active_observation_index,
    o.active_reasons,
    o.tx_count,
    o.token_transfer_count,
    o.denom_transfer_count,
    o.buy_volume_denom,
    o.sell_volume_denom,
    o.total_bribe_eth,
    o.total_liquidity_denom,
    o.denom_reserve,
    o.token_reserve,
    o.pooled_token_supply_ratio,
    o.reserve_quality_status,
    o.price_to_initial_ratio_trustworthy,
    o.price_to_initial_ratio,
    o.price_denom_per_token,
    o.initial_price_denom_per_token,
    o.buy_tax,
    o.sell_tax,
    o.can_buy,
    o.can_sell,
    o.effective_can_buy,
    o.effective_can_sell,
    o.liquidity_removed_as_of,
    o.liquidity_removal_in_block,
    o.direct_lp_removal_as_of,
    o.direct_lp_removal_in_block,
    o.lp_approved_pct_as_of,
    o.lp_approval_count_in_block,
    o.lp_approval_seen_as_of,
    o.lp_total_supply,
    o.lp_max_approval_amount_as_of,
    o.lp_max_approval_pct_as_of,
    o.lp_removable_pct_as_of,
    o.lp_router_removable_pct_as_of,
    o.lp_approval_owner_is_creator,
    o.creator_lp_balance_pct_as_of,
    o.creator_lp_approved_pct_as_of,
    o.creator_lp_removable_pct_as_of,
    o.creator_lp_router_removable_pct_as_of,
    o.creator_lp_approved_gt_90_pct_as_of,
    o.creator_lp_router_removable_gt_90_pct_as_of,
    o.last_lp_approval_amount_pct_of_total_supply,
    o.first_lp_approval_to_as_of_chain_block_delta,
    o.last_lp_approval_to_as_of_chain_block_delta,
    o.pool_creation_to_first_lp_approval_chain_block_delta,
    o.pool_creation_to_last_lp_approval_chain_block_delta,
    o.trading_enabled_to_first_lp_approval_chain_block_delta,
    o.trading_enabled_to_last_lp_approval_chain_block_delta,
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
    o.last_pair_balance_backdoor_signal_to_as_of_chain_block_delta,
    o.token_transfer_to_total_supply_ratio,
    o.token_transfer_to_pool_token_reserve_ratio
FROM public.risk_atlas_observations o
JOIN public.risk_atlas_pool_eligibility e
  ON e.run_id = o.run_id
 AND lower(e.token_address) = lower(o.token_address)
 AND lower(e.pool_address) = lower(o.pool_address)
WHERE o.run_id = %(run_id)s
  AND e.eligible
  AND UPPER(COALESCE(e.protocol, o.protocol, '')) = %(protocol)s;
"""

SCAM_LABELS_SQL = """
SELECT
    ev.run_id,
    lower(ev.token_address) AS token_address,
    lower(ev.pool_address) AS pool_address,
    MIN(ev.block_number) AS scam_block,
    MIN(ev.block_timestamp) AS scam_block_timestamp
FROM public.risk_atlas_event_evidence ev
JOIN public.risk_atlas_pool_eligibility e
  ON e.run_id = ev.run_id
 AND lower(e.token_address) = lower(ev.token_address)
 AND lower(e.pool_address) = lower(ev.pool_address)
WHERE ev.run_id = %(run_id)s
  AND ev.event_kind = 'scam'
  AND e.eligible
  AND UPPER(COALESCE(e.protocol, ev.protocol, '')) = %(protocol)s
GROUP BY 1, 2, 3;
"""

EVENT_EVIDENCE_SQL = """
SELECT
    ev.run_id,
    lower(ev.token_address) AS token_address,
    lower(ev.pool_address) AS pool_address,
    ev.event_kind,
    ev.mechanism,
    ev.block_number,
    ev.block_timestamp,
    ev.tx_hash,
    ev.mempool_first_seen_ms
FROM public.risk_atlas_event_evidence ev
JOIN public.risk_atlas_pool_eligibility e
  ON e.run_id = ev.run_id
 AND lower(e.token_address) = lower(ev.token_address)
 AND lower(e.pool_address) = lower(ev.pool_address)
WHERE ev.run_id = %(run_id)s
  AND e.eligible
  AND UPPER(COALESCE(e.protocol, ev.protocol, '')) = %(protocol)s;
"""

PRELAUNCH_CREATOR_NETWORK_SQL = """
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
"""

TOKEN_CONTROL_SIGNALS_SQL = """
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
"""
