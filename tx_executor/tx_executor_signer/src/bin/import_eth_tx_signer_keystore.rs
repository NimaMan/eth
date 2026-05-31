use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use ethers_core::types::Address;
use ethers_signers::{LocalWallet, Signer};
use rand::thread_rng;

const DEFAULT_EXPECTED_ADDRESS: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_SECRET_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/tx_executor/.state/signer/secrets";
const DEFAULT_KEYSTORE_NAME: &str = "eth-signer-keystore.json";
const DEFAULT_PASSWORD_NAME: &str = "eth-signer-password";
const DEFAULT_ENV_NAME: &str = "eth-signer.env";
const DEFAULT_ETH_CONFIG_ENV: &str = "/home/nima/code/crypto/blockchains/eth/config.env";

fn main() -> Result<()> {
    let config = ImportConfig::from_env()?;
    config.prepare()?;

    println!("Importing ETH signer key into encrypted keystore.");
    println!("Expected address: {:?}", config.expected_address);
    println!("Keystore path: {}", config.keystore_path.display());
    println!("Password file: {}", config.password_path.display());
    println!("Env fragment: {}", config.env_path.display());
    println!("Policy env: {}", config.policy_env_path.display());
    println!("The private key and password prompts below are hidden.");

    let private_key = rpassword::prompt_password("Private key: ")?;
    let private_key_bytes = decode_private_key(&private_key)?;
    let wallet = LocalWallet::from_str(private_key.trim())
        .context("failed to parse private key")?
        .with_chain_id(1_u64);
    let address = wallet.address();
    if address != config.expected_address {
        bail!(
            "private key address {:?} does not match expected {:?}",
            address,
            config.expected_address
        );
    }

    let password = rpassword::prompt_password("Keystore password: ")?;
    if password.is_empty() {
        bail!("keystore password must not be empty");
    }
    let confirm = rpassword::prompt_password("Confirm keystore password: ")?;
    if password != confirm {
        bail!("keystore passwords do not match");
    }

    let mut rng = thread_rng();
    LocalWallet::encrypt_keystore(
        &config.secret_dir,
        &mut rng,
        &private_key_bytes,
        password.as_bytes(),
        Some(DEFAULT_KEYSTORE_NAME),
    )
    .context("failed to write encrypted keystore")?;
    set_file_mode(&config.keystore_path, 0o600)?;

    write_secret_file(&config.password_path, &password)?;
    write_env_file(&config, address)?;

    let decrypted = LocalWallet::decrypt_keystore(&config.keystore_path, password.as_bytes())
        .context("failed to verify encrypted keystore")?;
    if decrypted.address() != address {
        bail!("keystore verification produced the wrong address");
    }

    println!("Imported encrypted keystore for {:?}", address);
    println!("Next: copy values from {} into /etc/eth-tx-executor/signer.env or point the systemd unit at this file.", config.env_path.display());
    Ok(())
}

#[derive(Debug)]
struct ImportConfig {
    expected_address: Address,
    secret_dir: PathBuf,
    keystore_path: PathBuf,
    password_path: PathBuf,
    env_path: PathBuf,
    policy_env_path: PathBuf,
    overwrite: bool,
}

