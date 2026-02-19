use reqwest::Client;
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;

/// Response structures for Yahoo Finance chart API (v8)
#[derive(Debug, Deserialize)]
struct ChartResponse {
    chart: ChartResult,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    result: Option<Vec<ChartData>>,
}

#[derive(Debug, Deserialize)]
struct ChartData {
    meta: ChartMeta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartMeta {
    regular_market_price: Option<f64>,
    chart_previous_close: Option<f64>,
    regular_market_volume: Option<i64>,
}

/// Response structures for Yahoo Finance quoteSummary API (v10)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteSummaryResponse {
    quote_summary: Option<QuoteSummaryResult>,
}

#[derive(Debug, Deserialize)]
struct QuoteSummaryResult {
    result: Option<Vec<QuoteSummaryData>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuoteSummaryData {
    default_key_statistics: Option<KeyStatistics>,
    summary_detail: Option<SummaryDetail>,
    price: Option<PriceData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyStatistics {
    beta: Option<YahooValue>,
    trailing_eps: Option<YahooValue>,
    price_to_book: Option<YahooValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryDetail {
    trailing_pe: Option<YahooValue>,
    dividend_yield: Option<YahooValue>,
    fifty_two_week_high: Option<YahooValue>,
    fifty_two_week_low: Option<YahooValue>,
    average_volume_10days: Option<YahooValue>,
    market_cap: Option<YahooValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceData {
    market_cap: Option<YahooValue>,
}

/// Yahoo Finance wraps many values in {"raw": 123.45, "fmt": "123.45"}
#[derive(Debug, Deserialize)]
struct YahooValue {
    raw: Option<f64>,
}

/// Combined stock quote with all metrics
#[derive(Debug)]
pub struct StockQuote {
    pub stock_id: i32,
    pub price: f64,
    pub price_change: f64,
    pub price_change_percent: f64,
    pub volume: Option<i64>,
    pub avg_volume_10d: Option<i64>,
    pub market_cap: Option<i64>,
    pub pe_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub eps: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub beta: Option<f64>,
    pub week52_high: Option<f64>,
    pub week52_low: Option<f64>,
}

/// Fetch price data from Yahoo Finance chart API.
async fn fetch_chart_data(
    client: &Client,
    symbol: &str,
) -> Result<(f64, f64, Option<i64>), String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=1d",
        symbol
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| format!("Network error fetching {symbol}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Yahoo chart API returned {} for {symbol}",
            resp.status()
        ));
    }

    let data: ChartResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse chart data for {symbol}: {e}"))?;

    let result = data
        .chart
        .result
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| format!("No chart data for {symbol}"))?;

    let price = result
        .meta
        .regular_market_price
        .ok_or_else(|| format!("No price for {symbol}"))?;
    let prev_close = result.meta.chart_previous_close.unwrap_or(price);
    let volume = result.meta.regular_market_volume;

    Ok((price, prev_close, volume))
}

/// Fetch fundamental data from Yahoo Finance quoteSummary API.
async fn fetch_fundamentals(
    client: &Client,
    symbol: &str,
) -> (
    Option<f64>,
    Option<f64>,
    Option<i64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<i64>,
    Option<f64>,
    Option<f64>,
) {
    let url = format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=defaultKeyStatistics,summaryDetail,price",
        symbol
    );

    let resp = match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        _ => return (None, None, None, None, None, None, None, None, None),
    };

    let data: QuoteSummaryResponse = match resp.json().await {
        Ok(d) => d,
        Err(_) => return (None, None, None, None, None, None, None, None, None),
    };

    let result = match data
        .quote_summary
        .and_then(|qs| qs.result)
        .and_then(|r| r.into_iter().next())
    {
        Some(r) => r,
        None => return (None, None, None, None, None, None, None, None, None),
    };

    let pe_ratio = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.trailing_pe.as_ref())
        .and_then(|v| v.raw);
    let pb_ratio = result
        .default_key_statistics
        .as_ref()
        .and_then(|ks| ks.price_to_book.as_ref())
        .and_then(|v| v.raw);
    let market_cap = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.market_cap.as_ref())
        .and_then(|v| v.raw)
        .or_else(|| {
            result
                .price
                .as_ref()
                .and_then(|p| p.market_cap.as_ref())
                .and_then(|v| v.raw)
        })
        .map(|v| v as i64);
    let eps = result
        .default_key_statistics
        .as_ref()
        .and_then(|ks| ks.trailing_eps.as_ref())
        .and_then(|v| v.raw);
    let dividend_yield = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.dividend_yield.as_ref())
        .and_then(|v| v.raw);
    let beta = result
        .default_key_statistics
        .as_ref()
        .and_then(|ks| ks.beta.as_ref())
        .and_then(|v| v.raw);
    let avg_volume_10d = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.average_volume_10days.as_ref())
        .and_then(|v| v.raw)
        .map(|v| v as i64);
    let week52_high = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.fifty_two_week_high.as_ref())
        .and_then(|v| v.raw);
    let week52_low = result
        .summary_detail
        .as_ref()
        .and_then(|sd| sd.fifty_two_week_low.as_ref())
        .and_then(|v| v.raw);

    (
        pe_ratio,
        pb_ratio,
        market_cap,
        eps,
        dividend_yield,
        beta,
        avg_volume_10d,
        week52_high,
        week52_low,
    )
}

/// Fetch quote for a single stock, combining chart + fundamentals.
pub async fn fetch_stock_quote(
    client: &Client,
    stock_id: i32,
    symbol: &str,
) -> Result<StockQuote, String> {
    let (price, prev_close, volume) = fetch_chart_data(client, symbol).await?;

    let price_change = price - prev_close;
    let price_change_percent = if prev_close != 0.0 {
        (price_change / prev_close) * 100.0
    } else {
        0.0
    };

    let (pe_ratio, pb_ratio, market_cap, eps, dividend_yield, beta, avg_volume_10d, week52_high, week52_low) =
        fetch_fundamentals(client, symbol).await;

    Ok(StockQuote {
        stock_id,
        price,
        price_change,
        price_change_percent,
        volume,
        avg_volume_10d,
        market_cap,
        pe_ratio,
        pb_ratio,
        eps,
        dividend_yield,
        beta,
        week52_high,
        week52_low,
    })
}

/// Save a stock quote to the market_data table.
pub async fn save_quote(pool: &SqlitePool, quote: &StockQuote) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO market_data (
            stock_id, price, price_change, price_change_percent,
            volume, avg_volume_10d, market_cap, pe_ratio, pb_ratio,
            eps, dividend_yield, beta, week52_high, week52_low
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(quote.stock_id)
    .bind(quote.price)
    .bind(quote.price_change)
    .bind(quote.price_change_percent)
    .bind(quote.volume)
    .bind(quote.avg_volume_10d)
    .bind(quote.market_cap)
    .bind(quote.pe_ratio)
    .bind(quote.pb_ratio)
    .bind(quote.eps)
    .bind(quote.dividend_yield)
    .bind(quote.beta)
    .bind(quote.week52_high)
    .bind(quote.week52_low)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to save market data: {e}"))?;

    Ok(())
}
