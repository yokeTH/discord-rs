//! Market data: real-time snapshot, order-book quotes, and historical bars.

use serde::Deserialize;

#[cfg(feature = "http")]
use crate::client::WebullClient;
#[cfg(feature = "http")]
use crate::types::{Category, Timespan};
#[cfg(feature = "http")]
use anyhow::Result;
#[cfg(feature = "http")]
use tracing::instrument;

/// Real-time snapshot for a single symbol.
///
/// Prices are strings (see [`crate::types`]). Extended-hours (`extend_hour_*`)
/// and overnight (`ovn_*`) fields are only populated when requested.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub instrument_id: String,
    pub symbol: String,
    pub price: String,
    pub pre_close: String,
    pub change: String,
    pub change_ratio: String,
    pub open: String,
    pub close: String,
    pub high: String,
    pub low: String,
    pub volume: String,
    pub last_trade_time: Option<i64>,
    pub ask: Option<String>,
    pub ask_size: Option<String>,
    pub bid: Option<String>,
    pub bid_size: Option<String>,
    pub extend_hour_last_price: Option<String>,
    pub extend_hour_high: Option<String>,
    pub extend_hour_low: Option<String>,
    pub extend_hour_change: Option<String>,
    pub extend_hour_change_ratio: Option<String>,
    pub extend_hour_volume: Option<String>,
    pub extend_hour_last_trade_time: Option<i64>,
    pub ovn_price: Option<String>,
    pub ovn_high: Option<String>,
    pub ovn_low: Option<String>,
    pub ovn_volume: Option<String>,
    pub ovn_change: Option<String>,
    pub ovn_change_ratio: Option<String>,
    pub ovn_last_trade_time: Option<i64>,
}

/// A single order-book price level.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct QuoteLevel {
    pub price: String,
    pub size: String,
    pub order: Vec<QuoteOrder>,
    pub broker: Vec<QuoteBroker>,
}

/// An individual order within an order-book level.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteOrder {
    pub mpid: String,
    pub size: String,
}

/// Broker attribution within an order-book level.
#[derive(Debug, Clone, Deserialize)]
pub struct QuoteBroker {
    pub bid: String,
    pub name: String,
}

/// Order-book quote (bids and asks at the requested depth).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Quote {
    pub symbol: String,
    pub instrument_id: String,
    pub quote_time: String,
    pub asks: Vec<QuoteLevel>,
    pub bids: Vec<QuoteLevel>,
}

/// A single OHLCV candlestick.
#[derive(Debug, Clone, Deserialize)]
pub struct Bar {
    // Note: the API returns this key as camelCase while the rest are snake_case.
    #[serde(rename = "tickerId")]
    pub ticker_id: String,
    pub symbol: String,
    /// Bar time, ISO-8601 with offset (e.g. `2021-12-28T09:00:09.945+0000`).
    pub time: String,
    pub open: String,
    pub close: String,
    pub high: String,
    pub low: String,
    #[serde(default)]
    pub volume: Option<String>,
    #[serde(default)]
    pub trading_session: Option<String>,
}

#[cfg(feature = "http")]
impl WebullClient {
    /// Real-time snapshots for up to 100 symbols.
    #[instrument(name = "webull_snapshot", skip(self), fields(symbols = symbols.len()))]
    pub async fn snapshot(
        &self,
        symbols: &[&str],
        category: Category,
        extend_hour: bool,
        overnight: bool,
    ) -> Result<Vec<Snapshot>> {
        let query = vec![
            ("symbols".to_string(), symbols.join(",")),
            ("category".to_string(), category.as_str().to_string()),
            ("extend_hour_required".to_string(), extend_hour.to_string()),
            ("overnight_required".to_string(), overnight.to_string()),
        ];
        self.get("/openapi/market-data/stock/snapshot", &query, true)
            .await
    }