impl ImportConfig {
    fn from_env() -> Result<Self> {
        let expected = std::env::var("ETH_TX_SIGNER_IMPORT_EXPECTED_ADDRESS")
            .or_else(|_| std::env::var("ETH_TX_SIGNER_ADDRESS"))
            .unwrap_or_else(|_| DEFAULT_EXPECTED_ADDRESS.to_string());
        let expected_address = expected
            .parse::<Address>()
            .with_context(|| format!("invalid expected address {expected:?}"))?;

        let secret_dir = std::env::var("ETH_TX_SIGNER_IMPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_SECRET_DIR));
        let keystore_path = secret_dir.join(DEFAULT_KEYSTORE_NAME);
        let password_path = std::env::var("ETH_TX_SIGNER_IMPORT_PASSWORD_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| secret_dir.join(DEFAULT_PASSWORD_NAME));
        let env_path = std::env::var("ETH_TX_SIGNER_IMPORT_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| secret_dir.join(DEFAULT_ENV_NAME));
        let policy_env_path = std::env::var("ETH_TX_SIGNER_POLICY_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_ETH_CONFIG_ENV));
        let overwrite = std::env::var("ETH_TX_SIGNER_IMPORT_OVERWRITE")
            .map(|value| matches!(value.trim(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);

        Ok(Self {
            expected_address,
            secret_dir,
            keystore_path,
            password_path,
            env_path,
            policy_env_path,
            overwrite,
        })
    }

    fn prepare(&self) -> Result<()> {
        fs::create_dir_all(&self.secret_dir)
            .with_context(|| format!("failed to create {}", self.secret_dir.display()))?;
        fs::set_permissions(&self.secret_dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("failed to chmod {}", self.secret_dir.display()))?;

        for path in [&self.keystore_path, &self.password_path, &self.env_path] {
            if path.exists() && !self.overwrite {
                bail!(
                    "{} already exists; set ETH_TX_SIGNER_IMPORT_OVERWRITE=1 to replace it",
                    path.display()
                );
            }
            if path.exists() {
                fs::remove_file(path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
        }

        Ok(())
    }
}

fn decode_private_key(value: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let decoded = hex::decode(hex).context("private key is not valid hex")?;
    if decoded.len() != 32 {
        bail!("private key must be 32 bytes, got {} bytes", decoded.len());
    }
    Ok(decoded)
}

fn write_secret_file(path: &Path, value: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(value.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to write {}", path.display()))?;
    set_file_mode(path, 0o600)
}

fn write_env_file(config: &ImportConfig, address: Address) -> Result<()> {
    let policy = read_env_file(&config.policy_env_path)?;
    let max_value_wei = required_env_value(&policy, "ETH_TX_SIGNER_MAX_VALUE_WEI")?;
    let max_gas_limit = required_env_value(&policy, "ETH_TX_SIGNER_MAX_GAS_LIMIT")?;
    let max_fee_wei = required_env_value(&policy, "ETH_TX_SIGNER_MAX_FEE_WEI")?;
    let max_priority_fee_wei = required_env_value(&policy, "ETH_TX_SIGNER_MAX_PRIORITY_FEE_WEI")?;
    let max_transaction_cost_wei =
        required_env_value(&policy, "ETH_TX_SIGNER_MAX_TRANSACTION_COST_WEI")?;
    let contents = format!(
        "\
ETH_TX_SIGNER_ADDRESS={address:?}
ETH_TX_SIGNER_KEY_BACKEND=keystore
ETH_TX_SIGNER_KEYSTORE_PATH={}
ETH_TX_SIGNER_PASSWORD_FILE={}
ETH_TX_SIGNER_ALLOWED_TARGETS=
ETH_TX_SIGNER_ALLOWED_SELECTORS=0x8a62666c,0x5f413d10
ETH_TX_SIGNER_MAX_VALUE_WEI={max_value_wei}
ETH_TX_SIGNER_MAX_GAS_LIMIT={max_gas_limit}
ETH_TX_SIGNER_MAX_FEE_WEI={max_fee_wei}
ETH_TX_SIGNER_MAX_PRIORITY_FEE_WEI={max_priority_fee_wei}
ETH_TX_SIGNER_MAX_TRANSACTION_COST_WEI={max_transaction_cost_wei}
",
        config.keystore_path.display(),
        config.password_path.display()
    );

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config.env_path)
        .with_context(|| format!("failed to create {}", config.env_path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("failed to write {}", config.env_path.display()))?;
    set_file_mode(&config.env_path, 0o600)
}

fn read_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy env file {}", path.display()))?;
    let mut values = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(
            key.to_string(),
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string(),
        );
    }
    Ok(values)
}

fn required_env_value(values: &HashMap<String, String>, key: &str) -> Result<String> {
    values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| format!("{key} must be set in {DEFAULT_ETH_CONFIG_ENV}"))
}

fn set_file_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("failed to chmod {:o} {}", mode, path.display()))
}
