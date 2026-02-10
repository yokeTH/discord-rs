use std::mem::take;

use chrono::Duration;
use log::{debug, warn};
use serenity::all::{CreateAttachment, CreateEmbed};
use serenity::futures::{StreamExt, stream};
use stock::indicators::cdc::{Signal, calculate, generate_chart};

use crate::{Context, Error};

struct Hit {
    embed: CreateEmbed,
    attachment: CreateAttachment,
}

#[poise::command(slash_command)]
pub async fn trigger(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let price_client = ctx.data().price_client.clone();
    let symbol_store = &ctx.data().symbol_store.clone();

    let symbols = symbol_store.list().await?;

    let mut embeds: Vec<CreateEmbed> = Vec::new();
    let mut attachments: Vec<CreateAttachment> = Vec::new();

    const CONCURRENCY: usize = 8;

    let mut tasks = stream::iter(symbols)
        .map(|symbol| {
            let price_client = price_client.clone();
            async move {
                let bars = price_client
                    .fetch_price(
                        symbol.as_str(),
                        Duration::days(300),
                        stock::Timeframe::Day1,
                        365,
                    )
                    .await?;

                if bars.is_empty() {
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

                        let image_bytes = tokio::task::spawn_blocking(move || {
                            generate_chart(&symbol_s, &closes_c, &ema12_c, &ema26_c, &dates_c)
                        })
                        .await??;

                        let attachment = CreateAttachment::bytes(image_bytes, filename);

                        Ok::<Option<Hit>, Error>(Some(Hit { embed, attachment }))
                    }

                    Signal::BullishZone | Signal::BearishZone | Signal::None => {
                        Ok::<Option<Hit>, Error>(None)
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY);

    while let Some(res) = tasks.next().await {
        match res {
            Ok(Some(hit)) => {
                embeds.push(hit.embed);
                attachments.push(hit.attachment);

                if embeds.len() == 10 {
                    ctx.send(poise::CreateReply {
                        embeds: take(&mut embeds),
                        attachments: take(&mut attachments),
                        ..Default::default()
                    })
                    .await?;
                }
            }
            Ok(None) => {}
            Err(e) => {
                warn!("symbol task failed: {:?}", e);
            }
        }
    }

    if !embeds.is_empty() {
        ctx.send(poise::CreateReply {
            embeds,
            attachments,
            ..Default::default()
        })
        .await?;
    } else {
        ctx.send(poise::CreateReply {
            content: Some("No Buy/Sell signals found.".to_string()),
            ..Default::default()
        })
        .await?;
    }

    Ok(())
}
