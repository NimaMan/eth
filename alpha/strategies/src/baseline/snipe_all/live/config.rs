use crate::baseline::snipe_all::SnipeAllConfig;

#[derive(Clone, Debug)]
pub struct LiveSnipeAllConfig {
    pub strategy: SnipeAllConfig,
}

impl LiveSnipeAllConfig {
    pub fn new(strategy: SnipeAllConfig) -> Self {
        Self { strategy }
    }
}

impl Default for LiveSnipeAllConfig {
    fn default() -> Self {
        Self::new(SnipeAllConfig::default())
    }
}
