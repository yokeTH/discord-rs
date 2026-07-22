//! Shared enums used across market-data, account and order requests/responses.
//!
//! Money and price fields are kept as `String` throughout the crate because
//! the Webull API returns them as strings; converting to `f64` would risk
//! precision loss, which is unacceptable for a trading client. Callers parse
//! as needed.

use serde::{Deserialize, Serialize};

/// Security category, used in market-data and instrument queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    UsStock,
    UsEtf,
}

impl Category {
    /// The wire value used in query parameters.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::UsStock => "US_STOCK",
            Category::UsEtf => "US_ETF",
        }
    }
}

/// Trading venue / region. Only US stocks are supported today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Market {
    Us,
}

/// Financial instrument type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentType {
    Equity,
}

/// Order combination type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComboType {
    Normal,
}

/// Buy or sell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// How an order executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    StopLossLimit,
}

/// Whether an order is sized by share quantity or by cash amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntrustType {
    Qty,
    Amount,
}

/// How long an order remains active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Day,
    Gtc,
}

/// Trading session window an order may execute in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradingSession {
    Night,
    All,
    Core,
    AllDay,
}

/// Lifecycle status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Submitted,
    Cancelled,
    Filled,
    Failed,
    PartialFilled,
}

/// Status of an access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenStatus {
    /// Awaiting SMS 2FA verification in the Webull app.
    Pending,
    /// Active and usable for API calls.
    Normal,
    /// Unused for 15+ days, or does not exist.
    Invalid,
    /// Verification window (5 min) was missed.
    Expired,
}

/// Candlestick granularity for historical bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timespan {
    S5,
    S15,
    M1,
    M5,
    M15,
    M30,
    M60,
    M120,
    M240,
    Day,
    Week,
    Month,
    Year,
}

impl Timespan {
    /// The wire value used in the `timespan` query parameter.
    pub fn as_str(self) -> &'static str {
        match self {
            Timespan::S5 => "S5",
            Timespan::S15 => "S15",
            Timespan::M1 => "M1",
            Timespan::M5 => "M5",
            Timespan::M15 => "M15",
            Timespan::M30 => "M30",
            Timespan::M60 => "M60",
            Timespan::M120 => "M120",
            Timespan::M240 => "M240",
            Timespan::Day => "D",
            Timespan::Week => "W",
            Timespan::Month => "M",
            Timespan::Year => "Y",
        }
    }
}
