use std::mem::take;

use chrono::Duration;
use log::{debug, error, info, trace, warn};
use serenity::all::CreateAttachment;
use serenity::futures::{StreamExt, stream};
use stock::Timeframe;
use stock::indicators::cdc::{Signal, calculate, generate_chart};

use crate::{Context, Error};

struct Hit {
    symbol: String,
    sig: Signal,
    attachment: CreateAttachment,
}

#[poise::command(slash_command)]
pub async fn trigger(ctx: Context<'_>) -> Result<(), Error> {
    info!("Trigger command invoked by user: {:?}", ctx.author().name);
    ctx.defer().await?;

    let price_client = ctx.data().price_client.clone();
    let symbol_store = &ctx.data().symbol_store.clone();

    info!("Fetching symbol list...");
    let symbols = symbol_store.list().await?;
    info!("Fetched {} symbols.", symbols.len());

    let mut attachments: Vec<CreateAttachment> = Vec::new();
    let mut lines: Vec<String> = Vec::new();

    const CONCURRENCY: usize = 8;

    let mut tasks = stream::iter(symbols)
        .map(|symbol| {
            let price_client = price_client.clone();
            async move {
                info!("Processing symbol: {}", symbol);

                let bars = price_client
                    .fetch_price(symbol.as_str(), Duration::days(300), Timeframe::Day1, 365)
                    .await?;

                if bars.is_empty() {
                    warn!("No bars found for symbol: {}", symbol);
                    return Ok::<Option<Hit>, Error>(None);
                }

                let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
                let dates: Vec<String> = bars
                    .iter()
                    .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
                    .collect();

                let (sig, ema12, ema26) = calculate(&closes);
                debug!("{} - {:?}", symbol, sig);

                match sig {
                    Signal::Buy | Signal::Sell => {
                        info!("Signal {:?} detected for symbol: {}", sig, symbol);

                        let filename = format!("{}_chart.png", symbol.to_lowercase());

                        let symbol_s = symbol.to_string();
                        let closes_c = closes.clone();
                        let ema12_c = ema12.clone();
                        let ema26_c = ema26.clone();
                        let dates_c = dates.clone();

                        let symbol_for_hit = symbol_s.clone();

                        trace!("Spawning blocking task to generate chart for {}", symbol_s);
                        let image_bytes = tokio::task::spawn_blocking(move || {
                            generate_chart(
                                symbol_s.as_str(),
                                &closes_c,
                                &ema12_c,
                                &ema26_c,
                                &dates_c,
                            )
                        })
                        .await??;

                        let attachment = CreateAttachment::bytes(image_bytes, filename);

                        Ok::<Option<Hit>, Error>(Some(Hit {
                            symbol: symbol_for_hit,
                            sig,
                            attachment,
                        }))
                    }

                    Signal::BullishZone | Signal::BearishZone | Signal::None => {
                        trace!("No actionable signal for symbol: {}", symbol);
                        Ok::<Option<Hit>, Error>(None)
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY);

    let mut processed = 0usize;
    let mut hits = 0usize;

    while let Some(res) = tasks.next().await {
        processed += 1;
        match res {
            Ok(Some(hit)) => {
                hits += 1;

                lines.push(format!(
                    "- **{}**: `{:?}`",
                    hit.symbol.to_uppercase(),
                    hit.sig
                ));
                attachments.push(hit.attachment);

                if attachments.len() == 10 {
                    info!("Sending batch of 10 attachments to Discord.");

                    let content = format!("Buy/Sell hits:\n{}", lines.join("\n"));

                    ctx.send(poise::CreateReply {
                        content: Some(content),
                        attachments: take(&mut attachments),
                        ..Default::default()
                    })
                    .await?;

                    lines.clear();
                }
            }
            Ok(None) => {
                trace!("No hit for processed symbol #{}", processed);
            }
            Err(e) => {
                error!("symbol task failed: {:?}", e);
                warn!("symbol task failed: {:?}", e);
            }
        }
    }

    if !attachments.is_empty() {
        info!(
            "Sending final batch of {} attachments to Discord.",
            attachments.len()
        );

        let content = format!("Buy/Sell hits:\n{}", lines.join("\n"));

        ctx.send(poise::CreateReply {
            content: Some(content),
            attachments,
            ..Default::default()
        })
        .await?;
    } else if hits == 0 {
        info!("No Buy/Sell signals found to send.");
        ctx.send(poise::CreateReply {
            content: Some("No Buy/Sell signals found.".to_string()),
            ..Default::default()
        })
        .await?;
    }

    info!(
        "Trigger command completed. processed={}, hits={}",
        processed, hits
    );
    Ok(())
}
