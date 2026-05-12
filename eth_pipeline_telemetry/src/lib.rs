use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineImpact {
    None,
    PoolLocal,
    TokenLocal,
    BlockLocal,
    ServiceDegraded,
    ServiceDown,
    TradingBlocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineHealthStatus {
    Healthy,
    Watch,
    Degraded,
    Down,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineIssue {
    pub schema_version: u16,
    pub event_type: String,
    pub event_id: String,
    pub dedupe_key: String,
    pub ts_unix_ms: u128,
    pub run_id: Option<String>,
    pub service: String,
    pub stage: String,
    pub component: String,
    pub severity: PipelineSeverity,
    pub impact: PipelineImpact,
    pub code: String,
    pub fatal: bool,
    pub retryable: bool,
    pub block_number: Option<u64>,
    pub tx_index: Option<u64>,
    pub tx_hash: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    #[serde(default)]
    pub context: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineHealth {
    pub schema_version: u16,
    pub event_type: String,
    pub event_id: String,
    pub ts_unix_ms: u128,
    pub run_id: Option<String>,
    pub service: String,
    pub stage: String,
    pub component: String,
    pub status: PipelineHealthStatus,
    pub current_block: Option<u64>,
    pub head_block: Option<u64>,
    pub lag_blocks: Option<u64>,
    pub heartbeat_age_ms: Option<u128>,
    #[serde(default)]
    pub metrics: BTreeMap<String, Value>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, PipelineHealthStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineBottleneckSample {
    pub schema_version: u16,
    pub event_type: String,
    pub event_id: String,
    pub ts_unix_ms: u128,
    pub run_id: Option<String>,
    pub service: String,
    pub stage: String,
    pub component: String,
    pub severity: PipelineSeverity,
    pub block_number: Option<u64>,
    pub duration_ms: u128,
    pub threshold_ms: Option<u128>,
    #[serde(default)]
    pub work_units: BTreeMap<String, Value>,
    #[serde(default)]
    pub breakdown_ms: BTreeMap<String, Value>,
    pub message: String,
}

pub trait TelemetrySink: Send + Sync {
    fn emit_issue(&self, issue: &PipelineIssue);
    fn emit_health(&self, health: &PipelineHealth);
    fn emit_bottleneck(&self, sample: &PipelineBottleneckSample);
}

#[derive(Default)]
pub struct NoopTelemetrySink;

impl TelemetrySink for NoopTelemetrySink {
    fn emit_issue(&self, _issue: &PipelineIssue) {}
    fn emit_health(&self, _health: &PipelineHealth) {}
    fn emit_bottleneck(&self, _sample: &PipelineBottleneckSample) {}
}

#[derive(Default)]
pub struct TracingTelemetrySink;

impl TelemetrySink for TracingTelemetrySink {
    fn emit_issue(&self, issue: &PipelineIssue) {
        match issue.severity {
            PipelineSeverity::Info => tracing::info!(
                target: "pipeline_issue",
                issue = ?issue,
                "pipeline issue"
            ),
            PipelineSeverity::Warn => tracing::warn!(
                target: "pipeline_issue",
                issue = ?issue,
                "pipeline issue"
            ),
            PipelineSeverity::Error | PipelineSeverity::Critical => tracing::error!(
                target: "pipeline_issue",
                issue = ?issue,
                "pipeline issue"
            ),
        }
    }

    fn emit_health(&self, health: &PipelineHealth) {
        tracing::info!(target: "pipeline_health", health = ?health, "pipeline health");
    }

    fn emit_bottleneck(&self, sample: &PipelineBottleneckSample) {
        tracing::warn!(
            target: "pipeline_bottleneck",
            sample = ?sample,
            "pipeline bottleneck sample"
        );
    }
}

pub struct MultiTelemetrySink {
    sinks: Vec<Arc<dyn TelemetrySink>>,
}

impl MultiTelemetrySink {
    pub fn new(sinks: Vec<Arc<dyn TelemetrySink>>) -> Self {
        Self { sinks }
    }
}

impl TelemetrySink for MultiTelemetrySink {
    fn emit_issue(&self, issue: &PipelineIssue) {
        for sink in &self.sinks {
            sink.emit_issue(issue);
        }
    }

    fn emit_health(&self, health: &PipelineHealth) {
        for sink in &self.sinks {
            sink.emit_health(health);
        }
    }

    fn emit_bottleneck(&self, sample: &PipelineBottleneckSample) {
        for sink in &self.sinks {
            sink.emit_bottleneck(sample);
        }
    }
}

pub struct JsonlTelemetrySink {
    issues: Mutex<BufWriter<File>>,
    health: Mutex<BufWriter<File>>,
    bottlenecks: Mutex<BufWriter<File>>,
}

impl JsonlTelemetrySink {
    pub fn open(dir: impl AsRef<Path>) -> eyre::Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            issues: Mutex::new(BufWriter::new(open_append(
                dir.join("pipeline_issues.jsonl"),
            )?)),
            health: Mutex::new(BufWriter::new(open_append(
                dir.join("pipeline_health.jsonl"),
            )?)),
            bottlenecks: Mutex::new(BufWriter::new(open_append(
                dir.join("pipeline_bottlenecks.jsonl"),
            )?)),
        })
    }
}

impl TelemetrySink for JsonlTelemetrySink {
    fn emit_issue(&self, issue: &PipelineIssue) {
        write_jsonl(&self.issues, issue);
    }

    fn emit_health(&self, health: &PipelineHealth) {
        write_jsonl(&self.health, health);
    }

    fn emit_bottleneck(&self, sample: &PipelineBottleneckSample) {
        write_jsonl(&self.bottlenecks, sample);
    }
}

static GLOBAL_SINK: OnceLock<Arc<dyn TelemetrySink>> = OnceLock::new();

pub fn init_global_sink(sink: Arc<dyn TelemetrySink>) -> bool {
    GLOBAL_SINK.set(sink).is_ok()
}

pub fn emit_issue(issue: &PipelineIssue) {
    if let Some(sink) = GLOBAL_SINK.get() {
        sink.emit_issue(issue);
    } else {
        TracingTelemetrySink.emit_issue(issue);
    }
}

pub fn emit_health(health: &PipelineHealth) {
    if let Some(sink) = GLOBAL_SINK.get() {
        sink.emit_health(health);
    } else {
        TracingTelemetrySink.emit_health(health);
    }
}

pub fn emit_bottleneck(sample: &PipelineBottleneckSample) {
    if let Some(sink) = GLOBAL_SINK.get() {
        sink.emit_bottleneck(sample);
    } else {
        TracingTelemetrySink.emit_bottleneck(sample);
    }
}

impl PipelineIssue {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        service: impl Into<String>,
        stage: impl Into<String>,
        component: impl Into<String>,
        severity: PipelineSeverity,
        impact: PipelineImpact,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut issue = Self {
            schema_version: SCHEMA_VERSION,
            event_type: "issue".to_string(),
            event_id: String::new(),
            dedupe_key: String::new(),
            ts_unix_ms: now_unix_ms(),
            run_id: None,
            service: service.into(),
            stage: stage.into(),
            component: component.into(),
            severity,
            impact,
            code: code.into(),
            fatal: false,
            retryable: false,
            block_number: None,
            tx_index: None,
            tx_hash: None,
            token_address: None,
            pool_address: None,
            message: message.into(),
            detail: None,
            context: BTreeMap::new(),
        };
        issue.refresh_ids();
        issue
    }

    pub fn live_transaction_error(
        run_id: Option<String>,
        block_number: u64,
        tx_index: u64,
        tx_hash: String,
        message: String,
    ) -> Self {
        let mut issue = classify_live_transaction_error(&message);
        issue.run_id = run_id;
        issue.block_number = Some(block_number);
        issue.tx_index = Some(tx_index);
        issue.tx_hash = Some(tx_hash);
        issue.detail = Some(message);
        issue.refresh_ids();
        issue
    }

    pub fn refresh_ids(&mut self) {
        self.dedupe_key = issue_dedupe_key(self);
        let event_key = format!(
            "{}|{}|{}|{}|{}",
            self.dedupe_key,
            self.block_number
                .map(|value| value.to_string())
                .unwrap_or_default(),
            self.tx_index
                .map(|value| value.to_string())
                .unwrap_or_default(),
            self.tx_hash.clone().unwrap_or_default(),
            self.ts_unix_ms
        );
        self.event_id = stable_id(&event_key);
    }
}

