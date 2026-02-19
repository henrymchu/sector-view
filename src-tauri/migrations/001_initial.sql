CREATE TABLE IF NOT EXISTS sectors (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    sector_id INTEGER NULL REFERENCES sectors(id)
);

-- Seed the 11 GICS sectors
INSERT OR IGNORE INTO sectors (name, symbol) VALUES
    ('Technology', 'XLK'),
    ('Health Care', 'XLV'),
    ('Financials', 'XLF'),
    ('Consumer Discretionary', 'XLY'),
    ('Communication Services', 'XLC'),
    ('Industrials', 'XLI'),
    ('Consumer Staples', 'XLP'),
    ('Energy', 'XLE'),
    ('Utilities', 'XLU'),
    ('Real Estate', 'XLRE'),
    ('Materials', 'XLB');
