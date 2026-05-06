use eyre::{eyre, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

const MINIMAL_ROUTER_BYTECODE_RELATIVE_PATH: &str = "contracts/uniswap_v4/MinimalV4Router.bin";
const BAYGUS_EXECUTOR_ARTIFACT_RELATIVE_PATH: &str = "out/BaygusExecutor.sol/BaygusExecutor.json";
const MOCK_POOL_MANAGER_ARTIFACT_RELATIVE_PATH: &str =
    "out/MockPoolManager.sol/MockPoolManager.json";
const MOCK_ERC20_ARTIFACT_RELATIVE_PATH: &str = "out/MockERC20.sol/MockERC20.json";

#[derive(Debug, Deserialize)]
struct FoundryBytecode {
    object: String,
}

#[derive(Debug, Deserialize)]
struct FoundryArtifact {
    bytecode: FoundryBytecode,
}

impl FoundryArtifact {
    fn object_hex(&self) -> &str {
        self.bytecode.object.trim_start_matches("0x")
    }
}

/// Repository-local Baygus executor artifact root under `soleth`.
pub fn soleth_baygus_executor_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("soleth/baygus-executor")
}

pub fn minimal_router_bytecode_path() -> PathBuf {
    soleth_baygus_executor_dir().join(MINIMAL_ROUTER_BYTECODE_RELATIVE_PATH)
}

pub fn baygus_executor_artifact_path() -> PathBuf {
    soleth_baygus_executor_dir().join(BAYGUS_EXECUTOR_ARTIFACT_RELATIVE_PATH)
}

#[deprecated(note = "use soleth_baygus_executor_dir")]
pub fn soleth_baygus_router_dir() -> PathBuf {
    soleth_baygus_executor_dir()
}

#[deprecated(note = "use baygus_executor_artifact_path")]
pub fn baygus_router_artifact_path() -> PathBuf {
    baygus_executor_artifact_path()
}

pub fn mock_pool_manager_artifact_path() -> PathBuf {
    soleth_baygus_executor_dir().join(MOCK_POOL_MANAGER_ARTIFACT_RELATIVE_PATH)
}

pub fn mock_erc20_artifact_path() -> PathBuf {
    soleth_baygus_executor_dir().join(MOCK_ERC20_ARTIFACT_RELATIVE_PATH)
}

pub(super) fn decode_hex_bytecode(hex_value: &str, label: &str) -> Result<Vec<u8>> {
    let trimmed = hex_value.trim().trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err(eyre!("{label} bytecode is empty"));
    }

    hex::decode(trimmed).map_err(|err| eyre!("invalid {label} bytecode hex: {err}"))
}

pub(super) fn read_raw_bytecode_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let contents = fs::read_to_string(path).map_err(|err| {
        eyre!(
            "failed to read {label} bytecode from {}: {err}; build or restore soleth/baygus-executor artifacts first",
            path.display()
        )
    })?;
    decode_hex_bytecode(&contents, label)
}

pub(super) fn read_foundry_artifact_bytecode(path: &Path, label: &str) -> Result<Vec<u8>> {
    let contents = fs::read_to_string(path).map_err(|err| {
        eyre!(
            "failed to read {label} artifact from {}: {err}; build or restore soleth/baygus-executor artifacts first",
            path.display()
        )
    })?;
    let artifact: FoundryArtifact = serde_json::from_str(&contents)
        .map_err(|err| eyre!("failed to parse {label} artifact {}: {err}", path.display()))?;
    decode_hex_bytecode(artifact.object_hex(), label)
}
