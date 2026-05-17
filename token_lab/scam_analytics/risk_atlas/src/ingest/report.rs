use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use eyre::{eyre, Result};
use serde_json::{json, Value};

use crate::atlas;
use crate::db::schema::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, ModelReadinessItem, NumericStat,
    PoolEligibilityRow, ReviewExample, RiskAtlasRun,
};

pub const DEFAULT_100K_DISTRIBUTION_REPORT: &str =
    "token_lab/scam_analytics/artifacts/reports/scam_100k_25007276_25107275_distribution.md";

#[derive(Clone, Debug, PartialEq)]
pub struct RiskAtlasReportImport {
    pub run: RiskAtlasRun,
    pub distributions: Vec<DistributionBucket>,
    pub pool_eligibility: Vec<PoolEligibilityRow>,
    pub observations: Vec<crate::db::schema::ObservationRow>,
    pub numeric_stats: Vec<NumericStat>,
    pub active_targets: Vec<ActiveTargetSummary>,
    pub decision_questions: Vec<DecisionQuestion>,
    pub review_examples: Vec<ReviewExample>,
    pub model_readiness: Vec<ModelReadinessItem>,
}

#[derive(Clone, Debug)]
struct MarkdownTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

pub fn import_distribution_report(
    path: impl AsRef<Path>,
    run_id_override: Option<&str>,
) -> Result<RiskAtlasReportImport> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|error| {
        eyre!(
            "failed to read risk atlas report {}: {error}",
            path.display()
        )
    })?;
    let lines: Vec<&str> = contents.lines().collect();
    let tables = markdown_tables(&lines);

    let mut run = parse_run(&lines, path)?;
    if let Some(run_id) = run_id_override {
        run.run_id = run_id.to_string();
    }

    let mut distributions = Vec::new();
    for (heading, section) in distribution_sections() {
        if let Some(table) = tables.get(*heading) {
            append_distribution(section, table, &mut distributions)?;
        }
    }

    let mut numeric_stats = Vec::new();
    if let Some(table) = tables.get("Scam Age Distributions") {
        append_numeric_stats("scam_age", table, &mut numeric_stats)?;
    }
    if let Some(table) = tables.get("Direct LP Age And Approval Timing") {
        append_numeric_stats("direct_lp_age_approval", table, &mut numeric_stats)?;
    }

    let mut active_targets = Vec::new();
    if let Some(table) = tables.get("Active Observation Target Rows") {
        append_active_target_rows(table, &mut active_targets)?;
    }
    if let Some(table) = tables.get("Active Horizons") {
        append_active_horizon_rows(table, &mut active_targets)?;
    }
    let feature_tables = feature_report_tables(path)?;
    let decision_questions = build_decision_questions(
        &run,
        &distributions,
        &numeric_stats,
        &active_targets,
        &tables,
        feature_tables.as_ref(),
    )?;

    Ok(RiskAtlasReportImport {
        run,
        distributions,
        pool_eligibility: Vec::new(),
        observations: Vec::new(),
        numeric_stats,
        active_targets,
        decision_questions,
        review_examples: Vec::new(),
        model_readiness: default_model_readiness(),
    })
}

fn feature_report_tables(path: &Path) -> Result<Option<BTreeMap<String, MarkdownTable>>> {
    let Some(parent) = path.parent() else {
        return Ok(None);
    };
    let feature_path =
        parent.join("direct_lp_liquidity_removal_features_100k_25007276_25107275_summary.md");
    if !feature_path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&feature_path).map_err(|error| {
        eyre!(
            "failed to read risk atlas feature report {}: {error}",
            feature_path.display()
        )
    })?;
    let lines: Vec<&str> = contents.lines().collect();
    Ok(Some(markdown_tables(&lines)))
}

