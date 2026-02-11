use chrono::Duration;
use log::{debug, error, info, trace, warn};
use poise::CreateReply;
use serenity::all::{CreateAttachment, CreateEmbed};
use stock::indicators::cdc::{Signal, calculate, generate_chart};

use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Symbol of stock to generate"] symbol: String,
) -> Result<(), Error> {
    info!("Received graph command for symbol: {}", symbol);
    ctx.defer().await?;

    let price_client = &ctx.data().price_client;

    info!("Fetching price data for symbol: {}", symbol);
    let bars = match price_client
        .fetch_price(
            symbol.as_str(),
            Duration::days(300),
            stock::Timeframe::Day1,
            365,
        )
        .await
    {
        Ok(b) => {
            info!("Fetched {} bars for {}", b.len(), symbol);
            b
        }
        Err(e) => {
            error!("Failed to fetch price data for {}: {:?}", symbol, e);
            return Err(e.into());
        }
    };
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let dates: Vec<String> = bars
        .iter()
        .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
        .collect();

    debug!("Calculating CDC indicators for {}", symbol);
    let (sig, ema12, ema26) = calculate(&closes);

    info!("{} - Signal: {:?}", symbol, sig);

    debug!("Generating chart for {}", symbol);
    let image_bytes = match generate_chart(symbol.as_str(), &closes, &ema12, &ema26, &dates) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to generate chart for {}: {:?}", symbol, e);
            return Err(e.into());
        }
    };

    let filename = format!("{}_chart.png", symbol);
    let attachment = CreateAttachment::bytes(image_bytes, filename.clone());

    let mut embed = CreateEmbed::default()
        .title(format!("{} Analysis", symbol.to_uppercase()))
        .description(format!("Current Signal: {:?}", sig))
        .image(format!("attachment://{}", filename));

    embed = match sig {
        Signal::Buy | Signal::BullishZone => embed.color(0x00ff00),
        Signal::Sell | Signal::BearishZone => embed.color(0xff0000),
        Signal::None => embed.color(0xffffff),
    };

    info!("Sending embed for symbol: {}", symbol);
    ctx.send(CreateReply::default().embed(embed).attachment(attachment))
        .await?;

    Ok(())
}
