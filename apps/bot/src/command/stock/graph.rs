use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Symbol of stock to generate"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    ctx.say(format!("Unimplemented -> Symbol {}", symbol))
        .await?;

    Ok(())
}
