/// Token query module
/// 
/// Provides methods for querying token-related data:
/// - ERC20 tokens (balance, supply, decimals, name, symbol)
/// - ERC721 NFTs (future)
/// - ERC1155 multi-tokens (future)

mod erc20;

// Re-export ERC20 types
pub use erc20::{TokenQuery, TokenInfo, ERC20Info};