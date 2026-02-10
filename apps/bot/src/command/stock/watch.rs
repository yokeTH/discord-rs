use crate::{Context, Error};
use log::{info, warn};

#[poise::command(slash_command)]
pub async fn watch(
    ctx: Context<'_>,
    #[description = "Ticker symbol(s), comma-separated (e.g., TSLA,MSFT)"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let store = &ctx.data().symbol_store;

    let symbols: Vec<String> = symbol
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    info!(
        "User {} requested to watch symbols: {:?}",
        ctx.author().id,
        symbols
    );

    if symbols.is_empty() {
        warn!("No valid symbols provided by user {}", ctx.author().id);
        ctx.say("No valid symbols provided.").await?;
        return Ok(());
    }

    let mut added = Vec::new();
    let mut already = Vec::new();

    for sym in symbols {
        if store.add(&sym).await? {
            info!(
                "Added symbol '{}' to watchlist for user {}",
                sym,
                ctx.author().id
            );
            added.push(sym);
        } else {
            info!(
                "Symbol '{}' was already being watched for user {}",
                sym,
                ctx.author().id
            );
            already.push(sym);
        }
    }

    if !added.is_empty() {
        ctx.say(format!("Now watching: {}", added.join(", ")))
            .await?;
    }
    if !already.is_empty() {
        ctx.say(format!("Already watching: {}", already.join(", ")))
            .await?;
    }

    Ok(())
}
