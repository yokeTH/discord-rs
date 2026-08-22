use std::time::Duration as StdDuration;

use serenity::futures::StreamExt;
use tokio::time::timeout;
use tracing::{debug, info, instrument};

use crate::render::{BATCH_SIZE, render_batch};
use crate::{Context, Error};

#[poise::command(slash_command)]
#[instrument(name = "cmd_trigger", skip(ctx), fields(user_id = %ctx.author().id))]
pub async fn trigger(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    debug!("deferred reply");

    let channel_id = ctx.channel_id().get() as i64;
    let symbols = timeout(
        StdDuration::from_secs(2),
        ctx.data().symbol_store.list(channel_id),
    )
    .await
    .map_err(|_| Error::msg("list() timed out"))??;

    info!(total_symbols = symbols.len(), "loaded symbols");

    let mut batches = stock::scan(ctx.data().price_client.clone(), symbols).chunks(BATCH_SIZE);
    let mut hits = 0;

    while let Some(batch) = batches.next().await {
        hits += batch.len();
        info!(hits, batch = batch.len(), "sending batch");

        let (embeds, attachments) = render_batch(batch);
        ctx.send(poise::CreateReply {
            embeds,
            attachments,
            ..Default::default()
        })
        .await?;
    }

    info!(hits, "completed trigger scan");

    if hits == 0 {
        ctx.say("No Buy/Sell signals found.").await?;
    }

    Ok(())
}
