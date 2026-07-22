//! Market screeners.
//!
//! Responses are `serde_json::Value` (the SDK does not type them).

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::{Category, Direction};

/// Append the paging/sort params shared by most screeners.
fn push_paging(
    query: &mut Vec<(String, String)>,
    sort_by: Option<&str>,
    page_index: Option<u32>,
    page_size: Option<u32>,
    direction: Option<Direction>,
) {
    if let Some(s) = sort_by {
        query.push(("sort_by".to_string(), s.to_string()));
    }
    if let Some(i) = page_index {
        query.push(("page_index".to_string(), i.to_string()));
    }
    if let Some(p) = page_size {
        query.push(("page_size".to_string(), p.to_string()));
    }
    if let Some(d) = direction {
        query.push(("direction".to_string(), d.as_str().to_string()));
    }
}

impl WebullClient {
    /// 52-week high/low screener.
    #[instrument(skip(self))]
    pub async fn screener_52whl(
        &self,
        category: Category,
        rank_type: Option<&str>,
        sort_by: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if let Some(r) = rank_type {
            query.push(("rank_type".to_string(), r.to_string()));
        }
        push_paging(&mut query, sort_by, page_index, page_size, direction);
        self.get("/openapi/market-data/screener/52whl", &query, true)
            .await
    }

    /// Gainers / losers screener.
    #[instrument(skip(self))]
    pub async fn gainers_losers(
        &self,
        category: Category,
        rank_type: Option<&str>,
        sort_by: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if let Some(r) = rank_type {
            query.push(("rank_type".to_string(), r.to_string()));
        }
        push_paging(&mut query, sort_by, page_index, page_size, direction);
        self.get("/openapi/market-data/screener/gainers-losers", &query, true)
            .await
    }

    /// Most-active screener (path segment is `top-active`).
    #[instrument(skip(self))]
    pub async fn most_active(
        &self,
        category: Category,
        rank_type: Option<&str>,
        sort_by: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if let Some(r) = rank_type {
            query.push(("rank_type".to_string(), r.to_string()));
        }
        push_paging(&mut query, sort_by, page_index, page_size, direction);
        self.get("/openapi/market-data/screener/top-active", &query, true)
            .await
    }

    /// High-dividend screener.
    #[instrument(skip(self))]
    pub async fn high_dividend(
        &self,
        category: Category,
        sort_by: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        push_paging(&mut query, sort_by, page_index, page_size, direction);
        self.get("/openapi/market-data/screener/high-dividend", &query, true)
            .await
    }

    /// Market-sectors screener.
    #[instrument(skip(self))]
    pub async fn market_sectors(
        &self,
        category: Category,
        agg_type: Option<&str>,
        period: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![("category".to_string(), category.as_str().to_string())];
        if let Some(a) = agg_type {
            query.push(("agg_type".to_string(), a.to_string()));
        }
        if let Some(p) = period {
            query.push(("period".to_string(), p.to_string()));
        }
        push_paging(&mut query, None, page_index, page_size, direction);
        self.get("/openapi/market-data/screener/market-sectors", &query, true)
            .await
    }

    /// Market-sectors detail screener.
    #[instrument(skip(self))]
    #[allow(clippy::too_many_arguments)]
    pub async fn market_sectors_detail(
        &self,
        sector_id: &str,
        category: Category,
        period: Option<&str>,
        page_index: Option<u32>,
        page_size: Option<u32>,
        sort_by: Option<&str>,
        direction: Option<Direction>,
    ) -> Result<Value> {
        let mut query = vec![
            ("sector_id".to_string(), sector_id.to_string()),
            ("category".to_string(), category.as_str().to_string()),
        ];
        if let Some(p) = period {
            query.push(("period".to_string(), p.to_string()));
        }
        push_paging(&mut query, sort_by, page_index, page_size, direction);
        self.get(
            "/openapi/market-data/screener/market-sectors-detail",
            &query,
            true,
        )
        .await
    }
}
