use crate::{
    broadcast::RpcBroadcaster,
    config::{BroadcastMode, EthTxExecutorConfig},
    error::Result,
    nonce::NonceManager,
    repository::{ExecutionEvent, ExecutionRecorder},
    request::DirectRawTransactionRequest,
    signer::TransactionSigner,
    types::{
        ExecutionStatus, PreparedDirectRawTransaction, SignDirectRawResult, SubmitDirectRawResult,
    },
    validation::prepare_direct_raw_request,
};
use serde_json::json;
use std::{sync::Arc, time::Instant};
use tracing::{info, warn};

pub struct EthTxExecutionService {
    config: EthTxExecutorConfig,
    signer: Arc<dyn TransactionSigner>,
    nonce_manager: NonceManager,
    broadcaster: RpcBroadcaster,
    recorder: Arc<dyn ExecutionRecorder>,
}

impl EthTxExecutionService {
    pub fn new(
        config: EthTxExecutorConfig,
        signer: Arc<dyn TransactionSigner>,
        nonce_manager: NonceManager,
        broadcaster: RpcBroadcaster,
        recorder: Arc<dyn ExecutionRecorder>,
    ) -> Self {
        Self {
            config,
            signer,
            nonce_manager,
            broadcaster,
            recorder,
        }
    }

    pub async fn submit_direct_raw(
        &self,
        request: DirectRawTransactionRequest,
    ) -> Result<SubmitDirectRawResult> {
        let started = Instant::now();
        let mut prepared =
            prepare_direct_raw_request(&self.config, request, self.signer.address())?;
        self.record(
            &prepared.attempt_id,
            ExecutionStatus::Received,
            "direct raw request received",
            json!({
                "from": prepared.from,
                "to": prepared.to,
                "chain_id": prepared.chain_id,
                "gas_limit": prepared.gas_limit,
                "max_fee_per_gas": prepared.max_fee_per_gas,
                "max_priority_fee_per_gas": prepared.max_priority_fee_per_gas,
                "simulation": prepared.simulation,
                "metadata": prepared.metadata,
            }),
        )
        .await;

        let nonce = self.nonce_manager.reserve(prepared.nonce).await?;
        prepared.nonce = Some(nonce);

        let signed = match self.signer.sign_direct_raw(&prepared).await {
            Ok(signed) => signed,
            Err(error) => {
                self.nonce_manager.invalidate().await;
                return Err(error);
            }
        };
        self.record(
            &prepared.attempt_id,
            ExecutionStatus::Signed,
            "transaction signed",
            json!({
                "tx_hash": signed.tx_hash,
                "nonce": nonce,
            }),
        )
        .await;

        match self.config.broadcast_mode {
            BroadcastMode::DryRun => {
                info!(
                    attempt_id = prepared.attempt_id,
                    tx_hash = ?signed.tx_hash,
                    "dry-run direct raw transaction signed"
                );
                self.record(
                    &prepared.attempt_id,
                    ExecutionStatus::DryRun,
                    "dry run complete; transaction was not broadcast",
                    json!({ "tx_hash": signed.tx_hash }),
                )
                .await;
                Ok(self.result(
                    prepared,
                    ExecutionStatus::DryRun,
                    Some(signed.tx_hash),
                    None,
                    started,
                ))
            }
            BroadcastMode::PublicMempool => match self
                .broadcaster
                .send_raw_transaction(&signed.raw_tx_hex)
                .await
            {
                Ok(rpc_hash) => {
                    info!(
                        attempt_id = prepared.attempt_id,
                        tx_hash = ?rpc_hash,
                        "broadcast direct raw transaction"
                    );
                    self.record(
                        &prepared.attempt_id,
                        ExecutionStatus::Broadcast,
                        "transaction broadcast",
                        json!({
                            "local_tx_hash": signed.tx_hash,
                            "rpc_tx_hash": rpc_hash,
                        }),
                    )
                    .await;
                    Ok(self.result(
                        prepared,
                        ExecutionStatus::Broadcast,
                        Some(rpc_hash),
                        None,
                        started,
                    ))
                }
                Err(err) => {
                    self.nonce_manager.invalidate().await;
                    let message = err.to_string();
                    self.record(
                        &prepared.attempt_id,
                        ExecutionStatus::BroadcastError,
                        &message,
                        json!({ "tx_hash": signed.tx_hash }),
                    )
                    .await;
                    Ok(self.result(
                        prepared,
                        ExecutionStatus::BroadcastError,
                        Some(signed.tx_hash),
                        Some(message),
                        started,
                    ))
                }
            },
        }
    }

