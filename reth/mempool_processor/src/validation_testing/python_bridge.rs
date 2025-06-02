use serde::{Serialize, Deserialize};
use std::process::Command;
use eyre::{Result, eyre};
use tracing::{info, warn};
use std::path::PathBuf;

/// Python state change result for a single address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonStateChange {
    pub address: String,
    pub token_net: String,
    pub denom_net: f64,
}

/// Complete Python result for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonResult {
    pub transaction_hash: String,
    pub block_number: u64,
    pub state_changes: Vec<PythonStateChange>,
    pub processing_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Configuration for Python bridge
#[derive(Debug, Clone)]
pub struct PythonBridgeConfig {
    pub python_script_path: PathBuf,
    pub conda_env: String,
    pub timeout_seconds: u64,
    pub reth_url: String,
}

impl Default for PythonBridgeConfig {
    fn default() -> Self {
        Self {
            python_script_path: PathBuf::from("python/batch_validate_state_changes.py"),
            conda_env: "qw".to_string(),
            timeout_seconds: 300, // 5 minutes
            reth_url: "http://localhost:8545".to_string(),
        }
    }
}

/// Bridge to execute Python state change calculations
pub struct PythonBridge {
    config: PythonBridgeConfig,
}

impl PythonBridge {
    pub fn new(config: PythonBridgeConfig) -> Self {
        Self { config }
    }

    /// Process a single transaction through Python
    pub async fn process_transaction(&self, tx_hash: &str, block_number: u64) -> Result<PythonResult> {
        let start_time = std::time::Instant::now();
        
        info!("Processing transaction {} through Python", tx_hash);
        
        // Create temporary input file
        let input_data = serde_json::json!({
            "transaction_hash": tx_hash,
            "block_number": block_number,
            "reth_url": self.config.reth_url
        });
        
        let temp_input = format!("/tmp/python_input_{}.json", tx_hash.replace("0x", ""));
        std::fs::write(&temp_input, serde_json::to_string_pretty(&input_data)?)?;
        
        // Execute Python script
        let output = self.execute_python_script(&temp_input).await?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_input);
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        // Parse Python output
        let mut result: PythonResult = serde_json::from_str(&output)
            .map_err(|e| eyre!("Failed to parse Python output: {}", e))?;
        
        result.processing_time_ms = processing_time;
        
        if result.success {
            info!("Python processed {} successfully in {}ms with {} state changes", 
                  tx_hash, processing_time, result.state_changes.len());
        } else {
            warn!("Python failed to process {}: {}", 
                  tx_hash, result.error_message.as_deref().unwrap_or("Unknown error"));
        }
        
        Ok(result)
    }

    /// Process multiple transactions in batch
    pub async fn process_batch(&self, transactions: &[(String, u64)]) -> Result<Vec<PythonResult>> {
        info!("Processing batch of {} transactions through Python", transactions.len());
        
        let batch_input = serde_json::json!({
            "transactions": transactions.iter().map(|(hash, block)| {
                serde_json::json!({
                    "transaction_hash": hash,
                    "block_number": block
                })
            }).collect::<Vec<_>>(),
            "reth_url": self.config.reth_url
        });
        
        let temp_input = format!("/tmp/python_batch_input_{}.json", 
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs());
        
        std::fs::write(&temp_input, serde_json::to_string_pretty(&batch_input)?)?;
        
        // Execute Python batch script
        let output = self.execute_python_batch_script(&temp_input).await?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_input);
        
        // Parse batch results
        let results: Vec<PythonResult> = serde_json::from_str(&output)
            .map_err(|e| eyre!("Failed to parse Python batch output: {}", e))?;
        
        let successful = results.iter().filter(|r| r.success).count();
        info!("Python batch processing completed: {}/{} successful", 
              successful, results.len());
        
        Ok(results)
    }

    /// Execute Python script for single transaction
    async fn execute_python_script(&self, input_file: &str) -> Result<String> {
        let output = Command::new("conda")
            .args(&[
                "run", "-n", &self.config.conda_env,
                "python", 
                self.config.python_script_path.to_str().unwrap(),
                input_file
            ])
            .output()
            .map_err(|e| eyre!("Failed to execute Python script: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre!("Python script failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Execute Python batch script
    async fn execute_python_batch_script(&self, input_file: &str) -> Result<String> {
        // Use the same script but with batch flag
        let batch_script_path = self.config.python_script_path
            .parent()
            .unwrap()
            .join("batch_validate_state_changes.py");
            
        let output = Command::new("conda")
            .args(&[
                "run", "-n", &self.config.conda_env,
                "python", 
                batch_script_path.to_str().unwrap(),
                "--batch",
                input_file
            ])
            .output()
            .map_err(|e| eyre!("Failed to execute Python batch script: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre!("Python batch script failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Test Python environment setup
    pub async fn test_environment(&self) -> Result<()> {
        info!("Testing Python environment setup...");
        
        // Test conda environment
        let output = Command::new("conda")
            .args(&["run", "-n", &self.config.conda_env, "python", "--version"])
            .output()
            .map_err(|e| eyre!("Failed to test conda environment: {}", e))?;

        if !output.status.success() {
            return Err(eyre!("Conda environment '{}' not found or not working", self.config.conda_env));
        }

        let python_version = String::from_utf8_lossy(&output.stdout);
        info!("Python environment OK: {}", python_version.trim());

        // Test script existence
        if !self.config.python_script_path.exists() {
            return Err(eyre!("Python script not found: {:?}", self.config.python_script_path));
        }

        info!("Python script found: {:?}", self.config.python_script_path);

        // Test basic script execution
        let test_input = serde_json::json!({
            "test": true,
            "reth_url": self.config.reth_url
        });
        
        let temp_test = "/tmp/python_env_test.json";
        std::fs::write(temp_test, serde_json::to_string(&test_input)?)?;
        
        let test_output = Command::new("conda")
            .args(&[
                "run", "-n", &self.config.conda_env,
                "python", 
                self.config.python_script_path.to_str().unwrap(),
                "--test",
                temp_test
            ])
            .output();
            
        let _ = std::fs::remove_file(temp_test);
        
        match test_output {
            Ok(output) if output.status.success() => {
                info!("Python environment test successful");
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(eyre!("Python environment test failed: {}", stderr))
            }
            Err(e) => Err(eyre!("Failed to run Python environment test: {}", e))
        }
    }
} 