fn build_decision_questions(
    run: &RiskAtlasRun,
    distributions: &[DistributionBucket],
    numeric_stats: &[NumericStat],
    active_targets: &[ActiveTargetSummary],
    distribution_tables: &BTreeMap<String, MarkdownTable>,
    feature_tables: Option<&BTreeMap<String, MarkdownTable>>,
) -> Result<Vec<DecisionQuestion>> {
    let mut questions = Vec::new();
    let scam_labels = run.scam_label_count.unwrap_or_else(|| {
        distributions
            .iter()
            .filter(|row| row.section == "scam_mechanisms")
            .map(|row| row.count)
            .sum()
    });

    let mechanism_rows = distribution_payload(distributions, "scam_mechanisms");
    let top_mechanism = distributions
        .iter()
        .filter(|row| row.section == "scam_mechanisms")
        .max_by_key(|row| row.count);
    questions.push(decision_question(
        "scam_mechanism_mix",
        "Label mix",
        "Which scam mechanisms dominate eligible scam labels?",
        top_mechanism.map(|row| {
            format!(
                "{} / {}",
                labelize(&row.bucket),
                percent(row.count, scam_labels)
            )
        }),
        Some(format!(
            "{} scam labels are dominated by {}. This tells us which failure modes deserve the first feature work.",
            fmt_count(scam_labels),
            top_mechanism
                .map(|row| labelize(&row.bucket))
                .unwrap_or_else(|| "unknown mechanisms".to_string())
        )),
        "answered",
        Some("scam labels"),
        Some(scam_labels),
        json!({ "rows": mechanism_rows }),
        0,
    ));

    if let Some(stat) = numeric_stat(numeric_stats, "scam_age", "Trading Enabled To Label Blocks") {
        questions.push(decision_question(
            "scam_time_from_trading_enabled",
            "Timing",
            "How fast do eligible scam pools get scammed after trading becomes enabled?",
            stat.median.map(|value| format!("{} blocks median", fmt_number(value))),
            Some(format!(
                "Median scam label arrives after {} active-chain blocks; P90 is {} and P95 is {}. This is the base survival window for scam pools.",
                fmt_optional_number(stat.median),
                fmt_optional_number(stat.p90),
                fmt_optional_number(stat.p95)
            )),
            "answered",
            Some("scam labels"),
            Some(stat.count),
            json!({
                "stats": stat,
                "buckets": distribution_payload(distributions, "time_to_scam_buckets")
            }),
            1,
        ));
    }

    let pre_approval = distribution_count(distributions, "direct_lp_approval_pre_removal", "true");
    let no_pre_approval =
        distribution_count(distributions, "direct_lp_approval_pre_removal", "false");
    let approval_total = pre_approval + no_pre_approval;
    questions.push(decision_question(
        "lp_approval_visible_before_direct_removal",
        "Direct LP warning",
        "How often is LP approval visible before direct LP liquidity removal?",
        Some(format!("{}", percent(pre_approval, approval_total))),
        Some(format!(
            "{} of {} direct LP removals had a pre-removal LP approval; {} had no pre-removal approval visible in the current feature export.",
            fmt_count(pre_approval),
            fmt_count(approval_total),
            fmt_count(no_pre_approval)
        )),
        "answered",
        Some("direct LP removals"),
        Some(approval_total),
        json!({ "rows": distribution_payload(distributions, "direct_lp_approval_pre_removal") }),
        2,
    ));

    if let Some(stat) = numeric_stat(
        numeric_stats,
        "direct_lp_age_approval",
        "First Pre-Removal LP Approval To Removal Blocks",
    ) {
        questions.push(decision_question(
            "lp_approval_lead_time",
            "Direct LP warning",
            "How much warning does first relevant LP approval give before direct removal?",
            stat.median.map(|value| format!("{} blocks median", fmt_number(value))),
            Some(format!(
                "First pre-removal LP approval appears a median {} blocks before removal; P25 is {}, P75 is {}, P90 is {}. The current report measures chain-block lead time, not active-observation lead time yet.",
                fmt_optional_number(stat.median),
                fmt_optional_number(stat.p25),
                fmt_optional_number(stat.p75),
                fmt_optional_number(stat.p90)
            )),
            "answered",
            Some("direct LP removals with pre-approval"),
            Some(stat.count),
            json!({
                "stats": stat,
                "lead_buckets": feature_table_count_rows(feature_tables, "First Approval Lead To Removal")?
            }),
            3,
        ));
    }

    if let Some(stat) = numeric_stat(
        numeric_stats,
        "direct_lp_age_approval",
        "Last Pre-Removal LP Approval To Removal Blocks",
    ) {
        questions.push(decision_question(
            "last_lp_approval_lead_time",
            "Direct LP warning",
            "How close to removal is the latest pre-removal LP approval?",
            stat.median.map(|value| format!("{} blocks median", fmt_number(value))),
            Some(format!(
                "The latest pre-removal LP approval is still a median {} blocks before removal. P25 is {}, which helps separate same-block approvals from earlier warning.",
                fmt_optional_number(stat.median),
                fmt_optional_number(stat.p25)
            )),
            "answered",
            Some("direct LP removals with pre-approval"),
            Some(stat.count),
            json!({
                "stats": stat,
                "lead_buckets": feature_table_count_rows(feature_tables, "Last Pre-Removal Approval Lead")?
            }),
            4,
        ));
    }

    let can_sell_false = distribution_count(distributions, "scam_can_sell_at_label", "false");
    let can_sell_true = distribution_count(distributions, "scam_can_sell_at_label", "true");
    let can_sell_total = can_sell_false + can_sell_true;
    questions.push(decision_question(
        "sellability_at_scam_label",
        "Sellability",
        "What is sellability at the scam label?",
        Some(format!("{} cannot sell", percent(can_sell_false, can_sell_total))),
        Some(format!(
            "At label time, {} of {} scam labels cannot sell and {} can still sell. This answers label-time state; pre-scam sellability degradation needs time-sliced observations.",
            fmt_count(can_sell_false),
            fmt_count(can_sell_total),
            fmt_count(can_sell_true)
        )),
        "partial",
        Some("scam labels"),
        Some(can_sell_total),
        json!({
            "can_sell": distribution_payload(distributions, "scam_can_sell_at_label"),
            "can_buy": distribution_payload(distributions, "scam_can_buy_at_label")
        }),
        5,
    ));

    if let Some(stat) = numeric_stat(numeric_stats, "scam_age", "Liquidity ETH At Label") {
        questions.push(decision_question(
            "liquidity_state_at_label",
            "Liquidity",
            "What liquidity state do scams leave at the label?",
            Some(format!("{} median ETH", fmt_optional_number(stat.median))),
            Some(format!(
                "Scam labels usually occur after liquidity is already drained: median label liquidity is {} ETH and P90 is {} ETH.",
                fmt_optional_number(stat.median),
                fmt_optional_number(stat.p90)
            )),
            "answered",
            Some("scam labels"),
            Some(stat.count),
            json!({
                "stats": stat,
                "levels": distribution_payload(distributions, "scam_liquidity_at_label")
            }),
            6,
        ));
    }

    questions.push(decision_question(
        "direct_lp_no_observable_lp_warning",
        "Direct LP warning",
        "What share of direct LP removals have no observable pre-removal LP approval?",
        Some(format!("{}", percent(no_pre_approval, approval_total))),
        Some(format!(
            "{} direct LP removals have no pre-removal LP approval visible in the current feature export. These are the direct-removal cases where LP approval alone cannot warn us.",
            fmt_count(no_pre_approval)
        )),
        "answered",
        Some("direct LP removals"),
        Some(approval_total),
        json!({
            "rows": distribution_payload(distributions, "direct_lp_feature_scope")
        }),
        7,
    ));

    let horizon_rows = feature_table_horizon_rows(feature_tables)?;
    let first_positive = horizon_rows.iter().find(|row| {
        row.get("row_kind").and_then(|value| value.as_str()) == Some("positive")
            && row.get("horizon_blocks").and_then(|value| value.as_i64()) == Some(1)
    });
    let first_control = horizon_rows.iter().find(|row| {
        row.get("row_kind").and_then(|value| value.as_str()) == Some("control")
            && row.get("horizon_blocks").and_then(|value| value.as_i64()) == Some(1)
    });
    if let (Some(positive), Some(control)) = (first_positive, first_control) {
        let positive_pct = positive
            .get("approval_seen_percent")
            .and_then(|value| value.as_f64())
            .unwrap_or_default();
        let control_pct = control
            .get("approval_seen_percent")
            .and_then(|value| value.as_f64())
            .unwrap_or_default();
        questions.push(decision_question(
            "lp_approval_signal_in_controls",
            "Signal quality",
            "How noisy is LP approval when compared with non-scam controls?",
            Some(format!("{} vs {}", fmt_percent(positive_pct), fmt_percent(control_pct))),
            Some(format!(
                "At the 1-block as-of window, LP approval is seen in {} of direct-removal positive rows versus {} of control rows. The signal is strong but not unique to scams.",
                fmt_percent(positive_pct),
                fmt_percent(control_pct)
            )),
            "answered",
            Some("horizon rows"),
            None,
            json!({ "rows": horizon_rows }),
            8,
        ));
    }

    let active_target_rows: i64 = active_targets
        .iter()
        .filter(|row| row.horizon_active_observations.is_none())
        .map(|row| row.rows)
        .sum();
    let positive_rows = active_targets
        .iter()
        .filter(|row| row.row_kind == "pre_label_positive")
        .map(|row| row.rows)
        .sum::<i64>();
    questions.push(decision_question(
        "active_observation_target_base_rate",
        "Model target",
        "What is the current active-observation target base rate?",
        Some(format!("{}", percent(positive_rows, active_target_rows))),
        Some(format!(
            "{} of {} active-observation rows are positive across the near-future direct-removal horizons. This is the class balance for the first conditional model.",
            fmt_count(positive_rows),
            fmt_count(active_target_rows)
        )),
        "answered",
        Some("active-observation rows"),
        Some(active_target_rows),
        json!({
            "row_kinds": active_targets,
            "active_target": distribution_payload(distributions, "active_target"),
            "horizons": distribution_payload(distributions, "active_horizons"),
            "source_note": distribution_tables.contains_key("Active Observation Target Rows")
        }),
        9,
    ));

    Ok(questions)
}

