-- Create scam_signals table in live_trading schema
-- This table consolidates all scam-related signals into one place

CREATE TABLE IF NOT EXISTS live_trading.scam_signals (
    signal_id SERIAL PRIMARY KEY,
    
    -- Simple, clear scam types
    scam_type VARCHAR(50) NOT NULL CHECK (scam_type IN (
        'cant_sell',           -- Cannot sell tokens back
        'high_tax',            -- Buy or sell tax exceeds threshold
        'liquidity_drain',     -- Pool drained significantly
        'lp_approval'          -- LP tokens approved for withdrawal
    )),
    
    -- Core fields
    token_address VARCHAR(42) NOT NULL,
    pool_address VARCHAR(42),
    scammer_address VARCHAR(42) NOT NULL,
    detection_timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    detection_tx_hash VARCHAR(66) NOT NULL,
    
    -- Details as JSONB for flexibility
    scam_details JSONB,
    /* Examples:
       cant_sell: {"buy_tax": 0, "sell_tax": null, "can_buy": true, "can_sell": false}
       high_tax: {"buy_tax": 5.0, "sell_tax": 99.0}
       liquidity_drain: {"eth_drained": 4.5, "drain_percentage": 95.0, "remaining_eth": 0.2}
       lp_approval: {"approved_spender": "0x...", "pool_liquidity_eth": 10.5}
    */
    
    signal_source VARCHAR(20) NOT NULL DEFAULT 'mempool',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Prevent duplicate signals for same pool/tx/type combination
    UNIQUE(pool_address, detection_tx_hash, scam_type)
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_scam_signals_token ON live_trading.scam_signals(token_address);
CREATE INDEX IF NOT EXISTS idx_scam_signals_scammer ON live_trading.scam_signals(scammer_address);
CREATE INDEX IF NOT EXISTS idx_scam_signals_type ON live_trading.scam_signals(scam_type);
CREATE INDEX IF NOT EXISTS idx_scam_signals_timestamp ON live_trading.scam_signals(detection_timestamp);

-- Grant permissions if needed (adjust user as necessary)
-- GRANT SELECT, INSERT ON live_trading.scam_signals TO your_app_user;
-- GRANT USAGE, SELECT ON SEQUENCE live_trading.scam_signals_signal_id_seq TO your_app_user;