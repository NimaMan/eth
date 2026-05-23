-- Event evidence for audit/debug joins. Do not use future event fields as model inputs.
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
