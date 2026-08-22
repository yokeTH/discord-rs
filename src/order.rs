//! Trading rules for `/webull buy` and `/webull sell`.
//!
//! Pure: no Discord, no network. Sizing validation, the shape of the Webull
//! order, and the orders awaiting confirmation live here so the rules can be
//! tested without a broker or a gateway.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use webull::{EntrustType, NewOrder, OrderSide, OrderType, TimeInForce, TradingSession};

/// An order previewed and awaiting a Confirm press.
pub struct PendingTrade {
    /// Discord user who initiated it (only they may confirm).
    pub owner: u64,
    pub account_id: String,
    pub order: NewOrder,
    /// Human-readable one-liner shown on the confirmation and the receipt.
    pub summary: String,
}

/// In-memory store of orders awaiting confirmation, keyed by request id.
pub type PendingTrades = Arc<Mutex<HashMap<String, PendingTrade>>>;

/// Whether a trade's size is a number of shares or a dollar (cash) amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, poise::ChoiceParameter)]
pub enum Unit {
    #[name = "shares"]
    Shares,
    #[name = "dollars"]
    Dollars,
}

impl Unit {
    /// Stable token used in component custom ids.
    pub fn token(self) -> &'static str {
        match self {
            Unit::Shares => "shares",
            Unit::Dollars => "dollars",
        }
    }

    /// Parse a [`Unit::token`] back into a [`Unit`].
    pub fn from_token(token: &str) -> Option<Unit> {
        match token {
            "shares" => Some(Unit::Shares),
            "dollars" => Some(Unit::Dollars),
            _ => None,
        }
    }
}

/// Validate sizing and build the Webull order. `Err` carries a user-facing
/// reason the order can't be placed.
pub fn build_order(
    side: OrderSide,
    symbol: &str,
    size: f64,
    unit: Unit,
    limit_price: Option<f64>,
) -> Result<NewOrder, String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err("Provide a ticker symbol.".to_string());
    }
    if size <= 0.0 || size.is_nan() {
        return Err("Size must be a positive number.".to_string());
    }
    if let Some(p) = limit_price {
        if p <= 0.0 {
            return Err("`limit_price` must be positive.".to_string());
        }
        if unit == Unit::Dollars {
            return Err(
                "Dollar-amount orders are market only — leave the limit price blank.".to_string(),
            );
        }
    }

    let order_type = if limit_price.is_some() {
        OrderType::Limit
    } else {
        OrderType::Market
    };
    let entrust_type = match unit {
        Unit::Shares => EntrustType::Qty,
        Unit::Dollars => EntrustType::Amount,
    };

    let client_order_id = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
    let mut order = NewOrder::new(
        client_order_id,
        &symbol,
        side,
        order_type,
        entrust_type,
        TimeInForce::Day,
    );
    order.support_trading_session = Some(TradingSession::Core);
    match unit {
        Unit::Shares => order.quantity = Some(size.to_string()),
        Unit::Dollars => order.total_cash_amount = Some(size.to_string()),
    }
    if let Some(p) = limit_price {
        order.limit_price = Some(p.to_string());
    }

    Ok(order)
}

/// One-line summary shown on the confirmation and the receipt.
pub fn summarize(order: &NewOrder, account_id: &str) -> String {
    let sizing = match (&order.quantity, &order.total_cash_amount) {
        (Some(qty), _) => format!("{qty} sh"),
        (_, Some(cash)) => format!("${cash}"),
        _ => String::new(),
    };
    let price = match &order.limit_price {
        Some(p) => format!("LIMIT @ {p}"),
        None => "MARKET".to_string(),
    };

    format!(
        "**{} {}** — {sizing} · {price} · DAY\nAccount `{account_id}`",
        order.side, order.symbol
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_market_order() {
        let o = build_order(OrderSide::Buy, " aapl ", 10.0, Unit::Shares, None).unwrap();
        assert_eq!(o.symbol, "AAPL");
        assert_eq!(o.order_type, OrderType::Market);
        assert_eq!(o.entrust_type, EntrustType::Qty);
        assert_eq!(o.quantity.as_deref(), Some("10"));
        assert_eq!(o.total_cash_amount, None);
        assert_eq!(o.limit_price, None);
    }

    #[test]
    fn dollars_use_cash_amount_not_quantity() {
        let o = build_order(OrderSide::Buy, "AAPL", 250.0, Unit::Dollars, None).unwrap();
        assert_eq!(o.entrust_type, EntrustType::Amount);
        assert_eq!(o.total_cash_amount.as_deref(), Some("250"));
        assert_eq!(o.quantity, None);
    }

    #[test]
    fn limit_price_switches_order_type() {
        let o = build_order(OrderSide::Sell, "AAPL", 5.0, Unit::Shares, Some(150.5)).unwrap();
        assert_eq!(o.order_type, OrderType::Limit);
        assert_eq!(o.limit_price.as_deref(), Some("150.5"));
    }

    #[test]
    fn rejects_bad_sizing() {
        // Webull notional orders take no limit price.
        assert!(build_order(OrderSide::Buy, "AAPL", 100.0, Unit::Dollars, Some(10.0)).is_err());
        assert!(build_order(OrderSide::Buy, "AAPL", 0.0, Unit::Shares, None).is_err());
        assert!(build_order(OrderSide::Buy, "AAPL", -1.0, Unit::Shares, None).is_err());
        assert!(build_order(OrderSide::Buy, "AAPL", f64::NAN, Unit::Shares, None).is_err());
        assert!(build_order(OrderSide::Buy, "AAPL", 1.0, Unit::Shares, Some(0.0)).is_err());
        assert!(build_order(OrderSide::Buy, "   ", 1.0, Unit::Shares, None).is_err());
    }

    #[test]
    fn summary_reports_the_order_as_built() {
        let shares = build_order(OrderSide::Buy, "AAPL", 10.0, Unit::Shares, Some(150.5)).unwrap();
        assert_eq!(
            summarize(&shares, "ACC1"),
            "**BUY AAPL** — 10 sh · LIMIT @ 150.5 · DAY\nAccount `ACC1`"
        );

        let cash = build_order(OrderSide::Sell, "TSLA", 250.0, Unit::Dollars, None).unwrap();
        assert_eq!(
            summarize(&cash, "ACC2"),
            "**SELL TSLA** — $250 · MARKET · DAY\nAccount `ACC2`"
        );
    }
}
