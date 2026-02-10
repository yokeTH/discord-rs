use std::sync::Arc;

use stock::{PriceClient, SymbolStore};

pub mod command;
pub mod config;

pub struct Data {
    pub symbol_store: Arc<SymbolStore>,
    pub price_client: Arc<PriceClient>,
}

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, Data, Error>;