    pub async fn sign_direct_raw(
        &self,
        request: DirectRawTransactionRequest,
    ) -> Result<SignDirectRawResult> {
        let started = Instant::now();
        let mut prepared =
            prepare_direct_raw_request(&self.config, request, self.signer.address())?;
        self.record(
            &prepared.attempt_id,
            ExecutionStatus::Received,
            "direct raw sign request received",
            json!({
                "from": prepared.from,
                "to": prepared.to,
                "chain_id": prepared.chain_id,
                "gas_limit": prepared.gas_limit,
                "max_fee_per_gas": prepared.max_fee_per_gas,
                "max_priority_fee_per_gas": prepared.max_priority_fee_per_gas,
                "simulation": prepared.simulation,
                "metadata": prepared.metadata,
                "sign_only": true,
            }),
        )
        .await;

        let nonce = self.nonce_manager.reserve(prepared.nonce).await?;
        prepared.nonce = Some(nonce);

        let signed = match self.signer.sign_direct_raw(&prepared).await {
            Ok(signed) => signed,
            Err(error) => {
                self.nonce_manager.invalidate().await;
                return Err(error);
            }
        };
        self.record(
            &prepared.attempt_id,
            ExecutionStatus::Signed,
            "transaction signed for external bundle submission",
            json!({
                "tx_hash": signed.tx_hash,
                "nonce": nonce,
                "sign_only": true,
            }),
        )
        .await;

        Ok(SignDirectRawResult {
            attempt_id: prepared.attempt_id,
            status: ExecutionStatus::Signed,
            tx_hash: signed.tx_hash,
            raw_tx_hex: signed.raw_tx_hex,
            from: prepared.from,
            to: prepared.to,
            nonce,
            gas_limit: prepared.gas_limit,
            max_fee_per_gas: prepared.max_fee_per_gas,
            max_priority_fee_per_gas: prepared.max_priority_fee_per_gas,
            elapsed_ms: started.elapsed().as_millis(),
        })
    }

    pub async fn invalidate_nonce_cache(&self) {
        self.nonce_manager.invalidate().await;
    }

    fn result(
        &self,
        prepared: PreparedDirectRawTransaction,
        status: ExecutionStatus,
        tx_hash: Option<ethers_core::types::H256>,
        error: Option<String>,
        started: Instant,
    ) -> SubmitDirectRawResult {
        SubmitDirectRawResult {
            attempt_id: prepared.attempt_id,
            status,
            tx_hash,
            from: prepared.from,
            to: prepared.to,
            nonce: prepared.nonce.expect("nonce assigned before result"),
            gas_limit: prepared.gas_limit,
            max_fee_per_gas: prepared.max_fee_per_gas,
            max_priority_fee_per_gas: prepared.max_priority_fee_per_gas,
            error,
            elapsed_ms: started.elapsed().as_millis(),
        }
    }

    async fn record(
        &self,
        attempt_id: &str,
        status: ExecutionStatus,
        message: impl Into<String>,
        payload: serde_json::Value,
    ) {
        if let Err(err) = self
            .recorder
            .record(ExecutionEvent::new(
                attempt_id,
                status,
                Some(message.into()),
                payload,
            ))
            .await
        {
            warn!(attempt_id, error = %err, "failed to record execution event");
        }
    }
}
