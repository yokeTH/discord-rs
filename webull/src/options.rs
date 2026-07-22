//! Options market data and contracts.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::{Category, OptionType, Timespan};

/// Query filters for [`WebullClient::option_contracts`]. Build with `Default`
/// and set only the fields you need.
#[derive(Debug, Clone, Default)]
pub struct OptionContractsParams<'a> {
    pub category: Option<Category>,
    pub underlying_symbols: Option<&'a str>,
    pub status: Option<&'a str>,
    pub start_date: Option<&'a str>,
    pub end_date: Option<&'a str>,
    pub root_symbol: Option<&'a str>,
    pub option_symbol: Option<&'a str>,
    pub option_type: Option<OptionType>,
    pub style: Option<&'a str>,
    pub strike_price_gte: Option<&'a str>,
    pub strike_price_lte: Option<&'a str>,
    pub ppind: Option<&'a str>,
    pub show_deliverables: Option<bool>,
    pub page_size: Option<u32>,
    pub last_instrument_id: Option<&'a str>,
}

impl WebullClient {
    /// Option historical bars (max 20 symbols).
    #[instrument(skip(self))]
    pub async fn option_bars(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Timespan,
        count: Option<u32>,
        real_time_required: Option<bool>,
    ) -> Result<Value> {
        self.get_bars(
            "/openapi/market-data/option/bars",
            symbols,
            category,
            timespan,
            count,
            real_time_required,
        )
        .await
    }

    /// Option snapshot (max 20 symbols).
    #[instrument(skip(self))]
    pub async fn option_snapshot(&self, symbols: &[&str], category: Category) -> Result<Value> {
        self.get_symbols_category("/openapi/market-data/option/snapshot", symbols, category)
            .await
    }

    /// Option tick.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn option_tick(&self, symbol: &str, category: Category, count: u32) -> Result<Value> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            ("count".to_string(), count.to_string()),
        ];
        self.get("/openapi/market-data/option/tick", &query, true)
            .await
    }

    /// Option contracts matching the given filters.
    #[instrument(skip_all)]
    pub async fn option_contracts(&self, params: &OptionContractsParams<'_>) -> Result<Value> {
        let mut query: Vec<(String, String)> = Vec::new();
        if let Some(v) = params.category {
            query.push(("category".to_string(), v.as_str().to_string()));
        }
        if let Some(v) = params.underlying_symbols {
            query.push(("underlying_symbols".to_string(), v.to_string()));
        }
        if let Some(v) = params.status {
            query.push(("status".to_string(), v.to_string()));
        }
        if let Some(v) = params.start_date {
            query.push(("start_date".to_string(), v.to_string()));
        }
        if let Some(v) = params.end_date {
            query.push(("end_date".to_string(), v.to_string()));
        }
        if let Some(v) = params.root_symbol {
            query.push(("root_symbol".to_string(), v.to_string()));
        }
        if let Some(v) = params.option_symbol {
            query.push(("option_symbol".to_string(), v.to_string()));
        }
        if let Some(v) = params.option_type {
            query.push(("option_type".to_string(), v.as_str().to_string()));
        }
        if let Some(v) = params.style {
            query.push(("style".to_string(), v.to_string()));
        }
        if let Some(v) = params.strike_price_gte {
            query.push(("strike_price_gte".to_string(), v.to_string()));
        }
        if let Some(v) = params.strike_price_lte {
            query.push(("strike_price_lte".to_string(), v.to_string()));
        }
        if let Some(v) = params.ppind {
            query.push(("ppind".to_string(), v.to_string()));
        }
        if let Some(v) = params.show_deliverables {
            query.push(("show_deliverables".to_string(), v.to_string()));
        }
        if let Some(v) = params.page_size {
            query.push(("page_size".to_string(), v.to_string()));
        }
        if let Some(v) = params.last_instrument_id {
            query.push(("last_instrument_id".to_string(), v.to_string()));
        }
        self.get("/openapi/instrument/option/contracts", &query, true)
            .await
    }
}
