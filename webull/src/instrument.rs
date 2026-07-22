//! Instrument reference data (tradability, margin, lot size).

use anyhow::Result;
use serde::Deserialize;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::Category;

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
}
