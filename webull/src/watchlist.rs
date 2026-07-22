//! Watchlist management.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::{Value, json};
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::Category;

impl WebullClient {
    /// List the user's watchlists.
    #[instrument(skip(self))]
    pub async fn get_watchlists(&self) -> Result<Value> {
        self.get("/openapi/market-data/watchlist/list", &[], true)
            .await
    }

    /// Create a watchlist.
    #[instrument(skip(self))]
    pub async fn create_watchlist(&self, name: &str, sort: Option<i64>) -> Result<Value> {
        let mut body = json!({ "name": name });
        if let Some(s) = sort {
            body["sort"] = json!(s);
        }
        self.post("/openapi/market-data/watchlist/create", &body, true)
            .await
    }

    /// Rename / reorder a watchlist.
    #[instrument(skip(self))]
    pub async fn update_watchlist(
        &self,
        watchlist_id: &str,
        name: Option<&str>,
        sort: Option<i64>,
    ) -> Result<Value> {
        let mut body = json!({ "watchlist_id": watchlist_id });
        if let Some(n) = name {
            body["name"] = json!(n);
        }
        if let Some(s) = sort {
            body["sort"] = json!(s);
        }
        self.post("/openapi/market-data/watchlist/update", &body, true)
            .await
    }

    /// Delete a watchlist.
    #[instrument(skip(self))]
    pub async fn delete_watchlist(&self, watchlist_id: &str) -> Result<Value> {
        let body = json!({ "watchlist_id": watchlist_id });
        self.post("/openapi/market-data/watchlist/delete", &body, true)
            .await
    }

    /// List the instruments in a watchlist.
    #[instrument(skip(self))]
    pub async fn get_watchlist_instruments(&self, watchlist_id: &str) -> Result<Value> {
        let query = vec![("watchlist_id".to_string(), watchlist_id.to_string())];
        self.get(
            "/openapi/market-data/watchlist/instruments/list",
            &query,
            true,
        )
        .await
    }

    /// Add `(symbol, category)` instruments to a watchlist.
    #[instrument(skip(self, instruments))]
    pub async fn add_watchlist_instruments(
        &self,
        watchlist_id: &str,
        instruments: &[(&str, Category)],
    ) -> Result<Value> {
        self.watchlist_instruments(
            "/openapi/market-data/watchlist/instruments/add",
            watchlist_id,
            instruments,
        )
        .await
    }

    /// Remove `(symbol, category)` instruments from a watchlist.
    #[instrument(skip(self, instruments))]
    pub async fn remove_watchlist_instruments(
        &self,
        watchlist_id: &str,
        instruments: &[(&str, Category)],
    ) -> Result<Value> {
        self.watchlist_instruments(
            "/openapi/market-data/watchlist/instruments/remove",
            watchlist_id,
            instruments,
        )
        .await
    }

    /// Replace the `(symbol, category)` instruments in a watchlist.
    #[instrument(skip(self, instruments))]
    pub async fn update_watchlist_instruments(
        &self,
        watchlist_id: &str,
        instruments: &[(&str, Category)],
    ) -> Result<Value> {
        self.watchlist_instruments(
            "/openapi/market-data/watchlist/instruments/update",
            watchlist_id,
            instruments,
        )
        .await
    }

    async fn watchlist_instruments(
        &self,
        path: &str,
        watchlist_id: &str,
        instruments: &[(&str, Category)],
    ) -> Result<Value> {
        let items: Vec<Value> = instruments
            .iter()
            .map(|(symbol, category)| json!({ "symbol": symbol, "category": category.as_str() }))
            .collect();
        let body = json!({ "watchlist_id": watchlist_id, "instruments": items });
        self.post(path, &body, true).await
    }
}