    /// Order-book quote for a symbol. `depth` is the number of price levels
    /// (e.g. 1 for L1, 10 for L2).
    #[instrument(name = "webull_quotes", skip(self), fields(%symbol, depth))]
    pub async fn quotes(
        &self,
        symbol: &str,
        category: Category,
        depth: u32,
        overnight: bool,
    ) -> Result<Quote> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            ("depth".to_string(), depth.to_string()),
            ("overnight_required".to_string(), overnight.to_string()),
        ];
        self.get("/openapi/market-data/stock/quotes", &query, true)
            .await
    }

    /// Historical OHLCV bars for a symbol.
    ///
    /// `count` is the number of bars (1–1200; up to 1650 for `M1`).
    /// `real_time_required = true` returns completed bars only.
    #[instrument(name = "webull_bars", skip(self), fields(%symbol, timespan = timespan.as_str(), count))]
    pub async fn bars(
        &self,
        symbol: &str,
        category: Category,
        timespan: Timespan,
        count: u32,
        real_time_required: bool,
    ) -> Result<Vec<Bar>> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            ("timespan".to_string(), timespan.as_str().to_string()),
            ("count".to_string(), count.to_string()),
            (
                "real_time_required".to_string(),
                real_time_required.to_string(),
            ),
        ];
        self.get("/openapi/market-data/stock/bars", &query, true)
            .await
    }

    /// Tick-by-tick trades for a symbol.
    #[instrument(name = "webull_tick", skip(self), fields(%symbol))]
    pub async fn tick(
        &self,
        symbol: &str,
        category: Category,
        count: u32,
        trading_sessions: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            ("count".to_string(), count.to_string()),
        ];
        if let Some(ts) = trading_sessions {
            query.push(("trading_sessions".to_string(), ts.to_string()));
        }
        self.get("/openapi/market-data/stock/tick", &query, true)
            .await
    }

    /// Order-flow / volume-profile footprint for up to 100 symbols.
    #[instrument(name = "webull_footprint", skip(self))]
    pub async fn footprint(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Timespan,
        count: Option<u32>,
        real_time_required: Option<bool>,
        trading_sessions: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut query = vec![
            ("symbols".to_string(), symbols.join(",")),
            ("category".to_string(), category.as_str().to_string()),
            ("timespan".to_string(), timespan.as_str().to_string()),
        ];
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        if let Some(r) = real_time_required {
            query.push(("real_time_required".to_string(), r.to_string()));
        }
        if let Some(ts) = trading_sessions {
            query.push(("trading_sessions".to_string(), ts.to_string()));
        }
        self.get("/openapi/market-data/stock/footprint", &query, true)
            .await
    }

    /// Batch historical bars for multiple symbols.
    #[instrument(name = "webull_batch_bars", skip(self), fields(symbols = symbols.len()))]
    pub async fn batch_bars(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Timespan,
        count: u32,
        real_time_required: bool,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "symbols": symbols.join(","),
            "category": category.as_str(),
            "timespan": timespan.as_str(),
            "count": count.to_string(),
            "real_time_required": real_time_required,
        });
        self.post("/openapi/market-data/stock/batch-bars", &body, true)
            .await
    }

    /// NOII (net order imbalance indicator) bars. `imbalance_action_type` is
    /// `PRE_OPEN` or `PRE_CLOSE`.
    #[instrument(name = "webull_noii_bars", skip(self), fields(%symbol))]
    pub async fn noii_bars(
        &self,
        symbol: &str,
        category: Category,
        imbalance_action_type: &str,
    ) -> Result<serde_json::Value> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            (
                "imbalance_action_type".to_string(),
                imbalance_action_type.to_string(),
            ),
        ];
        self.get("/openapi/market-data/stock/noii/bars", &query, true)
            .await
    }

    /// NOII snapshot. `imbalance_action_type` is `PRE_OPEN` or `PRE_CLOSE`.
    #[instrument(name = "webull_noii_snapshot", skip(self), fields(%symbol))]
    pub async fn noii_snapshot(
        &self,
        symbol: &str,
        category: Category,
        imbalance_action_type: &str,
    ) -> Result<serde_json::Value> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            (
                "imbalance_action_type".to_string(),
                imbalance_action_type.to_string(),
            ),
        ];
        self.get("/openapi/market-data/stock/noii/snapshot", &query, true)
            .await
    }

    /// End-of-day bars by instrument id (note: this path omits `/openapi`).
    #[instrument(name = "webull_eod_bars", skip(self))]
    pub async fn eod_bars(
        &self,
        instrument_ids: &[&str],
        date: &str,
        count: Option<u32>,
    ) -> Result<serde_json::Value> {
        let mut query = vec![
            ("instrument_ids".to_string(), instrument_ids.join(",")),
            ("date".to_string(), date.to_string()),
        ];
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        self.get("/market-data/eod-bars", &query, true).await
    }
}
