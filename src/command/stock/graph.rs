use poise::CreateReply;
use tracing::{debug, info, instrument};

use crate::render::signal_embed;
use crate::{Context, Error};

#[poise::command(slash_command)]
#[instrument(name = "cmd_graph", skip(ctx), fields(symbol = %symbol))]
pub async fn graph(
    ctx: Context<'_>,
    #[description = "Symbol of stock to generate"] symbol: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    debug!("deferred reply");

    let analysis = stock::analyze(&ctx.data().price_client, &symbol).await?;
    let chart = analysis.chart().await?;
    info!(bytes = chart.len(), signal = ?analysis.signal, "chart generated");

    let (embed, attachment) = signal_embed(&analysis.symbol, analysis.signal, chart);
    ctx.send(CreateReply::default().embed(embed).attachment(attachment))
        .await?;

    info!("sent response");
    Ok(())
}
