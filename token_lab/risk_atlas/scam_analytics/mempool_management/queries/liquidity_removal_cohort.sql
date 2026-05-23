WITH signal_lifecycle_events AS (
    SELECT
        signal_id,
        lower(pool_identifier) AS pool_key,
        pool_identifier,
        pool_protocol,
        lower(token_address) AS token_key,
        token_address,
        event_kind,
        detection_timestamp,
        pending_tx_hash,
        lower(actor_address) AS actor_key,
        actor_address,
        lower(subject_address) AS subject_key,
        subject_address,
        headline,
        payload
    FROM live_trading.signal_events
    WHERE pool_identifier IS NOT NULL
      AND event_kind IN (
        'trading_enabled',
        'lp_position_approval',
        'liquidity_removal'
      )
),
liquidity_removal_pools AS (
    SELECT
        pool_key,
        min(detection_timestamp) AS first_liquidity_removal_at
    FROM signal_lifecycle_events
    WHERE event_kind = 'liquidity_removal'
    GROUP BY pool_key
),
pool_rollup AS (
    SELECT
        lr.pool_key,
        max(e.pool_identifier) AS pool_identifier,
        max(e.pool_protocol) AS pool_protocol,
        max(e.token_address) AS token_address,
        min(e.detection_timestamp) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ) AS first_trading_enabled_at,
        min(e.detection_timestamp) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ) AS first_lp_approval_at,
        lr.first_liquidity_removal_at,
        count(*) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ) AS trading_enabled_signal_count,
        count(*) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ) AS lp_approval_signal_count,
        count(*) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ) AS liquidity_removal_signal_count,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ), NULL) AS trading_enabled_actor_keys,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ), NULL) AS lp_approval_actor_keys,
        array_remove(array_agg(DISTINCT e.actor_key) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ), NULL) AS liquidity_removal_actor_keys,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'trading_enabled'
        ), NULL) AS trading_enabled_tx_hashes,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'lp_position_approval'
        ), NULL) AS lp_approval_tx_hashes,
        array_remove(array_agg(DISTINCT e.pending_tx_hash) FILTER (
            WHERE e.event_kind = 'liquidity_removal'
        ), NULL) AS liquidity_removal_tx_hashes,
        jsonb_agg(
            jsonb_build_object(
                'signal_id', e.signal_id,
                'event_kind', e.event_kind,
                'detection_timestamp', e.detection_timestamp,
                'pending_tx_hash', e.pending_tx_hash,
                'actor_address', e.actor_address,
                'subject_address', e.subject_address,
                'headline', e.headline,
                'payload', e.payload
            )
            ORDER BY e.detection_timestamp, e.signal_id
        ) AS evidence_events
    FROM liquidity_removal_pools lr
    JOIN signal_lifecycle_events e
      ON e.pool_key = lr.pool_key
    GROUP BY lr.pool_key, lr.first_liquidity_removal_at
)
SELECT
    pool_key,
    pool_identifier,
    pool_protocol,
    token_address,
    first_trading_enabled_at,
    first_lp_approval_at,
    first_liquidity_removal_at,
    CASE
        WHEN first_trading_enabled_at IS NULL THEN NULL
        ELSE EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_trading_enabled_at
        ))::bigint
    END AS seconds_trading_enabled_to_liquidity_removal,
    CASE
        WHEN first_lp_approval_at IS NULL THEN NULL
        ELSE EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_lp_approval_at
        ))::bigint
    END AS seconds_lp_approval_to_liquidity_removal,
    trading_enabled_signal_count,
    lp_approval_signal_count,
    liquidity_removal_signal_count,
    trading_enabled_actor_keys,
    lp_approval_actor_keys,
    liquidity_removal_actor_keys,
    trading_enabled_tx_hashes,
    lp_approval_tx_hashes,
    liquidity_removal_tx_hashes,
    first_trading_enabled_at < first_liquidity_removal_at
        AS creator_main_tx_public_mempool,
    first_lp_approval_at < first_liquidity_removal_at
        AS lp_control_tx_public_mempool,
    liquidity_removal_signal_count > 0
        AS removal_tx_public_mempool,
    (
        first_trading_enabled_at < first_liquidity_removal_at
        OR first_lp_approval_at < first_liquidity_removal_at
    ) AS pool_mempool_managed_pre_removal,
    liquidity_removal_signal_count > 0
        AS pool_mempool_exit_signal_available,
    evidence_events
FROM pool_rollup
ORDER BY first_liquidity_removal_at DESC, pool_key;
