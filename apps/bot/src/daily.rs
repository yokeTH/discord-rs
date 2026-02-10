use std::{mem::take, sync::Arc};

use anyhow::Result;
use bot::Error;
use chrono::Duration;
use log::{debug, trace, warn};
use serenity::all::{ChannelId, CreateAttachment, CreateMessage, Http};
use serenity::futures::{StreamExt, stream};
use stock::indicators::cdc::{Signal, calculate, generate_chart};
use stock::{PriceClient, SymbolStore, Timeframe};

struct Hit {
    symbol: String,
    sig: Signal,
    attachment: CreateAttachment,
}

pub async fn run_daily(
    http: Arc<Http>,
    channel: ChannelId,
    price_client: Arc<PriceClient>,
    symbol_store: Arc<SymbolStore>,
) -> Result<()> {
    let symbols = symbol_store.list().await?;

    let mut attachments: Vec<CreateAttachment> = Vec::new();
    let mut lines: Vec<String> = Vec::new();

    const CONCURRENCY: usize = 8;

    let mut tasks = stream::iter(symbols)
        .map(|symbol| {
            let price_client = price_client.clone();
            async move {
                let bars = price_client
                    .fetch_price(symbol.as_str(), Duration::days(300), Timeframe::Day1, 365)
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
                        Ok::<Option<Hit>, Error>(None)
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY);

    let mut hits = 0usize;

    while let Some(res) = tasks.next().await {
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
                    let msg = CreateMessage::new()
                        .content(format!("Buy/Sell hits:\n{}", lines.join("\n")))
                        .add_files(take(&mut attachments));

                    if let Err(e) = channel.send_message(&http, msg).await {
                        warn!("send batch failed: {:?}", e);
                    }

                    lines.clear();
                }
            }
            Ok(None) => {}
            Err(e) => warn!("symbol task failed: {:?}", e),
        }
    }

    if !attachments.is_empty() {
        let msg = CreateMessage::new()
            .content(format!("Buy/Sell hits:\n{}", lines.join("\n")))
            .add_files(attachments);

        channel.send_message(&http, msg).await?;
    } else if hits == 0 {
        channel
            .send_message(
                &http,
                CreateMessage::new().content("No Buy/Sell signals found."),
            )
            .await?;
    }

    Ok(())
}