fn parse_run(lines: &[&str], path: &Path) -> Result<RiskAtlasRun> {
    let mut run_id = None;
    let mut start_block = None;
    let mut end_block = None;
    let mut block_count = None;
    let mut status = "draft".to_string();
    let mut token_count = None;
    let mut pool_count = None;
    let mut scam_label_count = None;
    let mut direct_lp_feature_row_count = None;
    let mut active_observation_row_count = None;

    for line in lines {
        let line = line.trim();
        let values = backtick_values(line);
        if line.starts_with("- run_id:") {
            run_id = values.first().cloned();
        } else if line.starts_with("- range:") {
            start_block = values.first().and_then(|value| parse_i64(value).ok());
            end_block = values.get(1).and_then(|value| parse_i64(value).ok());
            block_count = values.get(2).and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- status:") {
            if let Some(value) = values.first() {
                status = value.to_string();
            }
        } else if line.starts_with("- tokens indexed:") {
            token_count = values.first().and_then(|value| parse_i64(value).ok());
            pool_count = values.get(1).and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- labels:") {
            scam_label_count = values.last().and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- label rows:") {
            scam_label_count = values.first().and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- direct LP features:") {
            direct_lp_feature_row_count = values.last().and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- direct LP feature rows:") {
            direct_lp_feature_row_count = values.first().and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- active observations:") {
            active_observation_row_count = values.last().and_then(|value| parse_i64(value).ok());
        } else if line.starts_with("- active observation rows:") {
            active_observation_row_count = values.first().and_then(|value| parse_i64(value).ok());
        }
    }

    let run_id = run_id.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("risk-atlas-report")
            .to_string()
    });

    Ok(RiskAtlasRun {
        run_id,
        chain: "ethereum".to_string(),
        source_kind: "markdown_distribution_report".to_string(),
        source_ref: Some(path.display().to_string()),
        start_block,
        end_block,
        block_count,
        token_count,
        pool_count,
        scam_label_count,
        direct_lp_feature_row_count,
        active_observation_row_count,
        status,
        generated_at: Utc::now(),
        metadata: json!({
            "importer": "token_lab_scam_risk_atlas::ingest::report",
            "source_report": path.display().to_string(),
        }),
    })
}

