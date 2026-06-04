// Address PnL SQL builds a reusable run-scoped aggregate over persisted
// token_pnl rows, then layers list/profile/trade-specific projections on top.
pub const ADDRESS_PNL_AGG_CTE: &str = r#"
WITH movement_counts AS (
    SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
    FROM token_pnl.pool_pnl_movements
    WHERE run_id = $1
    GROUP BY run_id, pool_id, address
),
address_rows AS (
    SELECT
        a.address,
        a.pool_id,
        s.token_address,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        s.lifecycle,
        ARRAY(
            SELECT CASE label
                WHEN 'valuation:terminal_zero' THEN 'valuation:closed_zero_valuation'
                ELSE label
            END
            FROM unnest(s.pool_labels) AS labels(label)
        ) AS pool_labels,
        s.token_creator_address,
        s.pool_creator_address,
        CASE a.position_status
            WHEN 'terminal_zero' THEN 'closed_zero_valuation'
            ELSE a.position_status
        END AS position_status,
        CASE a.valuation_status
            WHEN 'terminal_zero' THEN 'closed_zero_valuation'
            ELSE a.valuation_status
        END AS valuation_status,
        a.reconciliation_status,
        a.movement_rows_retained,
        a.movement_rows_backed,
        a.actor_roles,
        a.is_user_candidate,
        a.realized_pnl_denom,
        a.unrealized_value_denom,
        a.total_pnl_denom,
        a.first_block,
        a.latest_block,
        COALESCE(a.movement_count, 0)::bigint AS movement_count,
        COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
        COALESCE(a.denom_cashflow::double precision, 0.0) AS denom_cashflow,
        ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow,
        (a.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (a.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        COALESCE(a.native_fee::double precision, 0.0) AS native_fee,
        COALESCE(a.native_priority_fee::double precision, 0.0) AS native_priority_fee,
        (lower(s.denom_address) = '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2') AS denom_is_eth
    FROM token_pnl.pool_address_pnl a
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = a.run_id AND s.pool_id = a.pool_id
    LEFT JOIN movement_counts m
      ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
    WHERE a.run_id = $1
),
address_agg AS (
    SELECT
        address,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END)::bigint AS trade_count,
        SUM(exact_trade_count)::bigint AS exact_trade_count,
        SUM(movement_count)::bigint AS movement_count,
        MIN(first_block) AS first_block,
        MAX(latest_block) AS latest_block,
        SUM(denom_in) AS denom_in,
        SUM(denom_out) AS denom_out,
        SUM(denom_cashflow) AS net_denom_cashflow,
        SUM(abs_denom_cashflow) AS total_abs_denom_flow,
        SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END) AS scam_abs_denom_flow,
        (COUNT(*) FILTER (WHERE is_scam))::double precision / NULLIF(COUNT(*)::double precision, 0.0) AS scam_ratio,
        (COUNT(DISTINCT token_address) FILTER (WHERE is_scam))::double precision / NULLIF(COUNT(DISTINCT token_address)::double precision, 0.0) AS scam_token_ratio,
        ARRAY_AGG(DISTINCT scam_mechanism) FILTER (WHERE scam_mechanism IS NOT NULL) AS scam_mechanisms,
        ARRAY_AGG(DISTINCT scam_label) FILTER (WHERE scam_label IS NOT NULL) AS scam_labels,
        ARRAY_AGG(DISTINCT lifecycle) FILTER (WHERE lifecycle IS NOT NULL) AS lifecycles,
        SUM(CASE WHEN lower(address) = lower(COALESCE(token_creator_address, '')) THEN 1 ELSE 0 END)::bigint AS token_creator_position_count,
        SUM(CASE WHEN lower(address) = lower(COALESCE(pool_creator_address, '')) THEN 1 ELSE 0 END)::bigint AS pool_creator_position_count,
        SUM(realized_pnl_denom)::double precision AS realized_pnl_denom_sum,
        SUM(unrealized_value_denom)::double precision AS unrealized_pnl_denom_sum,
        SUM(total_pnl_denom)::double precision AS total_pnl_denom_sum,
        SUM(native_fee + native_priority_fee)::double precision AS gas_paid,
        COUNT(*) FILTER (WHERE total_pnl_denom > 0)::bigint AS win_count,
        COUNT(*) FILTER (WHERE total_pnl_denom < 0)::bigint AS loss_count,
        COUNT(*) FILTER (WHERE total_pnl_denom = 0 OR total_pnl_denom IS NULL)::bigint AS breakeven_count,
        SUM(movement_rows_retained)::bigint AS movement_rows_retained_sum,
        COUNT(*) FILTER (WHERE movement_rows_backed)::bigint AS movement_backed_position_count,
        BOOL_AND(denom_is_eth) AS all_denom_eth,
        BOOL_OR(is_user_candidate) AS any_user_candidate
    FROM address_rows
    GROUP BY address
),
label_agg AS (
    SELECT
        ar.address,
        ARRAY_AGG(DISTINCT label.value ORDER BY label.value) FILTER (WHERE label.value IS NOT NULL) AS pool_labels
    FROM address_rows ar
    LEFT JOIN LATERAL jsonb_array_elements_text(COALESCE(ar.pool_labels, '[]'::jsonb)) AS label(value) ON true
    GROUP BY ar.address
),
role_agg AS (
    SELECT
        ar.address,
        ARRAY_AGG(DISTINCT role.value ORDER BY role.value) FILTER (WHERE role.value IS NOT NULL) AS actor_roles
    FROM address_rows ar
    LEFT JOIN LATERAL jsonb_array_elements_text(COALESCE(ar.actor_roles, '[]'::jsonb)) AS role(value) ON true
    GROUP BY ar.address
),
movement_total AS (
    SELECT COUNT(*)::bigint AS movement_rows
    FROM token_pnl.pool_pnl_movements
    WHERE run_id = $1
)
"#;

