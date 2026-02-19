CREATE TABLE IF NOT EXISTS outlier_detections (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL REFERENCES stocks(id),
    sector_id INTEGER NOT NULL REFERENCES sectors(id),
    detection_date TEXT NOT NULL DEFAULT (date('now')),
    detection_timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Z-Scores
    pe_z_score REAL,
    pb_z_score REAL,
    price_z_score REAL NOT NULL,
    volume_z_score REAL,
    -- Analysis
    composite_score REAL NOT NULL,
    outlier_type TEXT NOT NULL,
    significance_level TEXT NOT NULL,
    threshold_used REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_outlier_stock_date ON outlier_detections(stock_id, detection_date);

CREATE INDEX IF NOT EXISTS idx_outlier_sector_date ON outlier_detections(sector_id, detection_date);

CREATE INDEX IF NOT EXISTS idx_outlier_date ON outlier_detections(detection_date);
