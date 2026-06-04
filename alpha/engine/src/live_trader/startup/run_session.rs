use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use eyre::{eyre, Result, WrapErr};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::constants::{ALPHA_TRADER_SESSION_DIR_CONFIG, DEFAULT_ALPHA_TRADER_SESSION_DIR};
use super::support::{optional_shared_config_value, sanitize_path_segment, TraderExecutionMode};

const SESSION_SCHEMA: &str = "alpha_live_trader_run_session_v1";

pub(super) struct RunSessionInput<'a> {
    pub(super) explicit_run_id: Option<&'a str>,
    pub(super) shared_config: &'a HashMap<String, String>,
    pub(super) execution_mode: TraderExecutionMode,
    pub(super) strategy_set: Option<&'a str>,
    pub(super) observation_strategy_name: &'a str,
}

pub(super) struct RunSession {
    run_id: String,
    source: RunSessionSource,
    path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum RunSessionSource {
    ExplicitCli,
    Generated,
    ReusedActiveSession,
}

impl RunSessionSource {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ExplicitCli => "explicit_cli",
            Self::Generated => "generated_session",
            Self::ReusedActiveSession => "reused_active_session",
        }
    }
}

impl RunSession {
    pub(super) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(super) fn source(&self) -> RunSessionSource {
        self.source
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(super) fn mark_finished(&self, status: &'static str) -> Result<()> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        let mut state = read_session_state(path)?.unwrap_or_else(|| RunSessionState {
            schema: SESSION_SCHEMA.to_string(),
            status: "active".to_string(),
            run_id: self.run_id.clone(),
            execution_mode: "unknown".to_string(),
            strategy_key: "unknown".to_string(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            pid: std::process::id(),
            reuse_count: 0,
        });
        if state.run_id != self.run_id {
            warn!(
                expected_run_id = %self.run_id,
                found_run_id = %state.run_id,
                session_path = %path.display(),
                "not marking alpha trader run session finished because the session file changed"
            );
            return Ok(());
        }
        state.status = status.to_string();
        state.updated_at = Utc::now().to_rfc3339();
        state.pid = std::process::id();
        write_session_state(path, &state)?;
        info!(
            run_id = %state.run_id,
            status = %state.status,
            session_path = %path.display(),
            "marked alpha trader run session finished"
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RunSessionState {
    schema: String,
    status: String,
    run_id: String,
    execution_mode: String,
    strategy_key: String,
    created_at: String,
    updated_at: String,
    pid: u32,
    reuse_count: u64,
}

pub(super) fn resolve_alpha_trader_run_session(input: RunSessionInput<'_>) -> Result<RunSession> {
    if let Some(run_id) = input
        .explicit_run_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(RunSession {
            run_id: run_id.to_string(),
            source: RunSessionSource::ExplicitCli,
            path: None,
        });
    }

    let session_dir =
        optional_shared_config_value(input.shared_config, ALPHA_TRADER_SESSION_DIR_CONFIG)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_ALPHA_TRADER_SESSION_DIR));
    fs::create_dir_all(&session_dir)
        .wrap_err_with(|| format!("failed to create run session dir {}", session_dir.display()))?;

    let strategy_key = input
        .strategy_set
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(input.observation_strategy_name);
    let run_base = semantic_run_base(input.execution_mode, strategy_key);
    let session_key = format!("{}-{run_base}", input.execution_mode.label());
    let session_path = session_dir.join(format!("{}.json", sanitize_path_segment(&session_key)));

    if let Some(mut state) = read_session_state(&session_path)? {
        if state.schema == SESSION_SCHEMA
            && state.status == "active"
            && !state.run_id.trim().is_empty()
        {
            if state.pid != std::process::id() && process_alive(state.pid) {
                return Err(eyre!(
                    "alpha trader session {} is already active for pid {}; refusing duplicate process",
                    session_path.display(),
                    state.pid
                ));
            }
            state.updated_at = Utc::now().to_rfc3339();
            state.pid = std::process::id();
            state.reuse_count = state.reuse_count.saturating_add(1);
            write_session_state(&session_path, &state)?;
            info!(
                run_id = %state.run_id,
                execution_mode = %state.execution_mode,
                strategy_key = %state.strategy_key,
                session_path = %session_path.display(),
                reuse_count = state.reuse_count,
                "reusing active alpha trader run session"
            );
            return Ok(RunSession {
                run_id: state.run_id,
                source: RunSessionSource::ReusedActiveSession,
                path: Some(session_path),
            });
        }
    }

