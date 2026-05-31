#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub database_url: String,
}

impl ExecutionConfig {
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }

    pub fn database_url(&self) -> &str {
        &self.database_url
    }
}
