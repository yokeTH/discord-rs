//! Crypto market data and instruments.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::{Category, Timespan};

impl WebullClient {
    /// Crypto historical bars.
    #[instrument(skip(self))]
    pub async fn crypto_bars(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Timespan,
        count: Option<u32>,
        real_time_required: Option<bool>,
    ) -> Result<Value> {
        self.get_bars(
            "/openapi/market-data/crypto/bars",
            symbols,
            category,
            timespan,
            count,
            real_time_required,
        )
        .await
    }

    /// Crypto snapshot.
    #[instrument(skip(self))]
    pub async fn crypto_snapshot(&self, symbols: &[&str], category: Category) -> Result<Value> {
        self.get_symbols_category("/openapi/market-data/crypto/snapshot", symbols, category)
            .await
    }

    /// Crypto instruments list.
    #[instrument(skip(self))]
    pub async fn crypto_instruments(
        &self,
        symbols: &[&str],
        category: Category,
        status: Option<&str>,
        last_instrument_id: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbols".to_string(), symbols.join(",")),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(s) = status {
            query.push(("status".to_string(), s.to_string()));
        }
        if let Some(id) = last_instrument_id {
            query.push(("last_instrument_id".to_string(), id.to_string()));
        }
        if let Some(p) = page_size {
            query.push(("page_size".to_string(), p.to_string()));
        }
        self.get("/openapi/instrument/crypto/list", &query, true)
            .await
    }
}
