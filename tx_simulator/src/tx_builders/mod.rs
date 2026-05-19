pub mod protocols;
pub mod route_dispatch;
pub mod routes;

pub mod amm_swap_route {
    pub use super::routes::*;
}

pub mod curve {
    pub use super::protocols::curve::*;
}

pub mod baygus_v2_vault {
    pub use super::protocols::baygus_v2_vault::*;
}

pub mod permit2 {
    pub use super::protocols::permit2::*;
}

pub mod uniswap_v2 {
    pub use super::protocols::uniswap::v2::*;
}

pub mod uniswap_v3 {
    pub use super::protocols::uniswap::v3::*;
}

pub mod sushiswap_v3 {
    pub use super::protocols::sushiswap::v3::*;
}

pub mod pancakeswap_v3 {
    pub use super::protocols::pancakeswap::v3::*;
}

pub mod uniswap_v4 {
    pub use super::protocols::uniswap::v4::*;
}

pub use protocols::baygus_v2_vault::*;
pub use route_dispatch::*;
pub use routes::AmmSwapRoute;
