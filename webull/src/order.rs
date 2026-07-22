//! Stock order lifecycle: preview, place, replace, cancel, and queries.

use serde::{Deserialize, Serialize};

use crate::types::{
    ComboType, EntrustType, InstrumentType, Market, OrderSide, OrderStatus, OrderType, TimeInForce,
    TradingSession,
};

#[cfg(feature = "http")]
use crate::client::WebullClient;
#[cfg(feature = "http")]
use anyhow::Result;
#[cfg(feature = "http")]
use tracing::instrument;

/// A single order to preview or place.
///
/// Build with [`NewOrder::new`] (which defaults `combo_type`/`instrument_type`/
/// `market` to the only supported stock values) and set the optional pricing
/// fields as needed. Optional fields are omitted from the request when `None`.
#[derive(Debug, Clone, Serialize)]
pub struct NewOrder {
    pub client_order_id: String,
    pub combo_type: ComboType,
    pub instrument_type: InstrumentType,
    pub market: Market,
    pub symbol: String,
    pub order_type: OrderType,
    pub entrust_type: EntrustType,
    pub side: OrderSide,
    pub time_in_force: TimeInForce,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_trading_session: Option<TradingSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cash_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
}

impl NewOrder {
    /// A stock order with the required fields set and the fixed stock values
    /// (`combo_type = NORMAL`, `instrument_type = EQUITY`, `market = US`)
    /// defaulted. Set `quantity`/`limit_price`/etc. on the returned value.
    pub fn new(
        client_order_id: impl Into<String>,
        symbol: impl Into<String>,
        side: OrderSide,
        order_type: OrderType,
        entrust_type: EntrustType,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            client_order_id: client_order_id.into(),
            combo_type: ComboType::Normal,
            instrument_type: InstrumentType::Equity,
            market: Market::Us,
            symbol: symbol.into(),
            order_type,
            entrust_type,
            side,
            time_in_force,
            support_trading_session: None,
            quantity: None,
            total_cash_amount: None,
            limit_price: None,
            stop_price: None,
        }
    }
}

/// Request body for preview and place (`account_id` + one or more orders).
#[derive(Debug, Clone, Serialize)]
pub struct OrderRequest {
    pub account_id: String,
    pub new_orders: Vec<NewOrder>,
}

/// A single modification within a [`ReplaceOrderRequest`].
#[derive(Debug, Clone, Serialize)]
pub struct ModifyOrder {
    pub client_order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
}

/// Request body for replacing (modifying) live orders.
#[derive(Debug, Clone, Serialize)]
pub struct ReplaceOrderRequest {
    pub account_id: String,
    pub modify_orders: Vec<ModifyOrder>,
}

#[cfg(feature = "http")]
#[derive(Serialize)]
struct CancelOrderBody<'a> {
    account_id: &'a str,
    client_order_id: &'a str,
}

/// Result of placing, replacing, or cancelling an order.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderResponse {
    pub client_order_id: String,
    pub order_id: String,
}

/// Result of previewing an order.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PreviewResponse {
    pub estimated_cost: String,
    pub estimated_transaction_fee: String,
}

/// Commission detail on a filled order leg.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Commission {
    pub actual_commission: String,
    pub receivable_commission: String,
}

/// A fee line item on an order leg.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Fee {
    #[serde(rename = "type")]
    pub fee_type: String,
    pub actual_value: String,
    pub receivable_value: String,
}

/// One leg of an order (a combo order may have several).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OrderLeg {
    pub client_order_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: Option<OrderSide>,
    pub status: Option<OrderStatus>,
    pub order_type: Option<OrderType>,
    pub instrument_type: Option<InstrumentType>,
    pub support_trading_session: Option<TradingSession>,
    pub time_in_force: Option<TimeInForce>,
    pub entrust_type: Option<EntrustType>,
    pub total_quantity: Option<String>,
    pub filled_quantity: Option<String>,
    pub filled_price: Option<String>,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
    /// Placement time, Unix milliseconds (as a string).
    pub place_time: Option<String>,
    /// Placement time, ISO-8601 UTC.
    pub place_time_at: Option<String>,
    pub filled_time: Option<String>,
    pub filled_time_at: Option<String>,
    pub commission: Option<Commission>,
    pub fees: Option<Vec<Fee>>,
}