fn markdown_tables(lines: &[&str]) -> BTreeMap<String, MarkdownTable> {
    let mut tables = BTreeMap::new();
    let mut heading = String::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim();
        if let Some(next_heading) = markdown_heading(line) {
            heading = next_heading;
            index += 1;
            continue;
        }

        if is_table_line(line)
            && index + 1 < lines.len()
            && is_separator_line(lines[index + 1].trim())
            && !heading.is_empty()
        {
            let headers = table_cells(line);
            index += 2;
            let mut rows = Vec::new();
            while index < lines.len() && is_table_line(lines[index].trim()) {
                rows.push(table_cells(lines[index].trim()));
                index += 1;
            }
            let table = MarkdownTable { headers, rows };
            if let Some(header_key) = table
                .headers
                .first()
                .filter(|value| should_key_table_by_header(value))
            {
                tables.insert(header_key.clone(), table.clone());
            }
            tables.insert(heading.clone(), table);
            continue;
        }

        index += 1;
    }

    tables
}

fn should_key_table_by_header(value: &str) -> bool {
    !matches!(value, "Value" | "Metric" | "Bucket" | "Field" | "Row Kind")
}

fn markdown_heading(line: &str) -> Option<String> {
    if line.starts_with('#') {
        Some(line.trim_start_matches('#').trim().to_string())
    } else {
        None
    }
}