impl PipelineHealth {
    pub fn new(
        service: impl Into<String>,
        stage: impl Into<String>,
        component: impl Into<String>,
        status: PipelineHealthStatus,
    ) -> Self {
        let mut health = Self {
            schema_version: SCHEMA_VERSION,
            event_type: "health".to_string(),
            event_id: String::new(),
            ts_unix_ms: now_unix_ms(),
            run_id: None,
            service: service.into(),
            stage: stage.into(),
            component: component.into(),
            status,
            current_block: None,
            head_block: None,
            lag_blocks: None,
            heartbeat_age_ms: None,
            metrics: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        };
        health.event_id = stable_id(&format!(
            "{}|{}|{}|{}",
            health.service, health.stage, health.component, health.ts_unix_ms
        ));
        health
    }
}

impl PipelineBottleneckSample {
    pub fn new(
        service: impl Into<String>,
        stage: impl Into<String>,
        component: impl Into<String>,
        duration_ms: u128,
        message: impl Into<String>,
    ) -> Self {
        let mut sample = Self {
            schema_version: SCHEMA_VERSION,
            event_type: "bottleneck_sample".to_string(),
            event_id: String::new(),
            ts_unix_ms: now_unix_ms(),
            run_id: None,
            service: service.into(),
            stage: stage.into(),
            component: component.into(),
            severity: PipelineSeverity::Warn,
            block_number: None,
            duration_ms,
            threshold_ms: None,
            work_units: BTreeMap::new(),
            breakdown_ms: BTreeMap::new(),
            message: message.into(),
        };
        sample.event_id = stable_id(&format!(
            "{}|{}|{}|{}|{}|{}",
            sample.service,
            sample.stage,
            sample.component,
            sample
                .block_number
                .map(|value| value.to_string())
                .unwrap_or_default(),
            sample.duration_ms,
            sample.ts_unix_ms
        ));
        sample
    }
}

