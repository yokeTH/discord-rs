use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn watch(
    ctx: Context<'_>,
    #[description = "Ticker symbol (e.g., TSLA)"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let store = &ctx.data().symbol_store;

    if store.add(&symbol).await? {
        ctx.say(format!("Now watching: {}", symbol.to_uppercase()))
            .await?;
    } else {
        ctx.say(format!("Already watching: {}", symbol.to_uppercase()))
            .await?;
    }

    Ok(())
}
