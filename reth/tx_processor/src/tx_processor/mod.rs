/// Transaction processing modules for decoding events and building ProcessedTransaction
/// These modules handle the core transaction processing pipeline

pub mod data_models;
pub mod tx_log_processor;
pub mod tx_trace_processor;
pub mod tx_classifier;
pub mod tx_processor;
pub mod tx_loader;
pub mod address_balance_change_calculator;
pub mod tax_calculator;

pub use tx_log_processor::{LogDecoder, DecodedEvent, EventSignatures};
pub use tx_trace_processor::TransactionTraceProcessor;
pub use tx_classifier::{TransactionClassifier, TransactionType};
pub use tx_processor::TxProcessor;
pub use address_balance_change_calculator::AddressBalanceChangeCalculator;
pub use tax_calculator::{TaxCalculationResult, calculate_buy_tax_from_processed_transaction, calculate_sell_tax_from_processed_transaction};
