mod delete;
mod graph;
mod trigger;
mod watch;

use crate::{Context, Error};
use delete::delete;
use graph::graph;
use trigger::trigger;
use watch::watch;

#[poise::command(
    slash_command,
    rename = "stock",
    subcommands("delete", "watch", "graph", "trigger")
)]
pub async fn stock_command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
