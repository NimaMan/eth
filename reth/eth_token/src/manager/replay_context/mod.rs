//! Block-local replay context for token and pool simulations.

mod block_context;
mod pool_prior_index;
#[cfg(test)]
mod tests;
mod token_prior_index;
pub(crate) mod triggers;

pub(crate) use block_context::BlockReplayContext;
