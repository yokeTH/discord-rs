use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn watch(
    ctx: Context<'_>,
    #[description = "Ticker symbol (e.g., TSLA)"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    ctx.say(format!("Unimplemented -> Symbol {}", symbol))
        .await?;

    Ok(())
}
