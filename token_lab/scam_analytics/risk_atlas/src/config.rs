pub const DEFAULT_RISK_ATLAS_DATABASE_URL: &str =
    "postgresql://postgres:postgres@localhost:5432/eth_db";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskAtlasConfig {
    pub database_url: String,
}

impl Default for RiskAtlasConfig {
    fn default() -> Self {
        Self {
            database_url: DEFAULT_RISK_ATLAS_DATABASE_URL.to_string(),
        }
    }
}
