/// Block query module
/// 
/// Provides methods for querying block-related data:
/// - Block information
/// - Block timestamps
/// - Block hashes

mod info;

pub use info::{BlockInfo, BlockQuery};