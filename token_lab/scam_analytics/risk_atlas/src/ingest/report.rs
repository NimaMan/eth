use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use eyre::{eyre, Result};
use serde_json::json;

use crate::atlas;
use crate::db::schema::{
    ActiveTargetSummary, DistributionBucket, ModelReadinessItem, NumericStat, ReviewExample,
    RiskAtlasRun,
};

pub const DEFAULT_100K_DISTRIBUTION_REPORT: &str =
    "token_lab/scam_analytics/artifacts/reports/scam_100k_25007276_25107275_distribution.md";

#[derive(Clone, Debug, PartialEq)]
pub struct RiskAtlasReportImport {
    pub run: RiskAtlasRun,
    pub distributions: Vec<DistributionBucket>,
    pub numeric_stats: Vec<NumericStat>,
    pub active_targets: Vec<ActiveTargetSummary>,
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

    Ok(RiskAtlasReportImport {
        run,
        distributions,
        numeric_stats,
        active_targets,
        review_examples: Vec::new(),
        model_readiness: default_model_readiness(),
    })
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
            tables.insert(heading.clone(), MarkdownTable { headers, rows });
            continue;
        }

        index += 1;
    }

    tables
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
