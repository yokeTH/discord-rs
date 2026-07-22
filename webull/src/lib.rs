//! Client for the Webull OpenAPI — a Rust port of the official Python SDK's
//! REST surface (trade + market data).
//!
//! Requests are signed with a per-request HMAC-SHA256 signature (see the
//! `signature` module) and, for authenticated calls, carry a verified
//! `x-access-token`. Enums and data models are always available; the HTTP
//! client itself lives behind the default `http` feature, so
//! `--no-default-features` yields a lightweight types-only build (no reqwest).
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
//! [`WebullClient::with_access_token`]. Tokens last ~15 days and can be kept
//! alive with [`WebullClient::refresh_and_store`].

mod account;
mod auth;
mod instrument;
mod market;
mod order;
mod region;
mod types;

#[cfg(feature = "http")]
mod client;
#[cfg(feature = "http")]
mod crypto;
#[cfg(feature = "http")]
mod error;
#[cfg(feature = "http")]
mod events;
#[cfg(feature = "http")]
mod fundamentals;
#[cfg(feature = "http")]
mod futures;
#[cfg(feature = "http")]
mod options;
#[cfg(feature = "http")]
mod screener;
#[cfg(feature = "http")]
mod signature;
#[cfg(feature = "http")]
mod trade_misc;
#[cfg(feature = "http")]
mod watchlist;

pub use account::{Account, Balance, CurrencyAsset, Position};
pub use auth::Token;
pub use instrument::Instrument;
pub use market::{Bar, Quote, QuoteBroker, QuoteLevel, QuoteOrder, Snapshot};
pub use order::{
    BatchOrderRequest, Commission, Fee, ModifyOrder, NewOrder, OrderLeg, OrderRecord, OrderRequest,
    OrderResponse, PreviewResponse, ReplaceOrderRequest,
};
pub use region::Region;
pub use types::{
    AccountType, Category, ComboTickerType, ComboType, ContractType, Currency, Direction,
    EntrustType, ExchangeCode, ExerciseStyle, ExpirationCycle, ForbidReason, InstrumentStatus,
    InstrumentType, Market, OptionType, OrderSide, OrderStatus, OrderType, PositionIntent,
    TimeInForce, Timespan, TokenStatus, TradePolicy, TradingDateType, TradingSession, TrailingType,
};

#[cfg(feature = "http")]
pub use client::{PROD_BASE_URL, UAT_BASE_URL, WebullClient};
#[cfg(feature = "http")]
pub use options::OptionContractsParams;

/// The crate error type. Matches the repo convention of using `anyhow`.
pub type Error = anyhow::Error;