pub(super) fn eth_trader_ranked_cte() -> String {
    format!(
        "{ADDRESS_PNL_AGG_CTE}
        , ranked AS (
            SELECT
                aa.*,
                COALESCE(la.pool_labels, ARRAY[]::text[]) AS pool_labels,
                ARRAY(
                    SELECT DISTINCT role
                    FROM unnest(
                        COALESCE(ra.actor_roles, ARRAY[]::text[])
                        || ARRAY_REMOVE(ARRAY[
                            CASE WHEN aa.token_creator_position_count > 0 THEN 'token_creator'::text END,
                            CASE WHEN aa.pool_creator_position_count > 0 THEN 'pool_creator'::text END
                        ], NULL)
                    ) AS role(role)
                    ORDER BY role
                ) AS role_flags,
                mt.movement_rows,
                (
                    COALESCE(aa.scam_ratio, 0.0)
                    * LN(1.0 + GREATEST(aa.trade_count::double precision, 0.0))
                    * LN(1.0 + GREATEST(aa.scam_token_count::double precision, 0.0))
                    * LN(1.0 + GREATEST(COALESCE(aa.scam_abs_denom_flow, 0.0), 0.000001))
                ) AS inflation_score
            FROM address_agg aa
            LEFT JOIN label_agg la USING (address)
            LEFT JOIN role_agg ra USING (address)
            CROSS JOIN movement_total mt
        )"
    )
}

