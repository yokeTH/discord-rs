use chrono::Duration;
use log::debug;
use poise::CreateReply;
use serenity::all::{CreateAttachment, CreateEmbed};
use stock::indicators::cdc::{Signal, calculate, generate_chart};

use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Symbol of stock to generate"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let price_client = &ctx.data().price_client;

    let bars = price_client
        .fetch_price(
            symbol.as_str(),
            Duration::days(300),
            stock::Timeframe::Day1,
            365,
        )
        .await?;
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let dates: Vec<String> = bars
        .iter()
        .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
        .collect();

    let (sig, ema12, ema26) = calculate(&closes);

    debug!("{} - {:?}", symbol, sig);

    let image_bytes = generate_chart(symbol.as_str(), &closes, &ema12, &ema26, &dates)?;

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

    ctx.send(CreateReply::default().embed(embed).attachment(attachment))
        .await?;

    Ok(())
}
