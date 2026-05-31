use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc};

use anyhow::{bail, Context, Result};
use serde_json::json;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};
use tracing::{error, info, warn};
use tx_executor::{
    signer::{
        LocalTransactionSigner, SignerStatus, SignerWireRequest, SignerWireResponse,
        TransactionSigner, ETH_SIGNER_WIRE_SCHEMA,
    },
    types::PreparedDirectRawTransaction,
};

use super::{
    config::{EthSignerConfig, EthSignerKeyBackend},
    journal::EthSignerJournal,
    policy::evaluate_signer_policy,
};

#[derive(Clone)]
struct EthSignerState {
    config: Arc<EthSignerConfig>,
    signer: Arc<LocalTransactionSigner>,
    journal: EthSignerJournal,
}

pub async fn run_eth_signer(config: EthSignerConfig) -> Result<()> {
    let signer = load_signer(&config).await?;
    let signer_address = signer.address();
    if let Some(expected) = config.expected_signer_address.as_ref() {
        let expected: ethers_core::types::Address = expected
            .parse()
            .with_context(|| format!("invalid configured signer address {expected:?}"))?;
        if expected != signer_address {
            bail!(
                "configured signer address {:?} does not match loaded key {:?}",
                expected,
                signer_address
            );
        }
    }

    prepare_socket_path(&config.socket_path).await?;
    let listener = UnixListener::bind(&config.socket_path)
        .with_context(|| format!("failed to bind {}", config.socket_path.display()))?;
    std::fs::set_permissions(
        &config.socket_path,
        std::fs::Permissions::from_mode(config.socket_mode),
    )
    .with_context(|| {
        format!(
            "failed to set socket mode {:o} on {}",
            config.socket_mode,
            config.socket_path.display()
        )
    })?;

    info!(
        socket = %config.socket_path.display(),
        mode = format_args!("{:o}", config.socket_mode),
        signer = ?signer_address,
        chain_id = config.chain_id,
        "ETH tx signer listening"
    );

    let state = EthSignerState {
        journal: EthSignerJournal::new(config.journal_path.clone()),
        config: Arc::new(config),
        signer: Arc::new(signer),
    };

    loop {
        let (stream, _addr) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(state, stream).await {
                warn!(error = %error, "failed to handle ETH signer request");
            }
        });
    }
}

async fn load_signer(config: &EthSignerConfig) -> Result<LocalTransactionSigner> {
    match &config.key_backend {
        EthSignerKeyBackend::Env { private_key_env } => {
            LocalTransactionSigner::from_env(private_key_env, config.chain_id)
                .map_err(|error| anyhow::anyhow!(error.to_string()))
        }
        EthSignerKeyBackend::Keystore {
            path,
            password_file,
            password_env,
        } => {
            let password =
                load_keystore_password(password_file.as_deref(), password_env.as_deref()).await?;
            LocalTransactionSigner::from_keystore(path, password, config.chain_id)
                .map_err(|error| anyhow::anyhow!(error.to_string()))
        }
    }
}

async fn load_keystore_password(
    password_file: Option<&Path>,
    password_env: Option<&str>,
) -> Result<String> {
    if let Some(path) = password_file {
        let value = fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read keystore password file {}", path.display()))?;
        return Ok(value.trim_end_matches(['\r', '\n']).to_string());
    }

    if let Some(env_key) = password_env {
        let value = std::env::var(env_key)
            .with_context(|| format!("environment variable {env_key} is not set"))?;
        if value.trim().is_empty() {
            bail!("environment variable {env_key} is empty");
        }
        return Ok(value);
    }

    bail!("no keystore password source configured");
}

async fn prepare_socket_path(socket_path: &Path) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create socket dir {}", parent.display()))?;
    }

    match fs::remove_file(socket_path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to remove stale socket {}", socket_path.display())),
    }
}

async fn handle_connection(state: EthSignerState, stream: UnixStream) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request = Vec::new();
    let read = reader
        .read_until(b'\n', &mut request)
        .await
        .context("failed to read signer request")?;
    if read == 0 {
        return Ok(());
    }

    let response = match serde_json::from_slice::<SignerWireRequest>(trim_ascii(&request)) {
        Ok(request) => handle_request(&state, request).await,
        Err(error) => SignerWireResponse::error(format!("invalid signer request JSON: {error}")),
    };

    let mut stream = reader.into_inner();
    let mut encoded = serde_json::to_vec(&response)?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .context("failed to write signer response")?;
    stream.shutdown().await.ok();
    Ok(())
}

async fn handle_request(state: &EthSignerState, request: SignerWireRequest) -> SignerWireResponse {
    match request {
        SignerWireRequest::Status { schema } => {
            if schema != ETH_SIGNER_WIRE_SCHEMA {
                return SignerWireResponse::error(format!(
                    "unexpected signer request schema {schema:?}"
                ));
            }
            SignerWireResponse::status(SignerStatus {
                ready: true,
                signer: state.signer.address(),
                chain_id: state.config.chain_id,
            })
        }
        SignerWireRequest::SignDirectRaw {
            schema,
            transaction,
        } => {
            if schema != ETH_SIGNER_WIRE_SCHEMA {
                return SignerWireResponse::error(format!(
                    "unexpected signer request schema {schema:?}"
                ));
            }
            sign_direct_raw(state, transaction).await
        }
    }
}

async fn sign_direct_raw(
    state: &EthSignerState,
    transaction: PreparedDirectRawTransaction,
) -> SignerWireResponse {
    let signer = state.signer.address();
    if let Err(reasons) = evaluate_signer_policy(&state.config.policy, signer, &transaction) {
        let reason = reasons.join("; ");
        if let Err(error) = state
            .journal
            .record("rejected", signer, Some(&transaction), &reasons, json!({}))
            .await
        {
            error!(error = %error, "failed to record signer rejection");
        }
        return SignerWireResponse::error(reason);
    }

    if let Err(error) = state
        .journal
        .record("accepted", signer, Some(&transaction), &[], json!({}))
        .await
    {
        error!(error = %error, "failed to record signer acceptance");
    }

    match state.signer.sign_direct_raw(&transaction).await {
        Ok(signed) => {
            if let Err(error) = state
                .journal
                .record(
                    "signed",
                    signer,
                    Some(&transaction),
                    &[],
                    json!({ "tx_hash": signed.tx_hash }),
                )
                .await
            {
                error!(error = %error, "failed to record signer signature");
            }
            SignerWireResponse::signed(signer, signed)
        }
        Err(error) => {
            let reason = error.to_string();
            if let Err(journal_error) = state
                .journal
                .record(
                    "sign_error",
                    signer,
                    Some(&transaction),
                    std::slice::from_ref(&reason),
                    json!({}),
                )
                .await
            {
                error!(error = %journal_error, "failed to record signer error");
            }
            SignerWireResponse::error(reason)
        }
    }
}

fn trim_ascii(value: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = value.len();
    while start < end && value[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && value[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &value[start..end]
}