pub(super) fn eth_trader_filtered_cte() -> String {
    format!(
        "{}
        ,
        filtered AS (
            SELECT *
            FROM ranked
            WHERE (
                $2::text = 'all'
                OR ($2::text = 'leaderboard' AND scam_pool_position_count > 0)
                OR ($2::text = 'inflation' AND scam_pool_position_count > 0)
                OR ($2::text = 'custody' AND (
                    EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                        WHERE lower(label.value) LIKE 'custody:%'
                           OR lower(label.value) IN (
                               'risk:custody_buyer_token_confiscation',
                               'risk:holder_balance_backdoor_drain',
                               'risk:pair_balance_backdoor_drain'
                           )
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                        WHERE lower(mechanism.value) IN (
                            'custody_buyer_token_confiscation',
                            'holder_balance_backdoor_drain',
                            'pair_balance_backdoor_drain'
                        )
                    )
                ))
                OR ($2::text = 'creators' AND (token_creator_position_count > 0 OR pool_creator_position_count > 0))
            )
            AND ($3::double precision IS NULL OR COALESCE(scam_ratio, 0.0) >= $3)
            AND ($4::bigint IS NULL OR trade_count >= $4)
            AND (
                $5::text IS NULL
                OR EXISTS (
                    SELECT 1
                    FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                    WHERE lower(mechanism.value) = lower($5)
                )
            )
            AND (
                $6::text IS NULL
                OR EXISTS (
                    SELECT 1
                    FROM unnest(COALESCE(scam_labels, ARRAY[]::text[]) || COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                    WHERE lower(label.value) = lower($6)
                )
            )
            AND (
                $7::text IS NULL
                OR ($7::text = 'token_creator' AND token_creator_position_count > 0)
                OR ($7::text = 'pool_creator' AND pool_creator_position_count > 0)
                OR ($7::text = 'creator' AND (token_creator_position_count > 0 OR pool_creator_position_count > 0))
                OR ($7::text = 'custody' AND (
                    EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                        WHERE lower(label.value) LIKE 'custody:%'
                           OR lower(label.value) IN (
                               'risk:custody_buyer_token_confiscation',
                               'risk:holder_balance_backdoor_drain',
                               'risk:pair_balance_backdoor_drain'
                           )
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                        WHERE lower(mechanism.value) IN (
                            'custody_buyer_token_confiscation',
                            'holder_balance_backdoor_drain',
                            'pair_balance_backdoor_drain'
                        )
                    )
                ))
            )
        )",
        eth_trader_ranked_cte()
    )
}

pub(super) fn eth_trader_filtered_summary_sql() -> String {
    format!(
        "{}
        SELECT
            COUNT(*)::bigint AS address_count,
            COALESCE(SUM(pool_position_count), 0)::bigint AS pool_position_count,
            COALESCE(SUM(token_count), 0)::bigint AS token_count,
            COALESCE(SUM(scam_pool_position_count), 0)::bigint AS scam_pool_position_count,
            COALESCE(SUM(scam_token_count), 0)::bigint AS scam_token_count,
            COUNT(*) FILTER (WHERE scam_pool_position_count > 0)::bigint AS scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio = 1.0)::bigint AS all_scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio >= 0.75 AND scam_pool_position_count > 0)::bigint AS high_scam_ratio_address_count,
            COALESCE(SUM(trade_count), 0)::bigint AS trade_count,
            COALESCE(SUM(exact_trade_count), 0)::bigint AS exact_trade_count,
            COALESCE(SUM(movement_count), 0)::bigint AS movement_count,
            COALESCE(SUM(total_abs_denom_flow), 0.0)::double precision AS total_abs_denom_flow,
            COALESCE(SUM(scam_abs_denom_flow), 0.0)::double precision AS scam_abs_denom_flow,
            AVG(scam_ratio) AS avg_scam_ratio,
            AVG(scam_token_ratio) AS avg_scam_token_ratio,
            AVG(inflation_score) AS avg_inflation_score,
            MAX(latest_block) AS latest_block,
            COALESCE(BOOL_OR(movement_rows > 0), false) AS movement_rows_available
        FROM filtered",
        eth_trader_filtered_cte()
    )
}

pub(super) fn eth_trader_rows_sql(sort_expression: &str) -> String {
    format!(
        "{}
        SELECT
            (ROW_NUMBER() OVER (ORDER BY {sort_expression}))::bigint AS rank,
            *
        FROM filtered
        ORDER BY {sort_expression}
        LIMIT $8 OFFSET $9",
        eth_trader_filtered_cte()
    )
}

pub(super) fn eth_trader_profile_summary_sql() -> String {
    format!(
        "{}
        SELECT *
        FROM ranked
        WHERE lower(address) = lower($2)
        LIMIT 1",
        eth_trader_ranked_cte()
    )
}