fn is_table_line(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|')
}

fn is_separator_line(line: &str) -> bool {
    is_table_line(line) && line.contains("---")
}

fn table_cells(line: &str) -> Vec<String> {
    line.trim_matches('|').split('|').map(clean_cell).collect()
}

fn clean_cell(value: &str) -> String {
    let value = value.trim().trim_matches('`').trim();
    if value.is_empty() {
        "(empty)".to_string()
    } else {
        value.to_string()
    }
}

fn distribution_sections() -> &'static [(&'static str, &'static str)] {
    &[
        ("Pool Eligibility Snapshot", "pool_eligibility"),
        ("All Pool Protocols", "all_pool_protocols"),
        ("All Pool Liquidity Levels", "all_pool_liquidity_levels"),
        ("All Pool Risk Labels", "all_pool_risk_labels"),
        (
            "All Pool Scam Mechanisms Including Empty",
            "all_pool_scam_mechanisms",
        ),
        ("Scam Mechanisms From Label Export", "scam_mechanisms"),
        ("Scam Mechanism Labels", "scam_mechanism_labels"),
        ("Scam Confidence", "scam_confidence"),
        (
            "Scam Time Buckets",
            atlas::time_to_scam::DISTRIBUTION_SECTION,
        ),
        ("Scam Protocols", "scam_protocols"),
        ("Scam Liquidity Level At Label", "scam_liquidity_at_label"),
        ("Scam Can Buy At Label", "scam_can_buy_at_label"),
        ("Scam Can Sell At Label", "scam_can_sell_at_label"),
        (
            "Direct LP Approval Seen Pre-Removal",
            "direct_lp_approval_pre_removal",
        ),
        ("Direct LP Feature Scope", "direct_lp_feature_scope"),
        ("Active Target", "active_target"),
        ("Active Horizons", "active_horizons"),
    ]
}

fn append_distribution(
    section: &str,
    table: &MarkdownTable,
    out: &mut Vec<DistributionBucket>,
) -> Result<()> {
    if table.headers.len() < 2 {
        return Ok(());
    }

    for (index, row) in table.rows.iter().enumerate() {
        if row.len() < 2 {
            continue;
        }
        let (count, share) = parse_count_and_share(&row[1])?;
        out.push(DistributionBucket {
            section: section.to_string(),
            bucket: row[0].clone(),
            count,
            share,
            sort_order: index as i32,
        });
    }
    Ok(())
}

fn append_numeric_stats(
    section: &str,
    table: &MarkdownTable,
    out: &mut Vec<NumericStat>,
) -> Result<()> {
    for (index, row) in table.rows.iter().enumerate() {
        if row.len() < 9 {
            continue;
        }
        out.push(NumericStat {
            section: section.to_string(),
            metric: row[0].clone(),
            count: parse_i64(&row[1])?,
            min: parse_optional_f64(&row[2])?,
            p25: parse_optional_f64(&row[3])?,
            median: parse_optional_f64(&row[4])?,
            p75: parse_optional_f64(&row[5])?,
            p90: parse_optional_f64(&row[6])?,
            p95: parse_optional_f64(&row[7])?,
            max: parse_optional_f64(&row[8])?,
            sort_order: index as i32,
        });
    }
    Ok(())
}

