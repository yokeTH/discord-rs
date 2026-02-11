use crate::{Context, Error};

use tracing::{debug, info, instrument, warn};

#[poise::command(slash_command)]
#[instrument(name = "cmd_watch", skip(ctx), fields(user_id = %ctx.author().id, raw = %symbol))]
pub async fn watch(
    ctx: Context<'_>,
    #[description = "Ticker symbol(s), comma-separated (e.g., TSLA,MSFT)"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    debug!("deferred reply");

    let store = &ctx.data().symbol_store;

    let symbols: Vec<String> = symbol
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    info!(count = symbols.len(), symbols = %symbols.join(", "), "parsed symbols");

    if symbols.is_empty() {
        warn!("no valid symbols provided");
        ctx.say("No valid symbols provided.").await?;
        return Ok(());
    }

    let mut added = Vec::new();
    let mut already = Vec::new();

    for sym in symbols {
        match store.add(&sym).await? {
            true => {
                info!(symbol = %sym, "added symbol to watchlist");
                added.push(sym);
            }
            false => {
                debug!(symbol = %sym, "symbol already watched");
                already.push(sym);
            }
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

    info!(
        added = added.len(),
        already = already.len(),
        "completed watch request"
    );

    Ok(())
}