/// An order record as returned by the query endpoints, grouping its legs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OrderRecord {
    pub client_order_id: String,
    pub combo_type: Option<ComboType>,
    pub orders: Vec<OrderLeg>,
}

#[cfg(feature = "http")]
impl WebullClient {
    /// Preview an order (estimated cost and fees) without placing it.
    #[instrument(name = "webull_preview_order", skip_all)]
    pub async fn preview_order(&self, request: &OrderRequest) -> Result<PreviewResponse> {
        self.post("/openapi/trade/order/preview", request, true)
            .await
    }

    /// Place one or more orders.
    #[instrument(name = "webull_place_order", skip_all, fields(account_id = %request.account_id, orders = request.new_orders.len()))]
    pub async fn place_order(&self, request: &OrderRequest) -> Result<OrderResponse> {
        self.post("/openapi/trade/order/place", request, true).await
    }

    /// Modify one or more live orders.
    #[instrument(name = "webull_replace_order", skip_all, fields(account_id = %request.account_id))]
    pub async fn replace_order(&self, request: &ReplaceOrderRequest) -> Result<OrderResponse> {
        self.post("/openapi/trade/order/replace", request, true)
            .await
    }

    /// Cancel a live order by its client order id.
    #[instrument(name = "webull_cancel_order", skip(self))]
    pub async fn cancel_order(
        &self,
        account_id: &str,
        client_order_id: &str,
    ) -> Result<OrderResponse> {
        let body = CancelOrderBody {
            account_id,
            client_order_id,
        };
        self.post("/openapi/trade/order/cancel", &body, true).await
    }

    /// Order history (defaults to the last 7 days). `start_date` is `yyyy-MM-dd`;
    /// `page_size` is 10–100; `last_client_order_id` is the pagination cursor.
    #[instrument(name = "webull_order_history", skip(self))]
    pub async fn order_history(
        &self,
        account_id: &str,
        start_date: Option<&str>,
        page_size: Option<u32>,
        last_client_order_id: Option<&str>,
    ) -> Result<Vec<OrderRecord>> {
        let mut query = vec![("account_id".to_string(), account_id.to_string())];
        if let Some(d) = start_date {
            query.push(("start_date".to_string(), d.to_string()));
        }
        if let Some(n) = page_size {
            query.push(("page_size".to_string(), n.to_string()));
        }
        if let Some(c) = last_client_order_id {
            query.push(("last_client_order_id".to_string(), c.to_string()));
        }
        self.get("/openapi/trade/order/history", &query, true).await
    }

    /// Currently open (working) orders for an account.
    #[instrument(name = "webull_open_orders", skip(self))]
    pub async fn open_orders(
        &self,
        account_id: &str,
        page_size: Option<u32>,
        last_client_order_id: Option<&str>,
    ) -> Result<Vec<OrderRecord>> {
        let mut query = vec![("account_id".to_string(), account_id.to_string())];
        if let Some(n) = page_size {
            query.push(("page_size".to_string(), n.to_string()));
        }
        if let Some(c) = last_client_order_id {
            query.push(("last_client_order_id".to_string(), c.to_string()));
        }
        self.get("/openapi/trade/order/open", &query, true).await
    }

    /// Full detail for a single order (includes commission and fees).
    #[instrument(name = "webull_order_detail", skip(self))]
    pub async fn order_detail(
        &self,
        account_id: &str,
        client_order_id: &str,
    ) -> Result<OrderRecord> {
        let query = vec![
            ("account_id".to_string(), account_id.to_string()),
            ("client_order_id".to_string(), client_order_id.to_string()),
        ];
        self.get("/openapi/trade/order/detail", &query, true).await
    }
}
