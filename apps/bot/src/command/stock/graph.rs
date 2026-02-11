use chrono::Duration;
use poise::CreateReply;
use serenity::all::{CreateAttachment, CreateEmbed};
use stock::indicators::cdc::{Signal, calculate, generate_chart};
use tracing::{debug, error, info, instrument};

use crate::{Context, Error};

#[poise::command(slash_command)]
#[instrument(name = "cmd_graph", skip(ctx), fields(symbol = %symbol))]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Symbol of stock to generate"] symbol: String,
) -> Result<(), Error> {
    info!("starting");

    ctx.defer().await?;
    debug!("deferred reply");

    let price_client = &ctx.data().price_client;

    debug!("fetching price bars");
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
            info!(bars = b.len(), "fetched price bars");
            b
        }
        Err(e) => {
            error!(error = ?e, "fetch_price failed");
            return Err(e.into());
        }
    };

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let dates: Vec<String> = bars
        .iter()
        .map(|b| b.timestamp.format("%Y-%m-%d").to_string())
        .collect();

    debug!(
        closes = closes.len(),
        dates = dates.len(),
        "prepared series"
    );

    let (sig, ema12, ema26) = calculate(&closes);
    info!(signal = ?sig, "calculated indicators");

    debug!("generating chart");
    let image_bytes = match generate_chart(symbol.as_str(), &closes, &ema12, &ema26, &dates) {
        Ok(bytes) => {
            info!(bytes = bytes.len(), "chart generated");
            bytes
        }
        Err(e) => {
            error!(error = ?e, "generate_chart failed");
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

    debug!("sending response");
    ctx.send(CreateReply::default().embed(embed).attachment(attachment))
        .await?;
    info!("sent response");

    Ok(())
}
