use crate::{Context, Error};
use log::{debug, info, warn};

#[poise::command(slash_command)]
pub async fn watch(
    ctx: Context<'_>,
    #[description = "Ticker symbol(s), comma-separated (e.g., TSLA,MSFT)"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id.get();
    let store = &ctx.data().symbol_store;

    info!("watch: invoked user_id={} raw_input={}", user_id, symbol);

    let symbols: Vec<String> = symbol
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    if symbols.is_empty() {
        warn!(
            "watch: no valid symbols user_id={} raw_input={}",
            user_id, symbol
        );
        ctx.say("No valid symbols provided.").await?;
        return Ok(());
    }

    info!(
        "watch: parsed symbols user_id={} count={} symbols=[{}]",
        user_id,
        symbols.len(),
        symbols.join(", ")
    );

    let mut added: Vec<String> = Vec::new();
    let mut already: Vec<String> = Vec::new();

    for sym in symbols {
        match store.add(&sym).await {
            Ok(true) => {
                debug!("watch: added user_id={} symbol={}", user_id, sym);
                added.push(sym);
            }
            Ok(false) => {
                debug!("watch: already_watched user_id={} symbol={}", user_id, sym);
                already.push(sym);
            }
            Err(e) => {
                // keep the error visible and return early, so caller sees failure
                warn!(
                    "watch: store.add failed user_id={} symbol={} err={:?}",
                    user_id, sym, e
                );
                return Err(e.into());
            }
        }
    }

    info!(
        "watch: completed user_id={} added_count={} already_count={}",
        user_id,
        added.len(),
        already.len()
    );

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
