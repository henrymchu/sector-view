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
    #[serde(rename = "trailingPE")]
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

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Build the Yahoo Finance chart API URL for a given symbol.
fn build_chart_url(symbol: &str) -> String {
    format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1d&interval=1d",
        symbol
    )
}

/// Build the Yahoo Finance quoteSummary API URL for a given symbol and crumb.
fn build_fundamentals_url(symbol: &str, crumb: &str) -> String {
    format!(
        "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=defaultKeyStatistics,summaryDetail,price&crumb={}",
        symbol, crumb
    )
}

/// Calculate price change and percent change from current price and previous close.
fn calculate_price_change(price: f64, prev_close: f64) -> (f64, f64) {
    let change = price - prev_close;
    let percent = if prev_close != 0.0 {
        (change / prev_close) * 100.0
    } else {
        0.0
    };
    (change, percent)
}

/// Authenticated Yahoo Finance session with cookie jar + crumb.
/// Created once per refresh cycle and reused for all quoteSummary calls.
pub struct YahooSession {
    client: Client,
    crumb: String,
}

impl YahooSession {
    /// Establish a Yahoo Finance session by fetching a cookie + crumb pair.
    pub async fn new() -> Result<Self, String> {
        let client = Client::builder()
            .cookie_store(true)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| format!("Failed to build Yahoo session client: {e}"))?;

        // Step 1: Hit fc.yahoo.com to get session cookies
        client
            .get("https://fc.yahoo.com")
            .send()
            .await
            .map_err(|e| format!("Failed to init Yahoo session: {e}"))?;

        // Step 2: Fetch crumb using the session cookies
        let crumb = client
            .get("https://query2.finance.yahoo.com/v1/test/getcrumb")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch Yahoo crumb: {e}"))?
            .text()
            .await
            .map_err(|e| format!("Failed to read Yahoo crumb: {e}"))?;

        if crumb.contains("Unauthorized") || crumb.contains("Too Many") {
            return Err(format!("Yahoo crumb fetch rejected: {crumb}"));
        }

