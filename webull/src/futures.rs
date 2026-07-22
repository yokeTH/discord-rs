//! Futures market data and instruments.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::{Category, ContractType, Timespan};

impl WebullClient {
    /// Futures order-book depth.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn futures_depth(
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
        self.get("/openapi/market-data/futures/depth", &query, true)
            .await
    }

    /// Futures order-flow footprint.
    #[instrument(skip(self))]
    pub async fn futures_footprint(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Option<Timespan>,
        count: Option<u32>,
        real_time_required: Option<bool>,
        trading_sessions: Option<&str>,
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
        if let Some(ts) = trading_sessions {
            query.push(("trading_sessions".to_string(), ts.to_string()));
        }
        self.get("/openapi/market-data/futures/footprint", &query, true)
            .await
    }

    /// Futures historical bars.
    #[instrument(skip(self))]
    pub async fn futures_bars(
        &self,
        symbols: &[&str],
        category: Category,
        timespan: Timespan,
        count: Option<u32>,
        real_time_required: Option<bool>,
    ) -> Result<Value> {
        self.get_bars(
            "/openapi/market-data/futures/bars",
            symbols,
            category,
            timespan,
            count,
            real_time_required,
        )
        .await
    }

    /// Futures snapshot.
    #[instrument(skip(self))]
    pub async fn futures_snapshot(&self, symbols: &[&str], category: Category) -> Result<Value> {
        self.get_symbols_category("/openapi/market-data/futures/snapshot", symbols, category)
            .await
    }

    /// Futures tick.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn futures_tick(
        &self,
        symbol: &str,
        category: Category,
        count: u32,
    ) -> Result<Value> {
        let query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            ("count".to_string(), count.to_string()),
        ];
        self.get("/openapi/market-data/futures/tick", &query, true)
            .await
    }

    /// Futures instrument by contract code.
    #[instrument(skip(self))]
    pub async fn futures_instruments_by_code(
        &self,
        code: &str,
        category: Category,
        contract_type: ContractType,
    ) -> Result<Value> {
        let query = vec![
            ("code".to_string(), code.to_string()),
            ("category".to_string(), category.as_str().to_string()),
            (
                "contract_type".to_string(),
                contract_type.as_str().to_string(),
            ),
        ];
        self.get("/openapi/instrument/futures/by-code", &query, true)
            .await
    }

    /// Futures instruments list.
    #[instrument(skip(self))]
    pub async fn futures_instruments(
        &self,
        symbols: &[&str],
        category: Category,
        code: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbols".to_string(), symbols.join(",")),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(c) = code {
            query.push(("code".to_string(), c.to_string()));
        }
        if let Some(s) = status {
            query.push(("status".to_string(), s.to_string()));
        }
        self.get("/openapi/instrument/futures/list", &query, true)
            .await
    }

    /// Futures product classes.
    #[instrument(skip(self))]
    pub async fn futures_product_classes(&self, category: Category) -> Result<Value> {
        let query = vec![("category".to_string(), category.as_str().to_string())];
        self.get("/openapi/instrument/futures/product-classes", &query, true)
            .await
    }

    /// Futures products in a class.
    #[instrument(skip(self))]
    pub async fn futures_products(
        &self,
        category: Category,
        product_class_id: Option<&str>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if let Some(p) = product_class_id {
            query.push(("product_class_id".to_string(), p.to_string()));
        }
        self.get("/openapi/instrument/futures/products", &query, true)
            .await
    }
}
