//! Backwards-compatible re-exports for legacy `dex::amm` paths.

pub mod uniswap_v2 {
    pub use crate::dex::uniswap::v2::*;
}

pub mod uniswap_v3 {
    pub use crate::dex::uniswap::v3::*;
}

pub mod uniswap_v4 {
    pub use crate::dex::uniswap::v4::*;
}

pub mod curve {
    pub use crate::dex::curve::*;
}

pub mod balancer {
    pub use crate::dex::balancer::*;
}

pub use crate::dex::liquidity::*;