        Ok(Self { client, crumb })
    }
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
    let url = build_chart_url(symbol);

    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
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
    session: &YahooSession,
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
    let url = build_fundamentals_url(symbol, &session.crumb);

    let resp = match session.client
        .get(&url)
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
    session: &YahooSession,
    stock_id: i32,
    symbol: &str,
) -> Result<StockQuote, String> {
    let (price, prev_close, volume) = fetch_chart_data(client, symbol).await?;

    let (price_change, price_change_percent) = calculate_price_change(price, prev_close);

    let (pe_ratio, pb_ratio, market_cap, eps, dividend_yield, beta, avg_volume_10d, week52_high, week52_low) =
        fetch_fundamentals(session, symbol).await;

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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    // ---- URL construction ----

    #[test]
    fn test_build_chart_url_standard_symbol() {
        let url = build_chart_url("AAPL");
        assert_eq!(
            url,
            "https://query1.finance.yahoo.com/v8/finance/chart/AAPL?range=1d&interval=1d"
        );
    }

    #[test]
    fn test_build_chart_url_with_dot_symbol() {
        // Symbols like BRK.B should appear unmodified in the URL
        let url = build_chart_url("BRK.B");
        assert!(url.contains("BRK.B"), "URL should contain 'BRK.B': {url}");
    }

    #[test]
    fn test_build_chart_url_contains_required_query_params() {
        let url = build_chart_url("MSFT");
        assert!(url.contains("range=1d"), "Missing range param: {url}");
        assert!(url.contains("interval=1d"), "Missing interval param: {url}");
    }

    #[test]
    fn test_build_chart_url_symbol_varies() {
        assert_ne!(build_chart_url("AAPL"), build_chart_url("MSFT"));
    }

    #[test]
    fn test_build_fundamentals_url_contains_symbol_and_crumb() {
        let url = build_fundamentals_url("AAPL", "my-crumb");
        assert!(url.contains("AAPL"), "Missing symbol: {url}");
        assert!(url.contains("crumb=my-crumb"), "Missing crumb: {url}");
    }

    #[test]
    fn test_build_fundamentals_url_contains_required_modules() {
        let url = build_fundamentals_url("AAPL", "crumb");
        assert!(url.contains("defaultKeyStatistics"), "Missing module: {url}");
        assert!(url.contains("summaryDetail"), "Missing module: {url}");
        assert!(url.contains("price"), "Missing module: {url}");
    }

    #[test]
    fn test_build_fundamentals_url_crumb_varies() {
        let url1 = build_fundamentals_url("AAPL", "crumb-aaa");
        let url2 = build_fundamentals_url("AAPL", "crumb-bbb");
        assert!(url1.contains("crumb-aaa"));
        assert!(url2.contains("crumb-bbb"));
        assert_ne!(url1, url2);
    }

    #[test]
    fn test_build_fundamentals_url_symbol_varies() {
        let url_msft = build_fundamentals_url("MSFT", "crumb");
        let url_goog = build_fundamentals_url("GOOGL", "crumb");
        assert!(url_msft.contains("MSFT"));
        assert!(url_goog.contains("GOOGL"));
        assert_ne!(url_msft, url_goog);
    }

    // ---- Price change calculation ----

    #[test]
    fn test_calculate_price_change_gain() {
        let (change, pct) = calculate_price_change(110.0, 100.0);
        assert!(approx_eq(change, 10.0));
        assert!(approx_eq(pct, 10.0));
    }

    #[test]
    fn test_calculate_price_change_loss() {
        let (change, pct) = calculate_price_change(90.0, 100.0);
        assert!(approx_eq(change, -10.0));
        assert!(approx_eq(pct, -10.0));
    }

    #[test]
    fn test_calculate_price_change_no_movement() {
        let (change, pct) = calculate_price_change(100.0, 100.0);
        assert!(approx_eq(change, 0.0));
        assert!(approx_eq(pct, 0.0));
    }

    #[test]
    fn test_calculate_price_change_zero_prev_close_returns_zero_percent() {
        // Prevent division by zero — percent should be 0.0 not NaN/Inf
        let (change, pct) = calculate_price_change(150.0, 0.0);
        assert!(approx_eq(change, 150.0));
        assert!(approx_eq(pct, 0.0));
        assert!(pct.is_finite());
    }

    #[test]
    fn test_calculate_price_change_fractional() {
        let (change, pct) = calculate_price_change(150.25, 147.50);
        assert!(approx_eq(change, 2.75));
        assert!(approx_eq(pct, (2.75 / 147.50) * 100.0));
    }

    // ---- JSON parsing: ChartResponse ----

    #[test]
    fn test_chart_json_full_response() {
        let json = r#"{
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 150.25,
                        "chartPreviousClose": 147.50,
                        "regularMarketVolume": 75000000
                    }
                }]
            }
        }"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        let meta = &parsed.chart.result.unwrap()[0].meta;
        assert!(approx_eq(meta.regular_market_price.unwrap(), 150.25));
        assert!(approx_eq(meta.chart_previous_close.unwrap(), 147.50));
        assert_eq!(meta.regular_market_volume, Some(75_000_000));
    }

    #[test]
    fn test_chart_json_null_result() {
        let json = r#"{"chart": {"result": null}}"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.chart.result.is_none());
    }

    #[test]
    fn test_chart_json_empty_result_array() {
        let json = r#"{"chart": {"result": []}}"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.chart.result.unwrap().is_empty());
    }

    #[test]
    fn test_chart_json_missing_price_field_gives_none() {
        let json = r#"{
            "chart": {
                "result": [{
                    "meta": {
                        "chartPreviousClose": 147.50,
                        "regularMarketVolume": 50000000
                    }
                }]
            }
        }"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        let meta = &parsed.chart.result.unwrap()[0].meta;
        assert!(meta.regular_market_price.is_none());
        assert!(approx_eq(meta.chart_previous_close.unwrap(), 147.50));
    }

    #[test]
    fn test_chart_json_null_volume() {
        let json = r#"{
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 150.0,
                        "chartPreviousClose": 148.0,
                        "regularMarketVolume": null
                    }
                }]
            }
        }"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        let meta = &parsed.chart.result.unwrap()[0].meta;
        assert!(meta.regular_market_volume.is_none());
    }

    #[test]
    fn test_chart_json_missing_volume_field_gives_none() {
        let json = r#"{
            "chart": {
                "result": [{
                    "meta": {
                        "regularMarketPrice": 200.0,
                        "chartPreviousClose": 195.0
                    }
                }]
            }
        }"#;
        let parsed: ChartResponse = serde_json::from_str(json).unwrap();
        let meta = &parsed.chart.result.unwrap()[0].meta;
        assert!(meta.regular_market_volume.is_none());
    }

    // ---- JSON parsing: QuoteSummaryResponse ----

    #[test]
    fn test_fundamentals_json_full_response() {
        let json = r#"{
            "quoteSummary": {
                "result": [{
                    "defaultKeyStatistics": {
                        "beta": {"raw": 1.2, "fmt": "1.20"},
                        "trailingEps": {"raw": 6.05, "fmt": "6.05"},
                        "priceToBook": {"raw": 8.5, "fmt": "8.50"}
                    },
                    "summaryDetail": {
                        "trailingPE": {"raw": 28.5, "fmt": "28.50"},
                        "dividendYield": {"raw": 0.006, "fmt": "0.60%"},
                        "fiftyTwoWeekHigh": {"raw": 198.23, "fmt": "198.23"},
                        "fiftyTwoWeekLow": {"raw": 124.17, "fmt": "124.17"},
                        "averageVolume10days": {"raw": 55000000.0, "fmt": "55M"},
                        "marketCap": {"raw": 2400000000000.0, "fmt": "2.4T"}
                    },
                    "price": {
                        "marketCap": {"raw": 2400000000000.0, "fmt": "2.4T"}
                    }
                }]
            }
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let result = &parsed.quote_summary.unwrap().result.unwrap()[0];

        let ks = result.default_key_statistics.as_ref().unwrap();
        assert!(approx_eq(ks.beta.as_ref().unwrap().raw.unwrap(), 1.2));
        assert!(approx_eq(ks.trailing_eps.as_ref().unwrap().raw.unwrap(), 6.05));
        assert!(approx_eq(ks.price_to_book.as_ref().unwrap().raw.unwrap(), 8.5));

        let sd = result.summary_detail.as_ref().unwrap();
        assert!(approx_eq(sd.trailing_pe.as_ref().unwrap().raw.unwrap(), 28.5));
        assert!(approx_eq(sd.dividend_yield.as_ref().unwrap().raw.unwrap(), 0.006));
        assert!(approx_eq(sd.fifty_two_week_high.as_ref().unwrap().raw.unwrap(), 198.23));
        assert!(approx_eq(sd.fifty_two_week_low.as_ref().unwrap().raw.unwrap(), 124.17));
        assert!(approx_eq(sd.average_volume_10days.as_ref().unwrap().raw.unwrap(), 55_000_000.0));
        assert!(approx_eq(sd.market_cap.as_ref().unwrap().raw.unwrap(), 2_400_000_000_000.0));
    }

    #[test]
    fn test_fundamentals_json_null_result() {
        let json = r#"{"quoteSummary": {"result": null}}"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.quote_summary.unwrap().result.is_none());
    }

    #[test]
    fn test_fundamentals_json_null_quote_summary() {
        let json = r#"{"quoteSummary": null}"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.quote_summary.is_none());
    }

    #[test]
    fn test_fundamentals_json_missing_pe_gives_none() {
        let json = r#"{
            "quoteSummary": {
                "result": [{
                    "defaultKeyStatistics": {},
                    "summaryDetail": {
                        "dividendYield": {"raw": 0.01, "fmt": "1%"}
                    },
                    "price": {}
                }]
            }
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let results = parsed.quote_summary.unwrap().result.unwrap();
        let sd = results[0].summary_detail.as_ref().unwrap();
        assert!(sd.trailing_pe.is_none());
    }

    #[test]
    fn test_fundamentals_json_yahoo_value_raw_present() {
        let json = r#"{
            "quoteSummary": {"result": [{
                "defaultKeyStatistics": {"beta": {"raw": 1.35, "fmt": "1.35"}},
                "summaryDetail": {},
                "price": {}
            }]}
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let ks = parsed.quote_summary.unwrap().result.unwrap()
            .into_iter().next().unwrap()
            .default_key_statistics.unwrap();
        assert!(approx_eq(ks.beta.unwrap().raw.unwrap(), 1.35));
    }

    #[test]
    fn test_fundamentals_json_yahoo_value_null_raw() {
        // Yahoo sometimes returns {"raw": null, "fmt": "N/A"} for unavailable values
        let json = r#"{
            "quoteSummary": {"result": [{
                "defaultKeyStatistics": {"beta": {"raw": null, "fmt": "N/A"}},
                "summaryDetail": {},
                "price": {}
            }]}
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let ks = parsed.quote_summary.unwrap().result.unwrap()
            .into_iter().next().unwrap()
            .default_key_statistics.unwrap();
        assert!(ks.beta.unwrap().raw.is_none());
    }

    #[test]
    fn test_fundamentals_json_market_cap_in_price_module() {
        // Verify the price.marketCap field deserializes correctly as a fallback source
        let json = r#"{
            "quoteSummary": {"result": [{
                "defaultKeyStatistics": {},
                "summaryDetail": {},
                "price": {"marketCap": {"raw": 500000000000.0, "fmt": "500B"}}
            }]}
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let result = &parsed.quote_summary.unwrap().result.unwrap()[0];
        assert!(result.summary_detail.as_ref().unwrap().market_cap.is_none());
        let cap = result.price.as_ref().unwrap()
            .market_cap.as_ref().unwrap().raw.unwrap();
        assert!(approx_eq(cap, 500_000_000_000.0));
    }

    #[test]
    fn test_fundamentals_json_missing_all_optional_fields() {
        // Empty modules → all fields None, should not panic
        let json = r#"{
            "quoteSummary": {"result": [{
                "defaultKeyStatistics": {},
                "summaryDetail": {},
                "price": {}
            }]}
        }"#;
        let parsed: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        let result = &parsed.quote_summary.unwrap().result.unwrap()[0];
        let ks = result.default_key_statistics.as_ref().unwrap();
        assert!(ks.beta.is_none());
        assert!(ks.trailing_eps.is_none());
        assert!(ks.price_to_book.is_none());
        let sd = result.summary_detail.as_ref().unwrap();
        assert!(sd.trailing_pe.is_none());
        assert!(sd.dividend_yield.is_none());
        assert!(sd.market_cap.is_none());
    }

    // ---- Performance ----

    #[test]
    fn test_url_construction_performance_500_symbols() {
        use std::time::Instant;
        let symbols: Vec<String> = (0..500).map(|i| format!("S{i:03}")).collect();
        let start = Instant::now();
        for sym in &symbols {
            let _ = build_chart_url(sym);
            let _ = build_fundamentals_url(sym, "test-crumb");
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "URL construction for 500 symbols took {}ms, expected < 50ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_chart_json_parsing_performance_500_responses() {
        use std::time::Instant;
        let json = r#"{
            "chart": {"result": [{
                "meta": {
                    "regularMarketPrice": 150.25,
                    "chartPreviousClose": 147.50,
                    "regularMarketVolume": 75000000
                }
            }]}
        }"#;
        let start = Instant::now();
        for _ in 0..500 {
            let _: ChartResponse = serde_json::from_str(json).unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 500,
            "Parsing 500 chart responses took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_fundamentals_json_parsing_performance_500_responses() {
        use std::time::Instant;
        let json = r#"{
            "quoteSummary": {"result": [{
                "defaultKeyStatistics": {
                    "beta": {"raw": 1.2, "fmt": "1.20"},
                    "trailingEps": {"raw": 6.05, "fmt": "6.05"},
                    "priceToBook": {"raw": 8.5, "fmt": "8.50"}
                },
                "summaryDetail": {
                    "trailingPE": {"raw": 28.5, "fmt": "28.50"},
                    "dividendYield": {"raw": 0.006, "fmt": "0.60%"},
                    "fiftyTwoWeekHigh": {"raw": 198.23, "fmt": "198.23"},
                    "fiftyTwoWeekLow": {"raw": 124.17, "fmt": "124.17"},
                    "averageVolume10days": {"raw": 55000000.0, "fmt": "55M"},
                    "marketCap": {"raw": 2400000000000.0, "fmt": "2.4T"}
                },
                "price": {"marketCap": {"raw": 2400000000000.0, "fmt": "2.4T"}}
            }]}
        }"#;
        let start = Instant::now();
        for _ in 0..500 {
            let _: QuoteSummaryResponse = serde_json::from_str(json).unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 500,
            "Parsing 500 fundamentals responses took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }
}
