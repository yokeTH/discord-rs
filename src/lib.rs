use std::sync::Arc;

use stock::{PriceClient, SymbolStore};
use webull::WebullClient;

pub mod auth;
pub mod command;
pub mod component;
pub mod config;
pub mod order;
pub mod render;
pub mod webull_session;

use order::PendingTrades;

pub struct Data {
    pub symbol_store: Arc<SymbolStore>,
    pub price_client: Arc<PriceClient>,
    /// Webull client, present only when Webull is configured with a live token.
    /// Clones share the auto-refreshed access token.
    pub webull: Option<Arc<WebullClient>>,
    /// Webull account to trade in (`WEBULL_ACCOUNT_ID`); falls back to the first
    /// account when unset.
    pub webull_account_id: Option<String>,
    /// Discord user allowed to trade; `None` disables trading.
    pub owner_id: Option<u64>,
    /// Orders awaiting a Confirm button press, keyed by request id.
    pub pending_trades: PendingTrades,
}

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, Data, Error>;
