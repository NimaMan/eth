use crate::{
    broadcast::RpcBroadcaster,
    config::EthTxExecutorConfig,
    error::{EthTxExecutorError, Result},
    nonce::NonceManager,
    repository::{ExecutionRecorder, JsonlRecorder, NoopRecorder},
    service::EthTxExecutionService,
    signer::LocalTransactionSigner,
};
use ethers_providers::{Http, Provider};
use std::sync::Arc;

pub struct EthTxExecutor {
    service: EthTxExecutionService,
}

impl EthTxExecutor {
    pub fn from_private_key(config: EthTxExecutorConfig, private_key: &str) -> Result<Self> {
        let signer = LocalTransactionSigner::from_private_key(private_key, config.chain_id)?;
        Self::from_signer(config, signer)
    }

    pub fn from_env(config: EthTxExecutorConfig, private_key_env: &str) -> Result<Self> {
        let signer = LocalTransactionSigner::from_env(private_key_env, config.chain_id)?;
        Self::from_signer(config, signer)
    }

    pub fn from_signer(
        config: EthTxExecutorConfig,
        signer: LocalTransactionSigner,
    ) -> Result<Self> {
        let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
            .map_err(|err| EthTxExecutorError::Config(format!("invalid rpc_url: {err}")))?;
        let recorder: Arc<dyn ExecutionRecorder> = match config.local_journal_path.as_ref() {
            Some(path) => Arc::new(JsonlRecorder::new(path)),
            None => Arc::new(NoopRecorder),
        };
        let nonce_manager = NonceManager::new(provider.clone(), signer.address());
        let broadcaster = RpcBroadcaster::new(provider);

        Ok(Self {
            service: EthTxExecutionService::new(
                config,
                signer,
                nonce_manager,
                broadcaster,
                recorder,
            ),
        })
    }

    pub async fn submit_direct_raw(
        &self,
        request: crate::request::DirectRawTransactionRequest,
    ) -> Result<crate::types::SubmitDirectRawResult> {
        self.service.submit_direct_raw(request).await
    }
}
