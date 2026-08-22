//! `/webull buy` — buy US stock by shares or dollars.
//!
//! Takes the symbol as a typed argument, then hands off to the shared preview +
//! Confirm flow in [`super`].

use tracing::instrument;
use webull::OrderSide;

use crate::auth::owner_only;
use crate::command::webull::prepare_order;
use crate::order::Unit;
use crate::{Context, Error};

/// Buy US stock by shares or dollars; limit price is shares-only (blank = market).
#[poise::command(slash_command, check = "owner_only")]
#[instrument(
    name = "cmd_buy",
    skip(ctx, size, unit, limit_price),
    fields(user_id = %ctx.author().id, symbol = %symbol)
)]
pub async fn buy(
    ctx: Context<'_>,
    #[description = "Ticker symbol (e.g. AAPL)"] symbol: String,
    #[description = "Size to buy (shares or dollars)"] size: f64,
    #[description = "Unit for size"] unit: Unit,
    #[description = "Limit price (shares only; blank = market)"] limit_price: Option<f64>,
) -> Result<(), Error> {
    // The preview shows share/dollar sizing and an estimated cost, so the whole
    // flow — including the Confirm/Cancel message it edits — stays ephemeral.
    ctx.defer_ephemeral().await?;

    match prepare_order(
        ctx.data(),
        ctx.author().id.get(),
        OrderSide::Buy,
        symbol,
        size,
        unit,
        limit_price,
    )
    .await
    {
        Ok(prepared) => {
            ctx.send(
                poise::CreateReply::default()
                    .content(prepared.content)
                    .components(vec![prepared.row])
                    .ephemeral(true),
            )
            .await?;
        }
        Err(msg) => {
            ctx.say(msg).await?;
        }
    }

    Ok(())
}
