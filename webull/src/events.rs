//! Prediction-market ("event") market data and instruments.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::{Category, Timespan};

impl WebullClient {
    /// Event historical bars.
    #[instrument(skip(self))]
    pub async fn event_bars(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Option<Timespan>,
        count: Option<u32>,
        real_time_required: Option<bool>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbols".to_string(), symbols.join(",")),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(t) = timespan {
            query.push(("timespan".to_string(), t.as_str().to_string()));
        }
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        if let Some(r) = real_time_required {
            query.push(("real_time_required".to_string(), r.to_string()));
        }
        self.get("/openapi/market-data/event/bars", &query, true)
            .await
    }

    /// Event order-book depth.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn event_depth(
        &self,
        symbol: &str,
        category: Category,
        depth: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(d) = depth {
            query.push(("depth".to_string(), d.to_string()));
        }
        self.get("/openapi/market-data/event/depth", &query, true)
            .await
    }

    /// Event snapshot.
    #[instrument(skip(self))]
    pub async fn event_snapshot(&self, symbols: &[&str], category: Category) -> Result<Value> {
        self.get_symbols_category("/openapi/market-data/event/snapshot", symbols, category)
            .await
    }

    /// Event tick.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn event_tick(
        &self,
        symbol: &str,
        category: Category,
        count: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        self.get("/openapi/market-data/event/tick", &query, true)
            .await
    }

    /// Events under a series.
    #[instrument(skip(self))]
    pub async fn event_events(
        &self,
        series_symbol: Option<&str>,
        symbols: &[&str],
        status: Option<&str>,
    ) -> Result<Value> {
        let mut query = Vec::new();
        if let Some(s) = series_symbol {
            query.push(("series_symbol".to_string(), s.to_string()));
        }
        if !symbols.is_empty() {
            query.push(("symbols".to_string(), symbols.join(",")));
        }
        if let Some(s) = status {
            query.push(("status".to_string(), s.to_string()));
        }
        self.get("/openapi/instrument/event/events", &query, true)
            .await
    }

    /// Event markets list.
    #[instrument(skip(self))]
    pub async fn event_market_list(
        &self,
        series_symbol: Option<&str>,
        event_symbol: Option<&str>,
        symbols: &[&str],
        expiration_date_after: Option<&str>,
        last_instrument_id: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = Vec::new();
        if let Some(s) = series_symbol {
            query.push(("series_symbol".to_string(), s.to_string()));
        }
        if let Some(s) = event_symbol {
            query.push(("event_symbol".to_string(), s.to_string()));
        }
        if !symbols.is_empty() {
            query.push(("symbols".to_string(), symbols.join(",")));
        }
        if let Some(d) = expiration_date_after {
            query.push(("expiration_date_after".to_string(), d.to_string()));
        }
        if let Some(id) = last_instrument_id {
            query.push(("last_instrument_id".to_string(), id.to_string()));
        }
        if let Some(p) = page_size {
            query.push(("page_size".to_string(), p.to_string()));
        }
        self.get("/openapi/instrument/event/market/list", &query, true)
            .await
    }

    /// Event series list.
    #[instrument(skip(self))]
    pub async fn event_series(
        &self,
        category: Category,
        symbols: &[&str],
        last_series_id: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if !symbols.is_empty() {
            query.push(("symbols".to_string(), symbols.join(",")));
        }
        if let Some(id) = last_series_id {
            query.push(("last_series_id".to_string(), id.to_string()));
        }
        if let Some(p) = page_size {
            query.push(("page_size".to_string(), p.to_string()));
        }
        self.get("/openapi/instrument/event/series/list", &query, true)
            .await
    }

    /// Event series categories.
    #[instrument(skip(self))]
    pub async fn event_categories(&self) -> Result<Value> {
        self.get("/openapi/instrument/event/categories", &[], true)
            .await
    }
}