pub fn classify_live_transaction_error(message: &str) -> PipelineIssue {
    if message.contains("Uniswap V3 factory reports pool")
        && message.contains("configuration provided")
    {
        let mut issue = PipelineIssue::new(
            "eth_token_server",
            "live_tracker",
            "pool_buy_sell_sim",
            PipelineSeverity::Warn,
            PipelineImpact::PoolLocal,
            "uniswap_v3_pool_identity_mismatch",
            "Uniswap V3 factory pool mismatch",
        );
        issue.fatal = false;
        issue.retryable = false;
        issue.token_address = word_after(message, " for token ");
        issue.pool_address = word_after(message, "configuration provided ");
        if let Some(resolved) = word_after(message, "reports pool ") {
            issue
                .context
                .insert("resolved_pool_address".to_string(), Value::String(resolved));
        }
        if let Some(denom) = word_after(message, " / denom ") {
            issue
                .context
                .insert("denom_address".to_string(), Value::String(denom));
        }
        if let Some(fee_tier) = word_after(message, " at fee tier ") {
            issue
                .context
                .insert("fee_tier".to_string(), Value::String(fee_tier));
        }
        issue.context.insert(
            "protocol".to_string(),
            Value::String("uniswap_v3".to_string()),
        );
        issue.refresh_ids();
        return issue;
    }

    let mut issue = PipelineIssue::new(
        "eth_token_server",
        "live_tracker",
        "token_block_apply",
        PipelineSeverity::Warn,
        PipelineImpact::BlockLocal,
        "token_transaction_update_failed",
        "Token transaction update failed",
    );
    issue.detail = Some(message.to_string());
    issue.refresh_ids();
    issue
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn open_append(path: impl AsRef<Path>) -> eyre::Result<File> {
    Ok(OpenOptions::new().create(true).append(true).open(path)?)
}

fn write_jsonl<T: Serialize>(writer: &Mutex<BufWriter<File>>, value: &T) {
    let Ok(mut writer) = writer.lock() else {
        tracing::error!(target: "pipeline_telemetry", "telemetry writer lock poisoned");
        return;
    };
    if let Err(error) = serde_json::to_writer(&mut *writer, value).and_then(|_| {
        writer.write_all(b"\n").map_err(serde_json::Error::io)?;
        writer.flush().map_err(serde_json::Error::io)
    }) {
        tracing::error!(target: "pipeline_telemetry", error = %error, "failed to write telemetry jsonl");
    }
}

fn issue_dedupe_key(issue: &PipelineIssue) -> String {
    let mut parts = vec![
        issue.stage.clone(),
        issue.component.clone(),
        issue.code.clone(),
        issue.token_address.clone().unwrap_or_default(),
        issue.pool_address.clone().unwrap_or_default(),
    ];
    for key in [
        "protocol",
        "fee_tier",
        "denom_address",
        "resolved_pool_address",
    ] {
        if let Some(value) = issue.context.get(key).and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    parts.join("|")
}

fn stable_id(value: &str) -> String {
    let mut hasher = StableHasher::default();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct StableHasher(u64);

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
}

fn word_after(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let rest = text[start..].trim_start();
    let value = rest
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ')' | '('))
        .next()?
        .trim_matches(|ch| matches!(ch, ',' | ';' | '.'));
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_uniswap_v3_pool_identity_mismatch() {
        let message = "Uniswap V3 factory reports pool 0xA9593EBF2F63082D698982990dB3C49322510A91 but configuration provided 0x067eEFfDe88550B5AC4fEd939dFd2c35d550Fb99 for token 0xF74cF56fC2c885dDdD853428852DEbB73f6865A0 / denom 0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c at fee tier 100 and block 25074189";

        let issue = classify_live_transaction_error(message);

        assert_eq!(issue.severity, PipelineSeverity::Warn);
        assert_eq!(issue.impact, PipelineImpact::PoolLocal);
        assert_eq!(issue.code, "uniswap_v3_pool_identity_mismatch");
        assert!(!issue.fatal);
        assert_eq!(
            issue.token_address.as_deref(),
            Some("0xF74cF56fC2c885dDdD853428852DEbB73f6865A0")
        );
        assert_eq!(
            issue.pool_address.as_deref(),
            Some("0x067eEFfDe88550B5AC4fEd939dFd2c35d550Fb99")
        );
        assert_eq!(
            issue
                .context
                .get("resolved_pool_address")
                .and_then(Value::as_str),
            Some("0xA9593EBF2F63082D698982990dB3C49322510A91")
        );
        assert_eq!(
            issue.context.get("fee_tier").and_then(Value::as_str),
            Some("100")
        );
    }
}
