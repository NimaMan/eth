WITH cohort AS (
    SELECT *
    FROM (
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
                actor_address
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
        )
        SELECT
            lr.pool_key,
            min(e.detection_timestamp) FILTER (
                WHERE e.event_kind = 'trading_enabled'
            ) AS first_trading_enabled_at,
            min(e.detection_timestamp) FILTER (
                WHERE e.event_kind = 'lp_position_approval'
            ) AS first_lp_approval_at,
            lr.first_liquidity_removal_at,
            count(*) FILTER (
                WHERE e.event_kind = 'liquidity_removal'
            ) AS liquidity_removal_signal_count
        FROM liquidity_removal_pools lr
        JOIN signal_lifecycle_events e
          ON e.pool_key = lr.pool_key
        GROUP BY lr.pool_key, lr.first_liquidity_removal_at
    ) rows
)
SELECT
    count(*) AS liquidity_removal_pools,
    count(*) FILTER (
        WHERE first_trading_enabled_at < first_liquidity_removal_at
    ) AS creator_main_tx_public_mempool_pools,
    count(*) FILTER (
        WHERE first_lp_approval_at < first_liquidity_removal_at
    ) AS lp_control_tx_public_mempool_pools,
    count(*) FILTER (
        WHERE first_trading_enabled_at < first_liquidity_removal_at
           OR first_lp_approval_at < first_liquidity_removal_at
    ) AS pool_mempool_managed_pre_removal_pools,
    count(*) FILTER (
        WHERE liquidity_removal_signal_count > 0
    ) AS pool_mempool_exit_signal_available_pools,
    percentile_cont(0.5) WITHIN GROUP (
        ORDER BY EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_lp_approval_at
        ))
    ) FILTER (
        WHERE first_lp_approval_at < first_liquidity_removal_at
    ) AS median_seconds_lp_approval_to_liquidity_removal,
    percentile_cont(0.9) WITHIN GROUP (
        ORDER BY EXTRACT(EPOCH FROM (
            first_liquidity_removal_at - first_lp_approval_at
        ))
    ) FILTER (
        WHERE first_lp_approval_at < first_liquidity_removal_at
    ) AS p90_seconds_lp_approval_to_liquidity_removal
FROM cohort;