pub(super) fn eth_trader_positions_sql(order_by: &str) -> String {
    format!(
        r#"
        WITH movement_counts AS (
            SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
            FROM token_pnl.pool_pnl_movements
            WHERE run_id = $1 AND lower(address) = lower($2)
            GROUP BY run_id, pool_id, address
        )
        SELECT
            s.pool_id,
            s.token_address,
            s.denom_address,
            s.protocol,
            s.is_scam,
            s.scam_label,
            s.scam_mechanism,
            s.lifecycle,
            s.token_creator_address,
            s.pool_creator_address,
            ARRAY(
                SELECT CASE label
                    WHEN 'valuation:terminal_zero' THEN 'valuation:closed_zero_valuation'
                    ELSE label
                END
                FROM unnest(COALESCE(labels.pool_labels, ARRAY[]::text[])) AS label_rows(label)
            ) AS pool_labels,
            COALESCE(roles.actor_roles, ARRAY[]::text[]) AS actor_role_flags,
            ARRAY(
                SELECT DISTINCT role
                FROM unnest(
                    COALESCE(roles.actor_roles, ARRAY[]::text[])
                    || ARRAY_REMOVE(ARRAY[
                        CASE WHEN lower(a.address) = lower(COALESCE(s.token_creator_address, '')) THEN 'token_creator'::text END,
                        CASE WHEN lower(a.address) = lower(COALESCE(s.pool_creator_address, '')) THEN 'pool_creator'::text END
                    ], NULL)
                ) AS role(role)
                ORDER BY role
            ) AS role_flags,
            CASE a.position_status
                WHEN 'terminal_zero' THEN 'closed_zero_valuation'
                ELSE a.position_status
            END AS position_status,
            CASE a.valuation_status
                WHEN 'terminal_zero' THEN 'closed_zero_valuation'
                ELSE a.valuation_status
            END AS valuation_status,
            a.reconciliation_status,
            a.realized_pnl_denom::double precision AS realized_pnl_denom,
            a.unrealized_value_denom::double precision AS unrealized_value_denom,
            a.unrealized_value_denom::double precision AS unrealized_pnl_denom,
            a.total_pnl_denom::double precision AS total_pnl_denom,
            a.movement_rows_retained,
            a.movement_rows_backed,
            a.is_user_candidate,
            a.accounting_context,
            lower(s.denom_address) = '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2' AS denom_is_eth,
            a.first_block,
            a.latest_block,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            COALESCE(a.denom_cashflow::double precision, 0.0) AS denom_cashflow,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow,
            (a.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
            (a.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
            a.token_balance::double precision AS token_balance,
            a.marked_token_value_denom::double precision AS marked_token_value_denom,
            a.pnl_proxy_denom::double precision AS pnl_proxy_denom
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        LEFT JOIN LATERAL (
            SELECT ARRAY_AGG(label.value ORDER BY label.value) AS pool_labels
            FROM jsonb_array_elements_text(COALESCE(s.pool_labels, '[]'::jsonb)) AS label(value)
        ) labels ON true
        LEFT JOIN LATERAL (
            SELECT ARRAY_AGG(DISTINCT role.value ORDER BY role.value) AS actor_roles
            FROM jsonb_array_elements_text(COALESCE(a.actor_roles, '[]'::jsonb)) AS role(value)
        ) roles ON true
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
        ORDER BY {order_by}
        LIMIT $3
        "#
    )
}

pub(super) fn eth_trader_position_detail_sql() -> String {
    eth_trader_positions_sql("s.pool_id")
        .replace(
            "WHERE a.run_id = $1 AND lower(a.address) = lower($2)",
            "WHERE a.run_id = $1 AND lower(a.address) = lower($2) AND lower(a.pool_id) = lower($3)",
        )
        .replace("LIMIT $3", "LIMIT 1")
}

pub(super) fn eth_trader_mechanism_breakdown_sql() -> String {
    r#"
    WITH movement_counts AS (
        SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
        FROM token_pnl.pool_pnl_movements
        WHERE run_id = $1 AND lower(address) = lower($2)
        GROUP BY run_id, pool_id, address
    ),
    rows AS (
        SELECT
            COALESCE(NULLIF(s.scam_mechanism, ''), 'unclassified') AS mechanism,
            s.token_address,
            s.is_scam,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
    )
    SELECT
        mechanism,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        COALESCE(SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END), 0)::bigint AS trade_count,
        COALESCE(SUM(abs_denom_cashflow), 0.0)::double precision AS total_abs_denom_flow,
        COALESCE(SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END), 0.0)::double precision AS scam_abs_denom_flow
    FROM rows
    GROUP BY mechanism
    ORDER BY scam_pool_position_count DESC, scam_abs_denom_flow DESC, trade_count DESC, mechanism
    LIMIT $3
    "#
    .to_string()
}