fn append_active_target_rows(
    table: &MarkdownTable,
    out: &mut Vec<ActiveTargetSummary>,
) -> Result<()> {
    for (index, row) in table.rows.iter().enumerate() {
        if row.len() < 3 {
            continue;
        }
        out.push(ActiveTargetSummary {
            row_kind: row[0].clone(),
            horizon_active_observations: None,
            rows: parse_i64(&row[1])?,
            unique_pools: Some(parse_i64(&row[2])?),
            positives: None,
            negatives: None,
            sort_order: index as i32,
        });
    }
    Ok(())
}

fn append_active_horizon_rows(
    table: &MarkdownTable,
    out: &mut Vec<ActiveTargetSummary>,
) -> Result<()> {
    for (index, row) in table.rows.iter().enumerate() {
        if row.len() < 2 {
            continue;
        }
        let (rows, _) = parse_count_and_share(&row[1])?;
        out.push(ActiveTargetSummary {
            row_kind: "horizon_total".to_string(),
            horizon_active_observations: Some(parse_i32(&row[0])?),
            rows,
            unique_pools: None,
            positives: None,
            negatives: None,
            sort_order: index as i32,
        });
    }
    Ok(())
}

fn decision_question(
    question_id: &str,
    category: &str,
    question: &str,
    headline: Option<String>,
    answer: Option<String>,
    status: &str,
    denominator_label: Option<&str>,
    denominator_count: Option<i64>,
    payload: Value,
    sort_order: i32,
) -> DecisionQuestion {
    DecisionQuestion {
        question_id: question_id.to_string(),
        category: category.to_string(),
        question: question.to_string(),
        headline,
        answer,
        status: status.to_string(),
        denominator_label: denominator_label.map(str::to_string),
        denominator_count,
        payload,
        sort_order,
    }
}

fn distribution_count(distributions: &[DistributionBucket], section: &str, bucket: &str) -> i64 {
    distributions
        .iter()
        .find(|row| row.section == section && row.bucket == bucket)
        .map(|row| row.count)
        .unwrap_or_default()
}

fn distribution_payload(distributions: &[DistributionBucket], section: &str) -> Vec<Value> {
    distributions
        .iter()
        .filter(|row| row.section == section)
        .map(|row| {
            json!({
                "bucket": row.bucket,
                "label": labelize(&row.bucket),
                "count": row.count,
                "share": row.share,
                "sort_order": row.sort_order,
            })
        })
        .collect()
}

fn numeric_stat<'a>(
    numeric_stats: &'a [NumericStat],
    section: &str,
    metric: &str,
) -> Option<&'a NumericStat> {
    numeric_stats
        .iter()
        .find(|row| row.section == section && row.metric == metric)
}

fn feature_table_count_rows(
    feature_tables: Option<&BTreeMap<String, MarkdownTable>>,
    heading: &str,
) -> Result<Vec<Value>> {
    let Some(table) = feature_tables.and_then(|tables| tables.get(heading)) else {
        return Ok(Vec::new());
    };
    table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            if row.len() < 2 {
                return None;
            }
            Some((index, row))
        })
        .map(|(index, row)| {
            let (count, share) = parse_count_and_share(&row[1])?;
            Ok(json!({
                "bucket": row[0],
                "label": labelize(&row[0]),
                "count": count,
                "share": share,
                "sort_order": index,
            }))
        })
        .collect()
}

fn feature_table_horizon_rows(
    feature_tables: Option<&BTreeMap<String, MarkdownTable>>,
) -> Result<Vec<Value>> {
    let Some(table) = feature_tables.and_then(|tables| tables.get("Horizon Signal Availability"))
    else {
        return Ok(Vec::new());
    };
    table
        .rows
        .iter()
        .filter(|row| row.len() >= 9)
        .map(|row| {
            Ok(json!({
                "row_kind": row[0],
                "horizon_blocks": parse_i64(&row[1])?,
                "rows": parse_i64(&row[2])?,
                "lp_approval_seen": parse_i64(&row[3])?,
                "approval_seen_percent": parse_f64(&row[4])?,
                "median_activity_density": parse_optional_f64(&row[5])?,
                "median_tx_last_10": parse_optional_f64(&row[6])?,
                "median_tx_last_100": parse_optional_f64(&row[7])?,
                "median_net_buy_eth": parse_optional_f64(&row[8])?,
            }))
        })
        .collect()
}

