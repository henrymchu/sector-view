CREATE TABLE IF NOT EXISTS market_data (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL REFERENCES stocks(id),
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Price Data
    price REAL NOT NULL,
    price_change REAL NOT NULL,
    price_change_percent REAL NOT NULL,
    -- Volume & Liquidity
    volume INTEGER,
    avg_volume_10d INTEGER,
    -- Market & Valuation
    market_cap INTEGER,
    pe_ratio REAL,
    pb_ratio REAL,
    -- Profitability
    eps REAL,
    dividend_yield REAL,
    -- Volatility
    beta REAL,
    -- 52-Week Range
    week52_high REAL,
    week52_low REAL
);

CREATE INDEX IF NOT EXISTS idx_market_data_stock_timestamp ON market_data(stock_id, timestamp);

CREATE INDEX IF NOT EXISTS idx_market_data_timestamp ON market_data(timestamp);
