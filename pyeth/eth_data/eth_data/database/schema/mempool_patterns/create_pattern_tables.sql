-- Mempool Pattern Detection Tables
-- These tables store pattern sequences, scores, and timing analysis for real-time scam detection

-- Create schema if not exists
CREATE SCHEMA IF NOT EXISTS mempool_patterns;

-- Pattern sequence table - tracks function call sequences for each token
CREATE TABLE IF NOT EXISTS mempool_patterns.pattern_sequences (
    id BIGSERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    sequence_start TIMESTAMP NOT NULL,
    sequence_end TIMESTAMP,
    function_sequence TEXT[] NOT NULL,  -- Array of function selectors
    function_names TEXT[] NOT NULL,     -- Array of function names
    from_addresses VARCHAR(42)[] NOT NULL,  -- Array of caller addresses
    timestamps TIMESTAMP[] NOT NULL,    -- Array of timestamps for each function
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Indexes
    INDEX idx_pattern_seq_token (token_address),
    INDEX idx_pattern_seq_active (is_active),
    INDEX idx_pattern_seq_start (sequence_start)
);

-- Pattern matches table - stores detected patterns
CREATE TABLE IF NOT EXISTS mempool_patterns.pattern_matches (
    id BIGSERIAL PRIMARY KEY,
    sequence_id BIGINT REFERENCES mempool_patterns.pattern_sequences(id),
    pattern_id VARCHAR(50) NOT NULL,
    pattern_type VARCHAR(100) NOT NULL,
    risk_level VARCHAR(20) NOT NULL CHECK (risk_level IN ('CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'MINIMAL')),
    confidence FLOAT NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    matched_sequence TEXT[] NOT NULL,  -- The specific sequence that matched
    timing_ms INTEGER,  -- Time between first and last function in pattern
    metadata JSONB,  -- Additional pattern-specific data
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Indexes
    INDEX idx_pattern_match_seq (sequence_id),
    INDEX idx_pattern_match_pattern (pattern_id),
    INDEX idx_pattern_match_risk (risk_level),
    INDEX idx_pattern_match_detected (detected_at)
);

-- Pattern scores table - aggregates risk scores per token
CREATE TABLE IF NOT EXISTS mempool_patterns.pattern_scores (
    token_address VARCHAR(42) PRIMARY KEY,
    total_risk_score INTEGER NOT NULL DEFAULT 0,
    risk_level VARCHAR(20) NOT NULL DEFAULT 'MINIMAL',
    pattern_count INTEGER NOT NULL DEFAULT 0,
    critical_patterns INTEGER NOT NULL DEFAULT 0,
    high_patterns INTEGER NOT NULL DEFAULT 0,
    medium_patterns INTEGER NOT NULL DEFAULT 0,
    low_patterns INTEGER NOT NULL DEFAULT 0,
    first_pattern_at TIMESTAMP,
    last_pattern_at TIMESTAMP,
    overall_confidence FLOAT,
    metadata JSONB,  -- Store pattern summary, creator info, etc.
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Indexes
    INDEX idx_pattern_score_risk (risk_level),
    INDEX idx_pattern_score_total (total_risk_score DESC),
    INDEX idx_pattern_score_updated (updated_at)
);

-- Timing analysis table - tracks time between critical events
CREATE TABLE IF NOT EXISTS mempool_patterns.timing_analysis (
    id BIGSERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    event_type VARCHAR(50) NOT NULL,  -- e.g., 'trading_enabled', 'liquidity_added', 'liquidity_removed'
    event_timestamp TIMESTAMP NOT NULL,
    event_function VARCHAR(10) NOT NULL,  -- Function selector
    event_tx_hash VARCHAR(66) NOT NULL,
    from_address VARCHAR(42) NOT NULL,
    time_since_creation_ms BIGINT,
    time_since_trading_ms BIGINT,
    metadata JSONB,
    
    -- Indexes
    INDEX idx_timing_token (token_address),
    INDEX idx_timing_event (event_type),
    INDEX idx_timing_timestamp (event_timestamp),
    UNIQUE idx_timing_unique_event (token_address, event_type, event_tx_hash)
);

-- Pattern statistics table - for ML training and analysis
CREATE TABLE IF NOT EXISTS mempool_patterns.pattern_statistics (
    pattern_id VARCHAR(50) PRIMARY KEY,
    total_detections INTEGER NOT NULL DEFAULT 0,
    true_positives INTEGER NOT NULL DEFAULT 0,
    false_positives INTEGER NOT NULL DEFAULT 0,
    false_negatives INTEGER NOT NULL DEFAULT 0,
    precision FLOAT,
    recall FLOAT,
    f1_score FLOAT,
    avg_confidence FLOAT,
    last_detected_at TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Real-time alerts table - for immediate notification of high-risk patterns
CREATE TABLE IF NOT EXISTS mempool_patterns.pattern_alerts (
    id BIGSERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    pattern_id VARCHAR(50) NOT NULL,
    risk_level VARCHAR(20) NOT NULL,
    alert_type VARCHAR(50) NOT NULL,  -- 'immediate_rug', 'high_tax', 'honeypot', etc.
    alert_message TEXT NOT NULL,
    confidence FLOAT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_at TIMESTAMP,
    
    -- Indexes
    INDEX idx_alert_token (token_address),
    INDEX idx_alert_risk (risk_level),
    INDEX idx_alert_created (created_at DESC),
    INDEX idx_alert_ack (acknowledged)
);

-- Function to update pattern scores
CREATE OR REPLACE FUNCTION mempool_patterns.update_pattern_score(
    p_token_address VARCHAR(42),
    p_risk_score INTEGER,
    p_risk_level VARCHAR(20),
    p_pattern_type VARCHAR(20)
) RETURNS VOID AS $$
BEGIN
    INSERT INTO mempool_patterns.pattern_scores (
        token_address,
        total_risk_score,
        risk_level,
        pattern_count,
        critical_patterns,
        high_patterns,
        medium_patterns,
        low_patterns,
        first_pattern_at,
        last_pattern_at
    ) VALUES (
        p_token_address,
        p_risk_score,
        p_risk_level,
        1,
        CASE WHEN p_pattern_type = 'CRITICAL' THEN 1 ELSE 0 END,
        CASE WHEN p_pattern_type = 'HIGH' THEN 1 ELSE 0 END,
        CASE WHEN p_pattern_type = 'MEDIUM' THEN 1 ELSE 0 END,
        CASE WHEN p_pattern_type = 'LOW' THEN 1 ELSE 0 END,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    )
    ON CONFLICT (token_address) DO UPDATE SET
        total_risk_score = pattern_scores.total_risk_score + p_risk_score,
        risk_level = p_risk_level,
        pattern_count = pattern_scores.pattern_count + 1,
        critical_patterns = pattern_scores.critical_patterns + CASE WHEN p_pattern_type = 'CRITICAL' THEN 1 ELSE 0 END,
        high_patterns = pattern_scores.high_patterns + CASE WHEN p_pattern_type = 'HIGH' THEN 1 ELSE 0 END,
        medium_patterns = pattern_scores.medium_patterns + CASE WHEN p_pattern_type = 'MEDIUM' THEN 1 ELSE 0 END,
        low_patterns = pattern_scores.low_patterns + CASE WHEN p_pattern_type = 'LOW' THEN 1 ELSE 0 END,
        last_pattern_at = CURRENT_TIMESTAMP,
        updated_at = CURRENT_TIMESTAMP;
END;
$$ LANGUAGE plpgsql;

-- Function to create pattern alert
CREATE OR REPLACE FUNCTION mempool_patterns.create_pattern_alert(
    p_token_address VARCHAR(42),
    p_pattern_id VARCHAR(50),
    p_risk_level VARCHAR(20),
    p_alert_type VARCHAR(50),
    p_alert_message TEXT,
    p_confidence FLOAT,
    p_metadata JSONB DEFAULT NULL
) RETURNS BIGINT AS $$
DECLARE
    v_alert_id BIGINT;
BEGIN
    INSERT INTO mempool_patterns.pattern_alerts (
        token_address,
        pattern_id,
        risk_level,
        alert_type,
        alert_message,
        confidence,
        metadata
    ) VALUES (
        p_token_address,
        p_pattern_id,
        p_risk_level,
        p_alert_type,
        p_alert_message,
        p_confidence,
        p_metadata
    ) RETURNING id INTO v_alert_id;
    
    RETURN v_alert_id;
END;
$$ LANGUAGE plpgsql;

-- Create update trigger for updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_pattern_sequences_updated_at BEFORE UPDATE
    ON mempool_patterns.pattern_sequences FOR EACH ROW
    EXECUTE PROCEDURE update_updated_at_column();

CREATE TRIGGER update_pattern_scores_updated_at BEFORE UPDATE
    ON mempool_patterns.pattern_scores FOR EACH ROW
    EXECUTE PROCEDURE update_updated_at_column();

-- Create view for high-risk tokens
CREATE OR REPLACE VIEW mempool_patterns.high_risk_tokens AS
SELECT 
    ps.token_address,
    ps.total_risk_score,
    ps.risk_level,
    ps.pattern_count,
    ps.critical_patterns,
    ps.high_patterns,
    ps.overall_confidence,
    ps.last_pattern_at,
    COALESCE(
        (SELECT array_agg(DISTINCT pm.pattern_type ORDER BY pm.pattern_type)
         FROM mempool_patterns.pattern_matches pm
         JOIN mempool_patterns.pattern_sequences seq ON pm.sequence_id = seq.id
         WHERE seq.token_address = ps.token_address
           AND pm.risk_level IN ('CRITICAL', 'HIGH')
        ), 
        ARRAY[]::TEXT[]
    ) AS critical_pattern_types
FROM mempool_patterns.pattern_scores ps
WHERE ps.risk_level IN ('CRITICAL', 'HIGH')
   OR ps.total_risk_score >= 100
ORDER BY ps.total_risk_score DESC;

-- Create materialized view for pattern effectiveness
CREATE MATERIALIZED VIEW IF NOT EXISTS mempool_patterns.pattern_effectiveness AS
WITH pattern_stats AS (
    SELECT 
        pm.pattern_id,
        pm.pattern_type,
        pm.risk_level,
        COUNT(*) as detection_count,
        AVG(pm.confidence) as avg_confidence,
        COUNT(DISTINCT seq.token_address) as unique_tokens
    FROM mempool_patterns.pattern_matches pm
    JOIN mempool_patterns.pattern_sequences seq ON pm.sequence_id = seq.id
    GROUP BY pm.pattern_id, pm.pattern_type, pm.risk_level
)
SELECT 
    ps.*,
    COALESCE(pst.precision, 0) as precision,
    COALESCE(pst.recall, 0) as recall,
    COALESCE(pst.f1_score, 0) as f1_score
FROM pattern_stats ps
LEFT JOIN mempool_patterns.pattern_statistics pst ON ps.pattern_id = pst.pattern_id
ORDER BY ps.detection_count DESC;

-- Create index on materialized view
CREATE INDEX idx_pattern_effectiveness_pattern ON mempool_patterns.pattern_effectiveness(pattern_id);

-- Grant permissions (adjust as needed)
GRANT SELECT ON ALL TABLES IN SCHEMA mempool_patterns TO readonly_user;
GRANT ALL ON ALL TABLES IN SCHEMA mempool_patterns TO app_user;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA mempool_patterns TO app_user;