-- Track which market universe each stock belongs to
CREATE TABLE IF NOT EXISTS stock_universe (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL REFERENCES stocks(id) ON DELETE CASCADE,
    universe_type TEXT NOT NULL CHECK (universe_type IN ('sp500', 'russell2000')),
    date_added TEXT NOT NULL DEFAULT (date('now')),
    date_removed TEXT,
    UNIQUE(stock_id, universe_type)
);

-- Add universe_type column to outlier_detections
ALTER TABLE outlier_detections ADD COLUMN universe_type TEXT NOT NULL DEFAULT 'sp500';

-- Seed all currently classified stocks as S&P 500 members
INSERT OR IGNORE INTO stock_universe (stock_id, universe_type)
SELECT id, 'sp500' FROM stocks WHERE sector_id IS NOT NULL;
