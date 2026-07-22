//! Miscellaneous trade endpoints: cash activities, the trading calendar, and
//! instrument / security lookups.
//!
//! These have no documented response schema, so they return `serde_json::Value`
//! (matching the SDK, which returns untyped dicts).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::Market;

impl WebullClient {
    /// Cash-account activities (deposits, dividends, fees, …).
    #[instrument(name = "webull_activities", skip(self))]
    pub async fn activities(
        &self,
        account_id: &str,
        activity_types: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
        last_activity_id: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![("account_id".to_string(), account_id.to_string())];
        if let Some(v) = activity_types {
            query.push(("activity_types".to_string(), v.to_string()));
        }
        if let Some(v) = start_time {
            query.push(("start_time".to_string(), v.to_string()));
        }
        if let Some(v) = end_time {
            query.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = last_activity_id {
            query.push(("last_activity_id".to_string(), v.to_string()));
        }
        if let Some(v) = page_size {
            query.push(("page_size".to_string(), v.to_string()));
        }
        self.get("/openapi/trade/activities/cash", &query, true)
            .await
    }

    /// Trading calendar for a market between `start` and `end` (`yyyy-MM-dd`).
    #[instrument(name = "webull_trade_calendar", skip(self))]
    pub async fn trade_calendar(&self, market: Market, start: &str, end: &str) -> Result<Value> {
        let query = vec![
            ("market".to_string(), market.as_str().to_string()),
            ("start".to_string(), start.to_string()),
            ("end".to_string(), end.to_string()),
        ];
        self.get("/trade/calendar", &query, true).await
    }

    /// Tradable-instrument detail by instrument id.
    #[instrument(name = "webull_trade_instrument", skip(self))]
    pub async fn trade_instrument_detail(&self, instrument_id: &str) -> Result<Value> {
        let query = vec![("instrument_id".to_string(), instrument_id.to_string())];
        self.get("/trade/instrument", &query, true).await
    }

    /// Security detail for an account + symbol.
    #[instrument(name = "webull_trade_security", skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn trade_security_detail(
        &self,
        account_id: &str,
        symbol: &str,
        market: Market,
        instrument_super_type: Option<&str>,
        instrument_type: Option<&str>,
        strike_price: Option<&str>,
        init_exp_date: Option<&str>,
    ) -> Result<Value> {
        let mut query = vec![
            ("account_id".to_string(), account_id.to_string()),
            ("symbol".to_string(), symbol.to_string()),
            ("market".to_string(), market.as_str().to_string()),
        ];
        if let Some(v) = instrument_super_type {
            query.push(("instrument_super_type".to_string(), v.to_string()));
        }
        if let Some(v) = instrument_type {
            query.push(("instrument_type".to_string(), v.to_string()));
        }
        if let Some(v) = strike_price {
            query.push(("strike_price".to_string(), v.to_string()));
        }
        if let Some(v) = init_exp_date {
            query.push(("init_exp_date".to_string(), v.to_string()));
        }
        self.get("/trade/security", &query, true).await
    }

    /// List tradable instruments (paginated).
    #[instrument(name = "webull_tradeable_instruments", skip(self))]
    pub async fn tradeable_instruments(
        &self,
        last_security_id: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = Vec::new();
        if let Some(v) = last_security_id {
            query.push(("last_security_id".to_string(), v.to_string()));
        }
        if let Some(v) = page_size {
            query.push(("page_size".to_string(), v.to_string()));
        }
        self.get("/trade/instrument/tradable/list", &query, true)
            .await
    }
}
