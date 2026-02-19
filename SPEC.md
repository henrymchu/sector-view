# Sector View - Desktop Application Specification

## Overview

A macOS desktop application for analyzing S&P 500 sectors and identifying stocks that deviate significantly from their sector averages. Built with Tauri (Rust + React) for optimal performance and native integration.

## Core Features

### 1. Sector Dashboard
- Display all 11 GICS sectors with key metrics:
  - Sector average price change (day/week/month)
  - Volume weighted averages
  - P/E ratio ranges
  - Market cap distribution

### 2. Outlier Detection
- Identify stocks that deviate >1.5 standard deviations from sector averages
- Configurable thresholds (1.5σ, 2σ, custom)
- Multiple metrics: price change %, volume, volatility, valuation ratios
- Color-coded visualization (green = underperforming, red = outperforming)

### 3. Real-time Updates
- Refresh data every 5-15 minutes during market hours
- Historical comparison (vs yesterday, last week)
- Pause/resume functionality for focused analysis

## Technical Architecture

### Frontend (React + TypeScript)
```
src/
├── components/
│   ├── SectorGrid.tsx        # Main sector overview
│   ├── OutlierList.tsx       # Outlier stock display
│   ├── StockCard.tsx         # Individual stock info
│   └── SettingsPanel.tsx     # Configuration UI
├── hooks/
│   ├── useMarketData.ts      # Data fetching logic
│   └── useOutlierDetection.ts # Statistical calculations
├── types/
│   └── market.ts             # TypeScript interfaces
└── utils/
    └── statistics.ts         # Math utilities
```

### Backend (Rust via Tauri)
```
src-tauri/
├── src/
│   ├── main.rs               # Tauri setup
│   ├── commands.rs           # Frontend API commands
│   ├── database.rs           # SQLite operations
│   ├── data_fetcher.rs       # External API calls
│   └── analytics.rs          # Statistical analysis
└── Cargo.toml
```

### Data Pipeline (Python FastAPI)
```
data_service/
├── main.py                   # FastAPI server
├── models.py                 # Data models
├── fetchers/
│   ├── yahoo_finance.py      # Yahoo Finance API
│   ├── alpha_vantage.py      # Alternative data source
│   └── polygon_io.py         # Backup data source
└── analytics.py              # Outlier calculations
```

## Data Sources

### Primary: Yahoo Finance API
- Free tier: 2000 requests/hour
- Real-time quotes with 15-minute delay
- Historical data available

### Backup: Alpha Vantage
- Free tier: 500 requests/day  
- Real-time data
- Fundamental data (P/E, etc.)

### Market Structure
- **11 GICS Sectors**: Technology, Healthcare, Financials, Consumer Discretionary, Communication Services, Industrials, Consumer Staples, Energy, Utilities, Real Estate, Materials
- **~500 stocks** from S&P 500 index

## Database Schema (SQLite)

```sql
-- Sectors table
CREATE TABLE sectors (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    description TEXT
);

-- Stocks table  
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    sector_id INTEGER,
    market_cap INTEGER,
    FOREIGN KEY (sector_id) REFERENCES sectors (id)
);

-- Market data (time series)
CREATE TABLE market_data (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    timestamp DATETIME,
    price REAL,
    volume INTEGER,
    change_percent REAL,
    pe_ratio REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);

-- Outlier alerts
CREATE TABLE outlier_alerts (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    metric_type TEXT,
    deviation_score REAL,
    threshold REAL,
    detected_at DATETIME,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);
```

## API Design (Tauri Commands)

```rust
#[tauri::command]
async fn get_sector_data() -> Result<Vec<SectorSummary>, String>

#[tauri::command]
async fn get_outliers(threshold: f64) -> Result<Vec<OutlierStock>, String>

#[tauri::command]
async fn refresh_market_data() -> Result<String, String>

#[tauri::command]
async fn get_stock_history(symbol: String, days: u32) -> Result<Vec<PricePoint>, String>

#[tauri::command]
async fn update_settings(config: AppSettings) -> Result<(), String>
```

## User Interface

### Main Window (1200x800)
```
┌─────────────────────────────────────────────────────┐
│ [File] [View] [Settings]                    [Refresh]│
├─────────────────────────────────────────────────────┤
│                                                     │
│  Sector Overview (Grid 3x4)                        │
│  ┌────────┐ ┌────────┐ ┌────────┐                   │
│  │Tech    │ │Health  │ │Finance │                   │
│  │+2.1%   │ │-0.8%   │ │+1.4%   │                   │
│  │23 out  │ │12 out  │ │8 out   │                   │
│  └────────┘ └────────┘ └────────┘                   │
│                                                     │
├─────────────────────────────────────────────────────┤
│  Outliers (Scrollable List)                        │
│  🔴 AAPL: +5.2% (Tech avg: +2.1%, 2.1σ)           │
│  🟢 META: -3.1% (Tech avg: +2.1%, -2.8σ)          │
│  🔴 TSLA: +8.7% (Consumer Disc avg: +1.2%, 3.2σ)  │
└─────────────────────────────────────────────────────┘
```

### Settings Panel
- Outlier threshold slider (1.0σ - 3.0σ)
- Refresh interval (5min - 30min)
- Metric selection (Price, Volume, P/E, etc.)
- Data source priority

## Development Phases

### Phase 1: Foundation (Week 1-2)
- [x] Git repository setup
- [ ] Tauri project initialization
- [ ] Basic React UI with mock data
- [ ] SQLite database setup
- [ ] Simple outlier detection algorithm

### Phase 2: Data Integration (Week 3-4)  
- [ ] Yahoo Finance API integration
- [ ] Real-time data pipeline
- [ ] Database population scripts
- [ ] Error handling and fallbacks

### Phase 3: Advanced Features (Week 5-6)
- [ ] Historical analysis
- [ ] Multiple outlier detection methods
- [ ] Export functionality (CSV, JSON)
- [ ] Performance optimization

### Phase 4: Polish (Week 7-8)
- [ ] UI/UX improvements
- [ ] App icon and branding
- [ ] macOS-specific integrations
- [ ] Documentation and README

## Success Metrics

### Technical Goals
- App bundle size < 15MB
- Startup time < 2 seconds
- Data refresh < 10 seconds
- Memory usage < 100MB

### Portfolio Goals
- Demonstrates full-stack development
- Shows understanding of financial data
- Proves ability to build native desktop apps
- Clean, documented code for GitHub showcase

## Future Enhancements

- **Portfolio tracking**: Allow users to add their own stocks
- **Alerts**: Push notifications for significant outliers
- **Backtesting**: Historical outlier performance analysis  
- **Sector rotation**: Identify trending sectors
- **Options data**: Include options flow analysis
- **Mobile companion**: iOS/Android app for alerts

## Risk Mitigation

### API Rate Limits
- Implement request caching
- Use multiple data sources
- Graceful degradation when limits hit

### Data Quality
- Validate all incoming data
- Handle missing/stale data gracefully
- Log data inconsistencies

### Performance
- Database indexing for fast queries
- Efficient React re-rendering
- Background data processing

---

**Last Updated**: January 2025  
**Status**: Specification Phase  
**Next**: Initialize Tauri project structure