pub(super) fn eth_trader_label_breakdown_sql() -> String {
    r#"
    WITH movement_counts AS (
        SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
        FROM token_pnl.pool_pnl_movements
        WHERE run_id = $1 AND lower(address) = lower($2)
        GROUP BY run_id, pool_id, address
    ),
    base AS (
        SELECT
            s.token_address,
            s.is_scam,
            s.scam_label,
            s.pool_labels,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
    ),
    label_rows AS (
        SELECT
            'scam_label'::text AS kind,
            scam_label AS label,
            token_address,
            is_scam,
            movement_count,
            exact_trade_count,
            abs_denom_cashflow
        FROM base
        WHERE scam_label IS NOT NULL AND scam_label <> ''
        UNION ALL
        SELECT
            'pool_label'::text AS kind,
            label.value AS label,
            base.token_address,
            base.is_scam,
            base.movement_count,
            base.exact_trade_count,
            base.abs_denom_cashflow
        FROM base
        JOIN LATERAL jsonb_array_elements_text(COALESCE(base.pool_labels, '[]'::jsonb)) AS label(value) ON true
    )
    SELECT
        kind,
        label,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        COALESCE(SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END), 0)::bigint AS trade_count,
        COALESCE(SUM(abs_denom_cashflow), 0.0)::double precision AS total_abs_denom_flow,
        COALESCE(SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END), 0.0)::double precision AS scam_abs_denom_flow
    FROM label_rows
    GROUP BY kind, label
    ORDER BY scam_pool_position_count DESC, scam_abs_denom_flow DESC, trade_count DESC, kind, label
    LIMIT $3
    "#
    .to_string()
}

pub(super) fn eth_trader_recent_movements_sql() -> String {
    r#"
    SELECT
        m.pool_id,
        s.token_address,
        s.denom_address,
        s.protocol,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        m.entry_index,
        m.tx_hash,
        m.block_number,
        m.block_timestamp,
        m.tx_index,
        m.log_index,
        m.kind,
        m.pool_direct,
        (m.token_in_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_in,
        (m.token_out_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_out,
        (m.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (m.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        ((m.denom_out_raw - m.denom_in_raw) / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_delta,
        ((m.token_out_raw - m.token_in_raw) / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_delta,
        (m.native_fee_raw / POWER(10::numeric, 18))::double precision AS native_fee_eth,
        (m.native_priority_fee_raw / POWER(10::numeric, 18))::double precision AS native_priority_fee_eth
    FROM token_pnl.pool_pnl_movements m
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = m.run_id AND s.pool_id = m.pool_id
    WHERE m.run_id = $1 AND lower(m.address) = lower($2)
    ORDER BY m.block_number DESC, m.tx_index DESC, m.entry_index DESC
    LIMIT $3
    "#
    .to_string()
}

pub(super) fn eth_trader_trade_movements_sql() -> String {
    r#"
    SELECT
        m.pool_id,
        s.token_address,
        s.denom_address,
        s.protocol,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        m.entry_index,
        m.tx_hash,
        m.block_number,
        m.block_timestamp,
        m.tx_index,
        m.log_index,
        m.kind,
        m.pool_direct,
        (m.token_in_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_in,
        (m.token_out_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_out,
        (m.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (m.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        ((m.denom_out_raw - m.denom_in_raw) / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_delta,
        ((m.token_out_raw - m.token_in_raw) / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_delta,
        (m.native_fee_raw / POWER(10::numeric, 18))::double precision AS native_fee_eth,
        (m.native_priority_fee_raw / POWER(10::numeric, 18))::double precision AS native_priority_fee_eth
    FROM token_pnl.pool_pnl_movements m
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = m.run_id AND s.pool_id = m.pool_id
    WHERE m.run_id = $1 AND lower(m.address) = lower($2) AND lower(m.pool_id) = lower($3)
    ORDER BY m.block_number DESC, m.tx_index DESC, m.entry_index DESC
    LIMIT $4
    "#
    .to_string()
}
