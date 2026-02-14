//! Alert Validation Module
//!
//! Provides HMAC signature validation for incoming alerts

use super::Alert;
use crate::common::errors::KartalError;
use base64::{engine::general_purpose, Engine as _};
use ethers::types::U256;
use ring::{hmac, rand};
use serde_json;
use tracing::{info, warn};

/// Alert validator with HMAC verification
pub struct AlertValidator {
    signing_key: hmac::Key,
}

impl AlertValidator {
    /// Create new validator with shared secret
    pub fn new(shared_secret: &str) -> Self {
        let key = hmac::Key::new(hmac::HMAC_SHA256, shared_secret.as_bytes());
        Self { signing_key: key }
    }

    /// Create validator with random key (for testing)
    pub fn new_random() -> (Self, String) {
        let rng = rand::SystemRandom::new();
        let mut key_bytes = [0u8; 32];
        rand::SecureRandom::fill(&rng, &mut key_bytes).expect("Failed to generate random key");

        let key = hmac::Key::new(hmac::HMAC_SHA256, &key_bytes);
        let secret = general_purpose::STANDARD.encode(&key_bytes);

        (Self { signing_key: key }, secret)
    }

    /// Validate alert with HMAC signature
    pub fn validate_alert(&self, alert_json: &str, signature: &str) -> Result<Alert, KartalError> {
        // Decode the signature from base64
        let signature_bytes = general_purpose::STANDARD
            .decode(signature)
            .map_err(|e| KartalError::Validation(format!("Invalid signature encoding: {}", e)))?;

        // Verify HMAC
        match hmac::verify(&self.signing_key, alert_json.as_bytes(), &signature_bytes) {
            Ok(_) => {
                info!("Alert signature validated successfully");

                // Parse the alert
                let alert: Alert = serde_json::from_str(alert_json).map_err(|e| {
                    KartalError::SerializationError(format!("Failed to parse alert: {}", e))
                })?;

                // Additional validation
                self.validate_alert_content(&alert)?;

                Ok(alert)
            }
            Err(_) => {
                warn!("Alert signature validation failed");
                Err(KartalError::Validation(
                    "Invalid HMAC signature".to_string(),
                ))
            }
        }
    }

    /// Sign an alert (for testing/debugging)
    pub fn sign_alert(&self, alert: &Alert) -> Result<String, KartalError> {
        let alert_json = serde_json::to_string(alert)
            .map_err(|e| KartalError::SerializationError(e.to_string()))?;

        let signature = hmac::sign(&self.signing_key, alert_json.as_bytes());
        Ok(general_purpose::STANDARD.encode(signature.as_ref()))
    }

    /// Validate alert content
    fn validate_alert_content(&self, alert: &Alert) -> Result<(), KartalError> {
        // Check alert ID format
        if alert.id.is_empty() {
            return Err(KartalError::Validation(
                "Alert ID cannot be empty".to_string(),
            ));
        }

        // Check timestamp is reasonable (not too old or in future)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let alert_time = alert.timestamp;

        // Alert should not be more than 5 minutes old
        if now > alert_time && (now - alert_time) > 300 {
            return Err(KartalError::Validation(
                "Alert timestamp too old".to_string(),
            ));
        }

        // Alert should not be from the future
        if alert_time > now + 60 {
            // Allow 1 minute clock skew
            return Err(KartalError::Validation(
                "Alert timestamp in future".to_string(),
            ));
        }

        // Validate addresses
        if alert.token_address.is_zero() {
            return Err(KartalError::Validation("Invalid token address".to_string()));
        }

        // Validate amounts
        if alert.params.amount == U256::zero() {
            return Err(KartalError::Validation("Amount cannot be zero".to_string()));
        }

        Ok(())
    }
}

/// Message wrapper with signature
#[derive(Debug, Clone)]
pub struct SignedMessage {
    pub payload: String,
    pub signature: String,
}

impl SignedMessage {
    pub fn new(payload: String, signature: String) -> Self {
        Self { payload, signature }
    }

    /// Parse from ZMQ message format
    pub fn from_zmq_message(msg: &str) -> Result<Self, KartalError> {
        // Expected format: "SIGNATURE:BASE64_SIG|PAYLOAD:JSON"
        let parts: Vec<&str> = msg.splitn(2, '|').collect();

        if parts.len() != 2 {
            return Err(KartalError::Validation(
                "Invalid message format".to_string(),
            ));
        }

        let sig_part = parts[0];
        let payload_part = parts[1];

        if !sig_part.starts_with("SIGNATURE:") {
            return Err(KartalError::Validation(
                "Missing signature prefix".to_string(),
            ));
        }

        if !payload_part.starts_with("PAYLOAD:") {
            return Err(KartalError::Validation(
                "Missing payload prefix".to_string(),
            ));
        }

        let signature = sig_part.trim_start_matches("SIGNATURE:");
        let payload = payload_part.trim_start_matches("PAYLOAD:");

        Ok(Self {
            signature: signature.to_string(),
            payload: payload.to_string(),
        })
    }

    /// Format for ZMQ transmission
    pub fn to_zmq_message(&self) -> String {
        format!("SIGNATURE:{}|PAYLOAD:{}", self.signature, self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::{Action, ExecutionParams};
    use crate::common::Priority;
    use ethers::types::Address;
    use std::str::FromStr;

    #[test]
    fn test_alert_validation() {
        let (validator, _secret) = AlertValidator::new_random();

        let alert = Alert {
            id: "TEST-001".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            token_address: Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            pool_address: Address::from_str("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            action: Action::Buy,
            params: ExecutionParams {
                amount: U256::from(1000000),
                slippage: 0.05,
                gas_price: U256::from(30_000_000_000u64),
                priority: Priority::Normal,
                deadline: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 300,
            },
        };

        // Sign the alert
        let signature = validator.sign_alert(&alert).unwrap();

        // Validate it
        let alert_json = serde_json::to_string(&alert).unwrap();
        let validated = validator.validate_alert(&alert_json, &signature).unwrap();

        assert_eq!(validated.id, alert.id);
    }

    #[test]
    fn test_invalid_signature() {
        let validator = AlertValidator::new("test-secret");

        let alert = Alert {
            id: "TEST-002".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            token_address: Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            pool_address: Address::from_str("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap(),
            action: Action::Buy,
            params: ExecutionParams {
                amount: U256::from(1000000),
                slippage: 0.05,
                gas_price: U256::from(30_000_000_000u64),
                priority: Priority::Normal,
                deadline: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 300,
            },
        };

        let alert_json = serde_json::to_string(&alert).unwrap();
        let invalid_sig = "invalid-signature";

        let result = validator.validate_alert(&alert_json, invalid_sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_signed_message_format() {
        let msg = SignedMessage::new(
            "{\"test\":\"payload\"}".to_string(),
            "base64signature".to_string(),
        );

        let zmq_msg = msg.to_zmq_message();
        assert_eq!(
            zmq_msg,
            "SIGNATURE:base64signature|PAYLOAD:{\"test\":\"payload\"}"
        );

        let parsed = SignedMessage::from_zmq_message(&zmq_msg).unwrap();
        assert_eq!(parsed.signature, "base64signature");
        assert_eq!(parsed.payload, "{\"test\":\"payload\"}");
    }
}
