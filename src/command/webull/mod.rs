//! `/webull` — account access and manual trading.
//!
//! Every subcommand is owner-gated (see [`crate::auth::owner_only`]). Both
//! [`buy`] and [`sell`] end in the same place: preview the order's estimated
//! cost, stash it behind a Confirm/Cancel button, and only place it when the
//! owner presses Confirm. That shared flow lives here; the two commands differ
//! only in how they collect the symbol and size.

mod buy;
mod login;
pub mod sell;

use poise::serenity_prelude as serenity;
use tracing::{error, info, instrument, warn};
use webull::{Category, OrderRequest, OrderSide, WebullClient};

use crate::component::{respond_message, update_message};
use crate::order::{PendingTrade, Unit, build_order, summarize};
use crate::{Context, Data, Error};

use buy::buy;
use login::login;
use sell::sell;

pub const TRADE_CONFIRM_PREFIX: &str = "trade_confirm_";
pub const TRADE_CANCEL_PREFIX: &str = "trade_cancel_";

#[poise::command(slash_command, rename = "webull", subcommands("login", "buy", "sell"))]
pub async fn webull_command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// A previewed order ready to show behind a Confirm/Cancel button row.
pub(crate) struct PreparedOrder {
    pub content: String,
    pub row: serenity::CreateActionRow,
}

/// Validate sizing, build and preview the order, stash it in `pending_trades`,
/// and return the confirmation message + button row. `Err(msg)` is a
/// user-facing reason it can't proceed (bad input, Webull unconfigured, etc.).
pub(crate) async fn prepare_order(
    data: &Data,
    user_id: u64,
    side: OrderSide,
    symbol: String,
    size: f64,
    unit: Unit,
    limit_price: Option<f64>,
) -> Result<PreparedOrder, String> {
    let order = build_order(side, &symbol, size, unit, limit_price)?;

    let Some(client) = data.webull.clone() else {
        return Err("Webull is not configured (no live access token).".to_string());
    };
    let account_id = resolve_account_id(data, &client)
        .await
        .map_err(|e| format!("Couldn't resolve a Webull account: {e}"))?;

    let request = OrderRequest {
        account_id: account_id.clone(),
        new_orders: vec![order.clone()],
    };
    let preview_line = preview(&client, &request).await;
    let summary = summarize(&order, &account_id);

    let req_id = format!("{user_id}-{}", uuid::Uuid::new_v4().simple());
    data.pending_trades.lock().unwrap().insert(
        req_id.clone(),
        PendingTrade {
            owner: user_id,
            account_id,
            order,
            summary: summary.clone(),
        },
    );

    info!(req_id = %req_id, "order previewed, awaiting confirmation");

    Ok(PreparedOrder {
        content: format!("{summary}\n{preview_line}"),
        row: confirm_row(&req_id, side),
    })
}

/// Webull's cost estimate for an order, as a display line. A failed preview is
/// reported inline rather than blocking the order.
async fn preview(client: &WebullClient, request: &OrderRequest) -> String {
    let estimate = match client.preview_order(request, Category::UsStock).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = ?e, "preview_order failed");
            return "Preview unavailable.".to_string();
        }
    };

    let mut parts = Vec::new();
    if !estimate.estimated_cost.is_empty() {
        parts.push(format!("est. cost ${}", estimate.estimated_cost));
    }
    if !estimate.estimated_transaction_fee.is_empty() {
        parts.push(format!("fee ${}", estimate.estimated_transaction_fee));
    }

    if parts.is_empty() {
        "Preview: no estimate returned.".to_string()
    } else {
        format!("Preview: {}", parts.join(" · "))
    }
}

/// Confirm/Cancel buttons for a previewed order.
fn confirm_row(req_id: &str, side: OrderSide) -> serenity::CreateActionRow {
    let confirm_style = if side == OrderSide::Sell {
        serenity::ButtonStyle::Danger
    } else {
        serenity::ButtonStyle::Success
    };

    serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("{TRADE_CONFIRM_PREFIX}{req_id}"))
            .label("Confirm")
            .style(confirm_style),
        serenity::CreateButton::new(format!("{TRADE_CANCEL_PREFIX}{req_id}"))
            .label("Cancel")
            .style(serenity::ButtonStyle::Secondary),
    ])
}

/// The account to trade in: `WEBULL_ACCOUNT_ID` if set, else the first account.
pub(crate) async fn resolve_account_id(
    data: &Data,
    client: &WebullClient,
) -> Result<String, Error> {
    if let Some(id) = &data.webull_account_id {
        return Ok(id.clone());
    }
    let accounts = client.account_list().await?;
    accounts
        .into_iter()
        .next()
        .map(|a| a.account_id)
        .ok_or_else(|| Error::msg("no Webull accounts on this token"))
}

/// Place the order behind a confirmed `trade_confirm_<req_id>` button.
#[instrument(
    name = "trade_confirm",
    skip(ctx, data, interaction),
    fields(req_id = %req_id, user_id = %interaction.user.id)
)]
pub async fn confirm(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    req_id: &str,
) -> Result<(), Error> {
    let pending = data.pending_trades.lock().unwrap().remove(req_id);
    let Some(pending) = pending else {
        return respond_message(
            ctx,
            interaction,
            "❌ Session expired. Run the command again.",
        )
        .await;
    };

    if pending.owner != interaction.user.id.get() {
        // Reinsert so the rightful owner can still act on it.
        data.pending_trades
            .lock()
            .unwrap()
            .insert(req_id.to_string(), pending);
        return respond_message(
            ctx,
            interaction,
            "❌ You can't confirm someone else's order.",
        )
        .await;
    }

    let Some(client) = data.webull.clone() else {
        return update_message(ctx, interaction, "❌ Webull is not configured.").await;
    };

    // Ack within Discord's 3s window and clear the buttons before the (possibly
    // slower) place call, so the order can't be double-submitted.
    update_message(
        ctx,
        interaction,
        &format!("⏳ Placing…\n{}", pending.summary),
    )
    .await?;

    let request = OrderRequest {
        account_id: pending.account_id.clone(),
        new_orders: vec![pending.order.clone()],
    };
    match client.place_order(&request, Category::UsStock).await {
        Ok(resp) => {
            info!(order_id = %resp.order_id, "order placed");
            edit_response(
                ctx,
                interaction,
                &format!(
                    "✅ Order placed.\n{}\nOrder id `{}`",
                    pending.summary, resp.order_id
                ),
            )
            .await
        }
        Err(e) => {
            error!(error = ?e, "place_order failed");
            edit_response(
                ctx,
                interaction,
                &format!("❌ Order failed: {e}\n{}", pending.summary),
            )
            .await
        }
    }
}

/// Drop a pending order behind a `trade_cancel_<req_id>` button.
#[instrument(name = "trade_cancel", skip(ctx, data, interaction), fields(req_id = %req_id))]
pub async fn cancel(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    req_id: &str,
) -> Result<(), Error> {
    data.pending_trades.lock().unwrap().remove(req_id);
    update_message(ctx, interaction, "Cancelled.").await
}

/// Edit the already-acked interaction response with the final text.
async fn edit_response(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    content: &str,
) -> Result<(), Error> {
    interaction
        .edit_response(
            ctx,
            serenity::EditInteractionResponse::new().content(content),
        )
        .await?;
    Ok(())
}
