-- First scam label per token/pool for a single Risk Atlas run.
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