    let now = Utc::now();
    let run_id = format!("{}-{}", run_base, run_timestamp(&now));
    let state = RunSessionState {
        schema: SESSION_SCHEMA.to_string(),
        status: "active".to_string(),
        run_id: run_id.clone(),
        execution_mode: input.execution_mode.label().to_string(),
        strategy_key: strategy_key.to_string(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        pid: std::process::id(),
        reuse_count: 0,
    };
    write_session_state(&session_path, &state)?;
    info!(
        run_id = %run_id,
        execution_mode = %input.execution_mode.label(),
        strategy_key = strategy_key,
        session_path = %session_path.display(),
        "created alpha trader run session"
    );
    Ok(RunSession {
        run_id,
        source: RunSessionSource::Generated,
        path: Some(session_path),
    })
}

fn semantic_run_base(execution_mode: TraderExecutionMode, strategy_key: &str) -> String {
    let suffix = match execution_mode {
        TraderExecutionMode::ChainSim => "chain-sim-block-frame",
        TraderExecutionMode::EthTxExecutorReal => "live-real-broadcast",
    };
    format!("{}-{suffix}", sanitize_path_segment(strategy_key))
}

fn run_timestamp(now: &chrono::DateTime<Utc>) -> String {
    format!(
        "{}{:09}Z",
        now.format("%Y%m%d-%H%M%S"),
        now.timestamp_subsec_nanos()
    )
}

fn read_session_state(path: &Path) -> Result<Option<RunSessionState>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read run session file {}", path.display()))?;
    let state = serde_json::from_str(&contents)
        .wrap_err_with(|| format!("failed to parse run session file {}", path.display()))?;
    Ok(Some(state))
}

fn write_session_state(path: &Path, state: &RunSessionState) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre!("run session path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .wrap_err_with(|| format!("failed to create run session dir {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre!("run session path has no file name: {}", path.display()))?;
    let tmp_path = parent.join(format!("{file_name}.tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(state).wrap_err("failed to encode run session state")?;
    fs::write(&tmp_path, bytes)
        .wrap_err_with(|| format!("failed to write run session file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).wrap_err_with(|| {
        format!(
            "failed to replace run session file {} with {}",
            path.display(),
            tmp_path.display()
        )
    })?;
    Ok(())
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    PathBuf::from(format!("/proc/{pid}")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_real_run_id_uses_strategy_and_mode() {
        assert_eq!(
            semantic_run_base(
                TraderExecutionMode::EthTxExecutorReal,
                "alpha11-univ2-lp30-pool-update-block-hold16"
            ),
            "alpha11-univ2-lp30-pool-update-block-hold16-live-real-broadcast"
        );
    }

    #[test]
    fn stopped_session_generates_new_run() {
        let dir = std::env::temp_dir().join(format!(
            "alpha-session-test-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut config = HashMap::new();
        config.insert(
            ALPHA_TRADER_SESSION_DIR_CONFIG.to_string(),
            dir.to_string_lossy().to_string(),
        );
        let first = resolve_alpha_trader_run_session(RunSessionInput {
            explicit_run_id: None,
            shared_config: &config,
            execution_mode: TraderExecutionMode::ChainSim,
            strategy_set: Some("strategy-a"),
            observation_strategy_name: "strategy-a",
        })
        .unwrap();
        first.mark_finished("stopped").unwrap();
        let second = resolve_alpha_trader_run_session(RunSessionInput {
            explicit_run_id: None,
            shared_config: &config,
            execution_mode: TraderExecutionMode::ChainSim,
            strategy_set: Some("strategy-a"),
            observation_strategy_name: "strategy-a",
        })
        .unwrap();
        assert_ne!(first.run_id(), second.run_id());
        let _ = fs::remove_dir_all(dir);
    }
}
