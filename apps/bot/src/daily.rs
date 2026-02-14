use std::{mem::take, sync::Arc};

use anyhow::Result;
use bot::Error;
use chrono::Duration;
use serenity::all::{ChannelId, CreateAttachment, CreateEmbed, CreateMessage, Http};
use serenity::futures::{StreamExt, stream};
use stock::indicators::cdc::{Signal, calculate, generate_chart};
use stock::{PriceClient, SymbolStore, Timeframe};

use tracing::{debug, error, info, instrument, warn};
use tracing_futures::Instrument;

struct Hit {
    embed: CreateEmbed,
    attachment: CreateAttachment,
}

#[instrument(
    name = "run_daily",
    skip(http, price_client, symbol_store),
    fields(channel_id = %channel)
)]
pub async fn run_daily(
    http: Arc<Http>,
    channel: ChannelId,
    price_client: Arc<PriceClient>,
    symbol_store: Arc<SymbolStore>,
) -> Result<()> {
    let symbols = symbol_store.list().await?;
    info!(total_symbols = symbols.len(), "loaded symbols");

    let mut embeds: Vec<CreateEmbed> = Vec::new();
    let mut attachments: Vec<CreateAttachment> = Vec::new();

    const CONCURRENCY: usize = 8;
    const BATCH_SIZE: usize = 10;

    let mut tasks = stream::iter(symbols)
        .map(|symbol| {
            let price_client = price_client.clone();

            let span = tracing::info_span!("daily_symbol", symbol = %symbol);

            async move {
                let bars = match price_client
                    .fetch_price(symbol.as_str(), Duration::days(300), Timeframe::Day1, 365)
                    .await
                {
                    Ok(b) => {
                        debug!(bars = b.len(), "fetched price bars");
                        b
                    }
                    Err(e) => {
                        warn!(error = ?e, "fetch_price failed");
                        return Ok::<Option<Hit>, Error>(None);
                    }
                };

                if bars.is_empty() {
                    debug!("no bars returned");
                    return Ok::<Option<Hit>, Error>(None);
                }

                let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
                let dates: Vec<String> = bars
                    .iter()
                    .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
                    .collect();

                let (sig, ema12, ema26) = calculate(&closes);
                info!(signal = ?sig, "calculated indicators");

                match sig {
                    Signal::Buy | Signal::Sell => {
                        let filename = format!("{}_chart.png", symbol);
                        let title = format!("{} Analysis", symbol.to_uppercase());
                        let desc = format!("Current Signal: {:?}", sig);

                        let color = match sig {
                            Signal::Buy => 0x00FF00,
                            Signal::Sell => 0xFF0000,
                            _ => 0x808080,
                        };

                        let embed = CreateEmbed::default()
                            .title(title)
                            .description(desc)
                            .color(color)
                            .image(format!("attachment://{}", filename));

                        let symbol_s = symbol.to_string();
                        let closes_c = closes.clone();
                        let ema12_c = ema12.clone();
                        let ema26_c = ema26.clone();
                        let dates_c = dates.clone();

                        debug!("generating chart (spawn_blocking)");
                        let image_bytes = match tokio::task::spawn_blocking(move || {
                            generate_chart(&symbol_s, &closes_c, &ema12_c, &ema26_c, &dates_c)
                        })
                        .await
                        {
                            Ok(r) => match r {
                                Ok(bytes) => {
                                    info!(bytes = bytes.len(), "chart generated");
                                    bytes
                                }
                                Err(e) => {
                                    warn!(error = ?e, "generate_chart failed");
                                    return Ok::<Option<Hit>, Error>(None);
                                }
                            },
                            Err(e) => {
                                warn!(error = ?e, "spawn_blocking join failed");
                                return Ok::<Option<Hit>, Error>(None);
                            }
                        };

                        let attachment = CreateAttachment::bytes(image_bytes, filename);
                        Ok::<Option<Hit>, Error>(Some(Hit { embed, attachment }))
                    }
                    Signal::BullishZone | Signal::BearishZone | Signal::None => {
                        debug!("no actionable signal");
                        Ok::<Option<Hit>, Error>(None)
                    }
                }
            }
            .instrument(span)
        })
        .buffer_unordered(CONCURRENCY);

    let mut processed: usize = 0;
    let mut hits: usize = 0;
    let mut failures: usize = 0;

    while let Some(res) = tasks.next().await {
        processed += 1;

        match res {
            Ok(Some(hit)) => {
                hits += 1;
                embeds.push(hit.embed);
                attachments.push(hit.attachment);

                if embeds.len() == BATCH_SIZE {
                    info!(processed, hits, "sending batch");
                    let msg = CreateMessage::new()
                        .embeds(take(&mut embeds))
                        .add_files(take(&mut attachments));

                    if let Err(e) = channel.send_message(&http, msg).await {
                        warn!(error = ?e, "send batch failed");
                    } else {
                        debug!("batch sent");
                    }
                }
            }
            Ok(None) => {
                // normal: no signal or skipped due to handled per-symbol issue
            }
            Err(e) => {
                failures += 1;
                error!(error = ?e, processed, "symbol task returned Err");
            }
        }
    }

    info!(processed, hits, failures, "completed daily scan");

    if !embeds.is_empty() {
        info!(remaining = embeds.len(), "sending final batch");
        let msg = CreateMessage::new().embeds(embeds).add_files(attachments);
        channel.send_message(&http, msg).await?;
    } else {
        info!("no actionable signals found");
        // channel
        //     .send_message(
        //         &http,
        //         CreateMessage::new().content("No Buy/Sell signals found."),
        //     )
        //     .await?;
    }

    Ok(())
}
