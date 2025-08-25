/// Account query module
/// 
/// Provides methods for querying account-related data:
/// - ETH balance
/// - Transaction nonce
/// - Contract code existence

mod balance;

pub use balance::{AccountInfo, AccountQuery};