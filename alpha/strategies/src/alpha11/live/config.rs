use crate::alpha11::Alpha11Config;

#[derive(Clone, Debug)]
pub struct LiveAlpha11Config {
    pub alpha11: Alpha11Config,
}

impl LiveAlpha11Config {
    pub fn new(alpha11: Alpha11Config) -> Self {
        Self { alpha11 }
    }
}
