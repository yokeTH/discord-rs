//! Fundamentals: stock calendars/flows, financial statements, and fund data.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::Category;

impl WebullClient {
    // --- stock fundamentals ---

    /// Dividend calendar for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn dividend_calendar(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category(
            "/openapi/fundamentals/stock/dividend-calendar",
            symbol,
            category,
        )
        .await
    }

    /// Earnings calendar for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn earnings_calendar(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category(
            "/openapi/fundamentals/stock/earnings-calendar",
            symbol,
            category,
        )
        .await
    }

    /// SEC filings for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn sec_filings(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/stock/filings", symbol, category)
            .await
    }

    /// Analyst EPS forecast for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn forecast_eps(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/stock/forecast-eps", symbol, category)
            .await
    }

    /// Capital flow for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn capital_flow(
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
        self.get("/openapi/fundamentals/stock/capital-flow", &query, true)
            .await
    }

    /// Industry comparison for a symbol.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn industry_comparison(
        &self,
        symbol: &str,
        category: Category,
        sort_by: Option<&str>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(s) = sort_by {
            query.push(("sort_by".to_string(), s.to_string()));
        }
        self.get(
            "/openapi/fundamentals/stock/industry-comparison",
            &query,
            true,
        )
        .await
    }

    // --- financial statements ---

    async fn financials(
        &self,
        path: &str,
        symbol: &str,
        category: Category,
        statement_type: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(t) = statement_type {
            query.push(("type".to_string(), t.to_string()));
        }
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        self.get(path, &query, true).await
    }

    /// Balance sheet.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn financials_balance_sheet(
        &self,
        symbol: &str,
        category: Category,
        statement_type: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        self.financials(
            "/openapi/fundamentals/financial/balance-sheet",
            symbol,
            category,
            statement_type,
            count,
        )
        .await
    }

    /// Cash-flow statement.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn financials_cash_flow(
        &self,
        symbol: &str,
        category: Category,
        statement_type: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        self.financials(
            "/openapi/fundamentals/financial/cash-flow",
            symbol,
            category,
            statement_type,
            count,
        )
        .await
    }

    /// Income statement.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn financials_income(
        &self,
        symbol: &str,
        category: Category,
        statement_type: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        self.financials(
            "/openapi/fundamentals/financial/income",
            symbol,
            category,
            statement_type,
            count,
        )
        .await
    }

    /// Financial indicators.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn financials_indicators(
        &self,
        symbol: &str,
        category: Category,
        statement_type: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        self.financials(
            "/openapi/fundamentals/financial/indicators",
            symbol,
            category,
            statement_type,
            count,
        )
        .await
    }

    /// Financial alerts.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn financials_alert(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/financial/alert", symbol, category)
            .await
    }

    // --- fund data ---

    /// Fund asset allocation.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_allocation(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/allocation", symbol, category)
            .await
    }

    /// Fund brief / overview.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_brief(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/brief", symbol, category)
            .await
    }

    /// Fund filings / documents.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_files(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/files", symbol, category)
            .await
    }

    /// Fund holdings.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_holdings(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/holdings", symbol, category)
            .await
    }

    /// Fund performance.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_performance(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/performance", symbol, category)
            .await
    }

    /// Fund rating.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_rating(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/rating", symbol, category)
            .await
    }

    /// Fund splits.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_splits(&self, symbol: &str, category: Category) -> Result<Value> {
        self.get_symbol_category("/openapi/fundamentals/fund/splits", symbol, category)
            .await
    }

    /// Fund dividends (paginated).
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_dividends(
        &self,
        symbol: &str,
        category: Category,
        page_index: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(i) = page_index {
            query.push(("page_index".to_string(), i.to_string()));
        }
        if let Some(s) = page_size {
            query.push(("page_size".to_string(), s.to_string()));
        }
        self.get("/openapi/fundamentals/fund/dividends", &query, true)
            .await
    }

    /// Fund net value / NAV history.
    #[instrument(skip(self), fields(%symbol))]
    pub async fn fund_net_value(
        &self,
        symbol: &str,
        category: Category,
        last_date: Option<&str>,
        count: Option<u32>,
    ) -> Result<Value> {
        let mut query = vec![
            ("symbol".to_string(), symbol.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(d) = last_date {
            query.push(("last_date".to_string(), d.to_string()));
        }
        if let Some(c) = count {
            query.push(("count".to_string(), c.to_string()));
        }
        self.get("/openapi/fundamentals/fund/net-value", &query, true)
            .await
    }
}
