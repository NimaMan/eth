#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLane {
    /// Mempool tx acts on a pool or liquidity ownership object already mapped
    /// in the live token cache.
    CurrentTrackedPool,
    /// Mempool tx acts on a tracked token contract or known creator-owned token.
    CurrentTrackedToken,
    /// Mempool tx is part of a token/pool launch sequence that may define new
    /// addresses before the token server has confirmed them.
    PendingPoolLifecycle,
    /// Mempool tx is protocol-shaped and actionable only after the token/pool
    /// cache exposes the mapping.
    UnresolvedCacheMapping,
    /// Sender-nonce sequencing is handled by the simulation manager after route
    /// classification, not by token/pool cache lookup.
    PendingNonceSequence,
    /// Cross-sender funding dependencies are handled by simulation dependency
    /// replay once the failing tx identifies the required sender balance.
    FundingDependency,
    Irrelevant,
}

impl RouteLane {
    pub fn waits_for_cache_mapping(self) -> bool {
        matches!(self, Self::UnresolvedCacheMapping)
    }
}
