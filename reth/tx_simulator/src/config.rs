/// Global configuration constants for the TxSimulator crate.
///
/// Centralizing these values keeps behaviour consistent across modules and avoids
/// scattering magic numbers throughout the codebase.
pub mod view_call {
    /// Maximum number of attempts to fetch historical state when serving a live view call.
    pub const STATE_RETRY_MAX_ATTEMPTS: usize = 9;

    /// Delay between retry attempts in milliseconds while waiting for the state provider.
    pub const STATE_RETRY_DELAY_MS: u64 = 25;
}
