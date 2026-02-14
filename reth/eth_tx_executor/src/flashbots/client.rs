//! Flashbots relay client for bundle submission

use ethers::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument, warn};

use super::bundle::Bundle;
use super::signer::BundleSigner;
use super::types::*;

/// Known Flashbots relay endpoints
#[derive(Debug, Clone)]
pub enum RelayEndpoint {
    /// Official Flashbots relay
    Flashbots,
    /// BloXroute relay
    BloXroute { auth_token: String },
    /// Eden Network relay
    Eden,
    /// Manifold Finance relay
    Manifold,
    /// Custom relay endpoint
    Custom { url: String, name: String },
}

impl RelayEndpoint {
    /// Get relay URL
    pub fn url(&self) -> &str {
        match self {
            RelayEndpoint::Flashbots => "https://relay.flashbots.net",
            RelayEndpoint::BloXroute { .. } => "https://mev.api.blxrbdn.com",
            RelayEndpoint::Eden => "https://api.edennetwork.io/v1/bundle",
            RelayEndpoint::Manifold => "https://api.manifold.finance/v1/bundle",
            RelayEndpoint::Custom { url, .. } => url,
        }
    }

    /// Get relay name for logging
    pub fn name(&self) -> &str {
        match self {
            RelayEndpoint::Flashbots => "Flashbots",
            RelayEndpoint::BloXroute { .. } => "BloXroute",
            RelayEndpoint::Eden => "Eden",
            RelayEndpoint::Manifold => "Manifold",
            RelayEndpoint::Custom { name, .. } => name,
        }
    }
}

/// Flashbots client configuration
#[derive(Debug, Clone)]
pub struct FlashbotsConfig {
    /// Relay endpoints to use
    pub relay_endpoints: Vec<RelayEndpoint>,
    /// Bundle signer for reputation
    pub signer: Arc<BundleSigner>,
    /// HTTP client timeout
    pub timeout: Duration,
    /// Enable bundle simulation before submission
    pub simulate_before_submit: bool,
    /// Maximum retries per relay
    pub max_retries: u32,
    /// Delay between retries
    pub retry_delay: Duration,
}

