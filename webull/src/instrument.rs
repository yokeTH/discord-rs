//! Instrument reference data (tradability, margin, lot size).

use serde::Deserialize;

#[cfg(feature = "http")]
use crate::client::WebullClient;
#[cfg(feature = "http")]
use crate::types::Category;
#[cfg(feature = "http")]
use anyhow::Result;
#[cfg(feature = "http")]
use tracing::instrument;

/// Reference data for a tradable instrument.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Instrument {
    pub instrument_id: String,
    pub symbol: String,
    pub name: String,
    pub exchange_code: String,
    pub category: String,
    pub currency: String,
    /// `OC` (tradable), `CO` (liquidate only), or `NT` (non-tradable).
    pub status: String,
    pub shortable: Option<bool>,
    pub fractionable: Option<bool>,
    pub marginable: Option<bool>,
    pub overnight_trading_supported: Option<bool>,
    pub easy_to_borrow: Option<bool>,
    pub lot_size: Option<String>,
    pub margin_requirement_long: Option<String>,
    pub margin_requirement_short: Option<String>,
}

#[cfg(feature = "http")]
impl WebullClient {
    /// Look up instruments by category, optionally filtered to `symbols`
    /// (comma-joined, max 100). Pass an empty slice to list the category.
    #[instrument(name = "webull_instruments", skip(self), fields(symbols = symbols.len()))]
    pub async fn instruments(
        &self,
        category: Category,
        symbols: &[&str],
    ) -> Result<Vec<Instrument>> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if !symbols.is_empty() {
            query.push(("symbols".to_string(), symbols.join(",")));
        }
        self.get("/openapi/instrument/stock/list", &query, true)
            .await
    }

    /// Corporate actions for instruments (note: this path omits `/openapi`).
    #[instrument(name = "webull_corp_action", skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn corp_action(
        &self,
        instrument_ids: &[&str],
        event_types: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        page_number: Option<u32>,
        page_size: Option<u32>,
        last_update_time: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut query = vec![("instrument_ids".to_string(), instrument_ids.join(","))];
        if let Some(v) = event_types {
            query.push(("event_types".to_string(), v.to_string()));
        }
        if let Some(v) = start_date {
            query.push(("start_date".to_string(), v.to_string()));
        }
        if let Some(v) = end_date {
            query.push(("end_date".to_string(), v.to_string()));
        }
        if let Some(v) = page_number {
            query.push(("page_number".to_string(), v.to_string()));
        }
        if let Some(v) = page_size {
            query.push(("page_size".to_string(), v.to_string()));
        }
        if let Some(v) = last_update_time {
            query.push(("last_update_time".to_string(), v.to_string()));
        }
        self.get("/instrument/corp-action", &query, true).await
    }

    /// Company profile.
    #[instrument(name = "webull_company_profile", skip(self), fields(%symbol))]
    pub async fn company_profile(
        &self,
        symbol: &str,
        category: Category,
    ) -> Result<serde_json::Value> {
        self.get_symbol_category("/openapi/instrument/company/profile", symbol, category)
            .await
    }

    /// Analyst ratings.
    #[instrument(name = "webull_analyst_rating", skip(self), fields(%symbol))]
    pub async fn analyst_rating(
        &self,
        symbol: &str,
        category: Category,
    ) -> Result<serde_json::Value> {
        self.get_symbol_category("/openapi/instrument/analyst/rating", symbol, category)
            .await
    }

    /// Analyst target prices.
    #[instrument(name = "webull_analyst_target_price", skip(self), fields(%symbol))]
    pub async fn analyst_target_price(
        &self,
        symbol: &str,
        category: Category,
    ) -> Result<serde_json::Value> {
        self.get_symbol_category("/openapi/instrument/analyst/target-price", symbol, category)
            .await
    }
}
