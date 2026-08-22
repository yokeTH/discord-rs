use std::sync::Arc;

use anyhow::Result;
use bot::render::{BATCH_SIZE, render_batch};
use serenity::all::{ChannelId, CreateMessage, Http};
use serenity::futures::StreamExt;
use stock::{PriceClient, SymbolStore};
use tracing::{error, info, instrument, warn};
use tracing_futures::Instrument;

#[instrument(name = "run_daily", skip_all)]
pub async fn run_daily(
    http: Arc<Http>,
    price_client: Arc<PriceClient>,
    symbol_store: Arc<SymbolStore>,
) -> Result<()> {
    let channels = symbol_store.channels().await?;
    info!(channels = channels.len(), "loaded channels");

    for channel_id in channels {
        let symbols = match symbol_store.list(channel_id).await {
            Ok(s) => s,
            Err(e) => {
                error!(channel_id, error = ?e, "failed to list symbols");
                continue;
            }
        };

        let span = tracing::info_span!("daily_channel", channel_id);
        scan_channel(
            &http,
            ChannelId::new(channel_id as u64),
            Arc::clone(&price_client),
            symbols,
        )
        .instrument(span)
        .await;
    }

    Ok(())
}

/// Scan one channel's watchlist and post each batch of signals as it lands. A
/// failed send is logged and skipped so the rest of the scan still reports.
async fn scan_channel(
    http: &Arc<Http>,
    channel: ChannelId,
    price_client: Arc<PriceClient>,
    symbols: Vec<String>,
) {
    info!(total_symbols = symbols.len(), "loaded symbols");

    let mut batches = stock::scan(price_client, symbols).chunks(BATCH_SIZE);
    let mut hits = 0;

    while let Some(batch) = batches.next().await {
        hits += batch.len();
        info!(hits, batch = batch.len(), "sending batch");

        let (embeds, attachments) = render_batch(batch);
        let msg = CreateMessage::new().embeds(embeds).add_files(attachments);

        if let Err(e) = channel.send_message(http, msg).await {
            warn!(error = ?e, "send batch failed");
        }
    }

    info!(hits, "completed daily scan");
}