impl Default for FlashbotsConfig {
    fn default() -> Self {
        Self {
            relay_endpoints: vec![RelayEndpoint::Flashbots],
            signer: Arc::new(BundleSigner::random()), // Should use proper key in production
            timeout: Duration::from_secs(5),
            simulate_before_submit: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// Flashbots relay client
pub struct FlashbotsClient {
    config: FlashbotsConfig,
    http_client: Client,
    provider: Arc<Provider<Http>>,
}

impl FlashbotsClient {
    /// Create new Flashbots client
    pub fn new(
        config: FlashbotsConfig,
        provider: Arc<Provider<Http>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = Client::builder().timeout(config.timeout).build()?;

        Ok(Self {
            config,
            http_client,
            provider,
        })
    }

    /// Submit bundle to relays
    #[instrument(skip(self, bundle))]
    pub async fn submit_bundle(
        &self,
        bundle: Bundle,
    ) -> Result<BundleResult, Box<dyn std::error::Error>> {
        info!(
            "Submitting bundle with {} transactions for block {}",
            bundle.transactions.len(),
            bundle.block_number
        );

        // Simulate bundle first if enabled
        if self.config.simulate_before_submit {
            match self.simulate_bundle(&bundle).await {
                Ok(sim_result) if !sim_result.success => {
                    error!("Bundle simulation failed: {:?}", sim_result.error);
                    return Ok(BundleResult::Failed {
                        error: sim_result
                            .error
                            .unwrap_or_else(|| "Simulation failed".to_string()),
                    });
                }
                Ok(sim_result) => {
                    info!(
                        "Bundle simulation successful. Gas: {}, Tip: {}",
                        sim_result.gas_used,
                        ethers::utils::format_ether(sim_result.coinbase_diff)
                    );
                }
                Err(e) => {
                    warn!("Bundle simulation error: {}. Proceeding anyway.", e);
                }
            }
        }

        // Submit to all configured relays
        let mut submission_results = Vec::new();

        for relay in &self.config.relay_endpoints {
            match self.submit_to_relay(&bundle, relay).await {
                Ok(bundle_hash) => {
                    info!("Bundle {} submitted to {}", bundle_hash, relay.name());
                    submission_results.push((relay.name(), Ok(bundle_hash)));
                }
                Err(e) => {
                    error!("Failed to submit to {}: {}", relay.name(), e);
                    submission_results.push((relay.name(), Err(e)));
                }
            }
        }

        // Check if any submission succeeded
        let successful_submissions: Vec<_> = submission_results
            .iter()
            .filter_map(|(name, result)| result.as_ref().ok().map(|hash| (*name, *hash)))
            .collect();

        if successful_submissions.is_empty() {
            return Ok(BundleResult::Failed {
                error: "Failed to submit to any relay".to_string(),
            });
        }

        info!(
            "Bundle submitted to {} relays successfully",
            successful_submissions.len()
        );

        // Wait for inclusion
        self.wait_for_inclusion(bundle.block_number, successful_submissions)
            .await
    }

    /// Submit bundle to specific relay
    async fn submit_to_relay(
        &self,
        bundle: &Bundle,
        relay: &RelayEndpoint,
    ) -> Result<H256, Box<dyn std::error::Error>> {
        let bundle_request = bundle.to_request();

        // Sign bundle for Flashbots authentication
        let signature = self.config.signer.sign_bundle(&bundle_request)?;

        let request_body = FlashbotsRequest::new("eth_sendBundle", vec![bundle_request]);

        let mut request = self.http_client.post(relay.url()).json(&request_body);

        // Add authentication headers
        match relay {
            RelayEndpoint::Flashbots => {
                request = request.header(
                    "X-Flashbots-Signature",
                    format!("{}:{}", self.config.signer.address(), signature),
                );
            }
            RelayEndpoint::BloXroute { auth_token } => {
                request = request.header("Authorization", format!("Bearer {}", auth_token));
            }
            _ => {}
        }

        let response = request.send().await?;
        let response_text = response.text().await?;

        // Parse response
        let flashbots_response: FlashbotsResponse<BundleStats> =
            serde_json::from_str(&response_text)?;

        match flashbots_response.data {
            FlashbotsResponseData::Success { result } => Ok(result.bundle_hash),
            FlashbotsResponseData::Error { error } => {
                Err(format!("Relay error {}: {}", error.code, error.message).into())
            }
        }
    }

    /// Simulate bundle execution
    async fn simulate_bundle(
        &self,
        bundle: &Bundle,
    ) -> Result<SimulationResult, Box<dyn std::error::Error>> {
        let bundle_request = bundle.to_request();
        let signature = self.config.signer.sign_bundle(&bundle_request)?;

        let request_body = FlashbotsRequest::new(
            "eth_callBundle",
            json!({
                "txs": bundle_request.txs,
                "blockNumber": format!("0x{:x}", bundle.block_number),
                "stateBlockNumber": "latest",
            }),
        );

        let response = self
            .http_client
            .post("https://relay.flashbots.net")
            .header(
                "X-Flashbots-Signature",
                format!("{}:{}", self.config.signer.address(), signature),
            )
            .json(&request_body)
            .send()
            .await?;

        let response_text = response.text().await?;
        let flashbots_response: FlashbotsResponse<SimulationResult> =
            serde_json::from_str(&response_text)?;

        match flashbots_response.data {
            FlashbotsResponseData::Success { result } => Ok(result),
            FlashbotsResponseData::Error { error } => {
                Err(format!("Simulation error {}: {}", error.code, error.message).into())
            }
        }
    }

    /// Wait for bundle inclusion in target block
    async fn wait_for_inclusion(
        &self,
        target_block: u64,
        _submissions: Vec<(&str, H256)>,
    ) -> Result<BundleResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(30); // Wait up to 30 seconds

        loop {
            // Check if timeout reached
            if start_time.elapsed() > timeout {
                return Ok(BundleResult::NotIncluded {
                    reason: BundleNotIncludedReason::BlockNotMined,
                });
            }

            // Get current block
            let current_block = self.provider.get_block_number().await?;

            if current_block.as_u64() >= target_block {
                // Block has been mined, check if bundle was included
                let block = self
                    .provider
                    .get_block_with_txs(target_block)
                    .await?
                    .ok_or("Block not found")?;

                // Check if any of our transactions are in the block
                // In production, would check all bundle transactions
                let _tx_hashes: Vec<H256> = block.transactions.iter().map(|tx| tx.hash()).collect();

                // For now, assume not included
                return Ok(BundleResult::NotIncluded {
                    reason: BundleNotIncludedReason::Outbid,
                });
            }

            // Wait before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get bundle status from relay
    pub async fn get_bundle_status(
        &self,
        bundle_hash: H256,
        relay: &RelayEndpoint,
    ) -> Result<BundleStatus, Box<dyn std::error::Error>> {
        let request_body = FlashbotsRequest::new(
            "flashbots_getBundleStats",
            json!({
                "bundleHash": bundle_hash,
            }),
        );

        let signature = self.config.signer.sign_message(bundle_hash.as_bytes())?;

        let response = self
            .http_client
            .post(relay.url())
            .header(
                "X-Flashbots-Signature",
                format!("{}:{}", self.config.signer.address(), signature),
            )
            .json(&request_body)
            .send()
            .await?;

        let response_text = response.text().await?;
        let flashbots_response: FlashbotsResponse<BundleStatus> =
            serde_json::from_str(&response_text)?;

        match flashbots_response.data {
            FlashbotsResponseData::Success { result } => Ok(result),
            FlashbotsResponseData::Error { error } => {
                Err(format!("Status query error {}: {}", error.code, error.message).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_endpoints() {
        let flashbots = RelayEndpoint::Flashbots;
        assert_eq!(flashbots.url(), "https://relay.flashbots.net");
        assert_eq!(flashbots.name(), "Flashbots");

        let custom = RelayEndpoint::Custom {
            url: "https://custom.relay".to_string(),
            name: "CustomRelay".to_string(),
        };
        assert_eq!(custom.url(), "https://custom.relay");
        assert_eq!(custom.name(), "CustomRelay");
    }
}
