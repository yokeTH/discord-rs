mod delete;
mod graph;
pub mod trade;
mod trigger;
mod watch;

use crate::{Context, Error};
use delete::delete;
use graph::graph;
use trade::{buy, sell};
use trigger::trigger;
use watch::watch;

#[poise::command(
    slash_command,
    rename = "stock",
    subcommands("delete", "watch", "graph", "trigger", "buy", "sell")
)]
pub async fn stock_command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
