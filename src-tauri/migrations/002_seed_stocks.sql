-- Seed representative S&P 500 stocks (~5 per sector)
-- GOOGL and META assigned to Communication Services (primary GICS classification)

-- Technology (XLK)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('AAPL', 'Apple Inc.', (SELECT id FROM sectors WHERE symbol = 'XLK')),
    ('MSFT', 'Microsoft Corp.', (SELECT id FROM sectors WHERE symbol = 'XLK')),
    ('NVDA', 'NVIDIA Corp.', (SELECT id FROM sectors WHERE symbol = 'XLK')),
    ('AVGO', 'Broadcom Inc.', (SELECT id FROM sectors WHERE symbol = 'XLK')),
    ('CRM', 'Salesforce Inc.', (SELECT id FROM sectors WHERE symbol = 'XLK'));

-- Health Care (XLV)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('JNJ', 'Johnson & Johnson', (SELECT id FROM sectors WHERE symbol = 'XLV')),
    ('PFE', 'Pfizer Inc.', (SELECT id FROM sectors WHERE symbol = 'XLV')),
    ('UNH', 'UnitedHealth Group Inc.', (SELECT id FROM sectors WHERE symbol = 'XLV')),
    ('ABBV', 'AbbVie Inc.', (SELECT id FROM sectors WHERE symbol = 'XLV')),
    ('TMO', 'Thermo Fisher Scientific Inc.', (SELECT id FROM sectors WHERE symbol = 'XLV'));

-- Financials (XLF)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('JPM', 'JPMorgan Chase & Co.', (SELECT id FROM sectors WHERE symbol = 'XLF')),
    ('BAC', 'Bank of America Corp.', (SELECT id FROM sectors WHERE symbol = 'XLF')),
    ('WFC', 'Wells Fargo & Co.', (SELECT id FROM sectors WHERE symbol = 'XLF')),
    ('GS', 'Goldman Sachs Group Inc.', (SELECT id FROM sectors WHERE symbol = 'XLF')),
    ('MS', 'Morgan Stanley', (SELECT id FROM sectors WHERE symbol = 'XLF'));

-- Consumer Discretionary (XLY)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('AMZN', 'Amazon.com Inc.', (SELECT id FROM sectors WHERE symbol = 'XLY')),
    ('TSLA', 'Tesla Inc.', (SELECT id FROM sectors WHERE symbol = 'XLY')),
    ('HD', 'Home Depot Inc.', (SELECT id FROM sectors WHERE symbol = 'XLY')),
    ('MCD', 'McDonald''s Corp.', (SELECT id FROM sectors WHERE symbol = 'XLY')),
    ('NKE', 'Nike Inc.', (SELECT id FROM sectors WHERE symbol = 'XLY'));

-- Communication Services (XLC)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('GOOGL', 'Alphabet Inc.', (SELECT id FROM sectors WHERE symbol = 'XLC')),
    ('META', 'Meta Platforms Inc.', (SELECT id FROM sectors WHERE symbol = 'XLC')),
    ('VZ', 'Verizon Communications Inc.', (SELECT id FROM sectors WHERE symbol = 'XLC')),
    ('T', 'AT&T Inc.', (SELECT id FROM sectors WHERE symbol = 'XLC')),
    ('DIS', 'Walt Disney Co.', (SELECT id FROM sectors WHERE symbol = 'XLC'));

-- Industrials (XLI)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('BA', 'Boeing Co.', (SELECT id FROM sectors WHERE symbol = 'XLI')),
    ('CAT', 'Caterpillar Inc.', (SELECT id FROM sectors WHERE symbol = 'XLI')),
    ('UPS', 'United Parcel Service Inc.', (SELECT id FROM sectors WHERE symbol = 'XLI')),
    ('GE', 'GE Aerospace', (SELECT id FROM sectors WHERE symbol = 'XLI')),
    ('RTX', 'RTX Corp.', (SELECT id FROM sectors WHERE symbol = 'XLI'));

-- Consumer Staples (XLP)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('PG', 'Procter & Gamble Co.', (SELECT id FROM sectors WHERE symbol = 'XLP')),
    ('KO', 'Coca-Cola Co.', (SELECT id FROM sectors WHERE symbol = 'XLP')),
    ('PEP', 'PepsiCo Inc.', (SELECT id FROM sectors WHERE symbol = 'XLP')),
    ('WMT', 'Walmart Inc.', (SELECT id FROM sectors WHERE symbol = 'XLP')),
    ('COST', 'Costco Wholesale Corp.', (SELECT id FROM sectors WHERE symbol = 'XLP'));

-- Energy (XLE)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('XOM', 'Exxon Mobil Corp.', (SELECT id FROM sectors WHERE symbol = 'XLE')),
    ('CVX', 'Chevron Corp.', (SELECT id FROM sectors WHERE symbol = 'XLE')),
    ('COP', 'ConocoPhillips', (SELECT id FROM sectors WHERE symbol = 'XLE')),
    ('EOG', 'EOG Resources Inc.', (SELECT id FROM sectors WHERE symbol = 'XLE')),
    ('SLB', 'Schlumberger Ltd.', (SELECT id FROM sectors WHERE symbol = 'XLE'));

-- Utilities (XLU)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('NEE', 'NextEra Energy Inc.', (SELECT id FROM sectors WHERE symbol = 'XLU')),
    ('SO', 'Southern Co.', (SELECT id FROM sectors WHERE symbol = 'XLU')),
    ('DUK', 'Duke Energy Corp.', (SELECT id FROM sectors WHERE symbol = 'XLU')),
    ('AEP', 'American Electric Power Co.', (SELECT id FROM sectors WHERE symbol = 'XLU')),
    ('EXC', 'Exelon Corp.', (SELECT id FROM sectors WHERE symbol = 'XLU'));

-- Real Estate (XLRE)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('AMT', 'American Tower Corp.', (SELECT id FROM sectors WHERE symbol = 'XLRE')),
    ('PLD', 'Prologis Inc.', (SELECT id FROM sectors WHERE symbol = 'XLRE')),
    ('CCI', 'Crown Castle Inc.', (SELECT id FROM sectors WHERE symbol = 'XLRE')),
    ('EQIX', 'Equinix Inc.', (SELECT id FROM sectors WHERE symbol = 'XLRE')),
    ('SPG', 'Simon Property Group Inc.', (SELECT id FROM sectors WHERE symbol = 'XLRE'));

-- Materials (XLB)
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('LIN', 'Linde plc', (SELECT id FROM sectors WHERE symbol = 'XLB')),
    ('APD', 'Air Products & Chemicals Inc.', (SELECT id FROM sectors WHERE symbol = 'XLB')),
    ('SHW', 'Sherwin-Williams Co.', (SELECT id FROM sectors WHERE symbol = 'XLB')),
    ('FCX', 'Freeport-McMoRan Inc.', (SELECT id FROM sectors WHERE symbol = 'XLB')),
    ('DOW', 'Dow Inc.', (SELECT id FROM sectors WHERE symbol = 'XLB'));

-- Unclassified stocks (NULL sector_id) for edge case testing
INSERT OR IGNORE INTO stocks (symbol, name, sector_id) VALUES
    ('BRK.B', 'Berkshire Hathaway Inc.', NULL),
    ('PLTR', 'Palantir Technologies Inc.', NULL);
