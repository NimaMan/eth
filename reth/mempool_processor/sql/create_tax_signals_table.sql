-- Create tax_signals table in live_trading schema
CREATE TABLE IF NOT EXISTS live_trading.tax_signals (
    signal_id BIGSERIAL PRIMARY KEY,
    
    -- Token and Pool identification
    token_address VARCHAR(42) NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    pool_type VARCHAR(10) NOT NULL CHECK (pool_type IN ('V2', 'V3', 'V4')),
    denom_address VARCHAR(42) NOT NULL,
    denom_currency VARCHAR(20),
    
    -- Detection details
    detection_timestamp TIMESTAMP NOT NULL,
    detection_tx_hash VARCHAR(66) NOT NULL,
    
    -- Signal information
    signal_type VARCHAR(50) NOT NULL CHECK (signal_type IN ('HighTaxOrHoneypot', 'TaxChange', 'SuspiciousPattern')),
    signal_details TEXT,
    confidence NUMERIC(3,2),
    
    -- Tax values
    buy_tax_at_signal NUMERIC(5,2),
    sell_tax_at_signal NUMERIC(5,2),
    buy_tax_exceeds_threshold BOOLEAN DEFAULT FALSE,
    sell_tax_exceeds_threshold BOOLEAN DEFAULT FALSE,
    cant_sell BOOLEAN DEFAULT FALSE,
    
    -- Creator and source
    creator_address VARCHAR(42) NOT NULL,
    signal_source VARCHAR(20) DEFAULT 'mempool',
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Unique constraint
    UNIQUE(pool_address, detection_tx_hash)
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_tax_signals_token_address ON live_trading.tax_signals(token_address);
CREATE INDEX IF NOT EXISTS idx_tax_signals_pool_address ON live_trading.tax_signals(pool_address);
CREATE INDEX IF NOT EXISTS idx_tax_signals_detection_timestamp ON live_trading.tax_signals(detection_timestamp);
CREATE INDEX IF NOT EXISTS idx_tax_signals_honeypot ON live_trading.tax_signals(cant_sell) WHERE cant_sell = true;
CREATE INDEX IF NOT EXISTS idx_tax_signals_high_taxes ON live_trading.tax_signals(buy_tax_exceeds_threshold, sell_tax_exceeds_threshold) 
    WHERE buy_tax_exceeds_threshold = true OR sell_tax_exceeds_threshold = true;
CREATE INDEX IF NOT EXISTS idx_tax_signals_token_pool_time ON live_trading.tax_signals(token_address, pool_address, detection_timestamp);