fn labelize(value: &str) -> String {
    let label = value
        .replace("(empty)", "Empty")
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    label
        .replace("Lp", "LP")
        .replace("Eth", "ETH")
        .replace("P90", "P90")
}

fn percent(count: i64, total: i64) -> String {
    if total <= 0 {
        "-".to_string()
    } else {
        fmt_percent((count as f64 / total as f64) * 100.0)
    }
}

fn fmt_percent(value: f64) -> String {
    format!("{value:.1}%")
}

fn fmt_count(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{sign}{}", out.chars().rev().collect::<String>())
}

fn fmt_optional_number(value: Option<f64>) -> String {
    value.map(fmt_number).unwrap_or_else(|| "-".to_string())
}

fn fmt_number(value: f64) -> String {
    if value.abs() >= 1000.0 {
        fmt_count(value.round() as i64)
    } else if value.fract().abs() < 0.000_001 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn default_model_readiness() -> Vec<ModelReadinessItem> {
    vec![
        ModelReadinessItem {
            name: "risk_atlas_distribution_import".to_string(),
            status: atlas::model_readiness::STATUS_READY.to_string(),
            detail: Some("100K distribution report can seed the Risk Atlas DB.".to_string()),
            sort_order: 0,
        },
        ModelReadinessItem {
            name: "source_feature_contract".to_string(),
            status: atlas::model_readiness::STATUS_NEEDS_REVIEW.to_string(),
            detail: Some(
                "Model rows should come from eth_token::token_analytics before the next training pass."
                    .to_string(),
            ),
            sort_order: 1,
        },
        ModelReadinessItem {
            name: "review_examples".to_string(),
            status: atlas::model_readiness::STATUS_BLOCKED.to_string(),
            detail: Some(
                "Flat-file review examples were removed; add a DB-backed review queue importer."
                    .to_string(),
            ),
            sort_order: 2,
        },
    ]
}

fn backtick_values(line: &str) -> Vec<String> {
    line.split('`')
        .enumerate()
        .filter_map(|(index, value)| {
            if index % 2 == 1 {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_count_and_share(value: &str) -> Result<(i64, Option<f64>)> {
    let count_text = value
        .split('(')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches('`');
    let count = parse_i64(count_text)?;
    let share = value
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once('%'))
        .map(|(share, _)| parse_f64(share).map(|value| value / 100.0))
        .transpose()?;
    Ok((count, share))
}

fn parse_optional_f64(value: &str) -> Result<Option<f64>> {
    let value = value.trim();
    if value == "-" || value.is_empty() || value == "(empty)" {
        Ok(None)
    } else {
        Ok(Some(parse_f64(value)?))
    }
}

fn parse_i32(value: &str) -> Result<i32> {
    let parsed = parse_i64(value)?;
    i32::try_from(parsed).map_err(|_| eyre!("value is too large for i32: {value}"))
}

fn parse_i64(value: &str) -> Result<i64> {
    normalize_number(value)
        .parse::<i64>()
        .map_err(|error| eyre!("failed to parse integer `{value}`: {error}"))
}

fn parse_f64(value: &str) -> Result<f64> {
    normalize_number(value)
        .parse::<f64>()
        .map_err(|error| eyre!("failed to parse float `{value}`: {error}"))
}

fn normalize_number(value: &str) -> String {
    value
        .trim()
        .trim_matches('`')
        .replace(',', "")
        .replace('%', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_count_and_share() {
        let (count, share) = parse_count_and_share("5,146 (47.7%)").unwrap();
        assert_eq!(count, 5146);
        assert!((share.unwrap() - 0.477).abs() < f64::EPSILON);
        assert_eq!(parse_count_and_share("7,881").unwrap(), (7881, None));
    }
}
