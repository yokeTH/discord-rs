//! Client for the Webull OpenAPI (Thailand — `developer.webull.co.th`).
//!
//! Covers market data (snapshot / quotes / bars), instrument reference data,
//! account queries, and the stock order lifecycle. Every request is signed with
//! a per-request HMAC-SHA256 signature (see [`signature`]) and, for
//! authenticated calls, carries a verified `x-access-token`.
//!
//! # Getting started
//!
//! ```no_run
//! # async fn run() -> anyhow::Result<()> {
//! use webull::{WebullClient, Category};
//!
//! // Reads WEBULL_APP_KEY / WEBULL_APP_SECRET / WEBULL_ACCESS_TOKEN.
//! let client = WebullClient::from_env()?;
//! let snap = client.snapshot(&["AAPL"], Category::UsStock, false, false).await?;
//! println!("AAPL: {}", snap[0].price);
//! # Ok(())
//! # }
//! ```
//!
//! Authenticated endpoints require a `NORMAL` access token. Create one with
//! [`WebullClient::create_token`], verify it via SMS 2FA in the Webull app
//! within 5 minutes, then supply it via `WEBULL_ACCESS_TOKEN` or
//! [`WebullClient::with_access_token`]. Tokens last ~15 days.

mod account;
mod auth;
mod client;
mod error;
mod instrument;
mod market;
mod order;
mod signature;
mod types;

pub use account::{Account, Balance, CurrencyAsset, Position};
pub use auth::Token;
pub use client::{PROD_BASE_URL, UAT_BASE_URL, WebullClient};
pub use instrument::Instrument;
pub use market::{Bar, Quote, QuoteBroker, QuoteLevel, QuoteOrder, Snapshot};
pub use order::{
    Commission, Fee, ModifyOrder, NewOrder, OrderLeg, OrderRecord, OrderRequest, OrderResponse,
    PreviewResponse, ReplaceOrderRequest,
};
pub use types::{
    Category, ComboType, EntrustType, InstrumentType, Market, OrderSide, OrderStatus, OrderType,
    TimeInForce, Timespan, TokenStatus, TradingSession,
};

/// The crate error type. Matches the repo convention of using `anyhow`.
pub type Error = anyhow::Error;
