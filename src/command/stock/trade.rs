//! Manual `/stock buy` and `/stock sell` commands.
//!
//! Both are owner-gated (see [`owner_only`]), preview the order's estimated cost
//! via Webull, then stash it behind a Confirm/Cancel button. The order is only
//! placed when the owner presses Confirm (handled in [`crate::component`]).
//!
//! Size is entered as a number plus a [`Unit`] (shares or dollars). Share orders
//! may carry a limit price; dollar (cash) orders are market-only, since Webull
//! notional orders don't take a limit.
//!
//! `/stock buy` takes the symbol as a typed argument. `/stock sell` instead
//! shows a dropdown of the account's current holdings; picking one shows
//! Shares/Dollars buttons (a modal can't hold a dropdown), and the chosen unit
//! opens a modal for the number (plus a limit for shares), then the same
//! preview + Confirm flow.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use poise::serenity_prelude as serenity;
use tracing::{error, info, instrument, warn};
use webull::{
    Category, EntrustType, NewOrder, OrderRequest, OrderSide, OrderType, TimeInForce,
    TradingSession, WebullClient,
};

use crate::command::auth::owner_only;
use crate::{Context, Data, Error};

pub const TRADE_CONFIRM_PREFIX: &str = "trade_confirm_";
pub const TRADE_CANCEL_PREFIX: &str = "trade_cancel_";
/// Custom id of the `/stock sell` holdings dropdown.
pub const SELL_SELECT_ID: &str = "sell_select";
/// Prefix of the unit buttons shown after a holding is picked; the rest is
/// `<unit>_<symbol>|<qty>|<market_value>`.
pub const SELL_UNIT_PREFIX: &str = "sell_unit_";
/// Prefix of the sell modal's custom id; the rest is `<unit>_<symbol>`.
pub const SELL_MODAL_PREFIX: &str = "sell_modal_";

/// An order previewed and awaiting a Confirm press.
pub struct PendingTrade {
    /// Discord user who initiated it (only they may confirm).
    pub owner: u64,
    pub account_id: String,
    pub order: NewOrder,
    /// Human-readable one-liner shown on the confirmation and the receipt.
    pub summary: String,
}

/// In-memory store of orders awaiting confirmation, keyed by request id.
pub type PendingTrades = Arc<Mutex<HashMap<String, PendingTrade>>>;

/// Whether a trade's size is a number of shares or a dollar (cash) amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, poise::ChoiceParameter)]
pub enum Unit {
    #[name = "shares"]
    Shares,
    #[name = "dollars"]
    Dollars,
}

impl Unit {
    /// Stable token used in component custom ids.
    fn token(self) -> &'static str {
        match self {
            Unit::Shares => "shares",
            Unit::Dollars => "dollars",
        }
    }
}

/// Parse a [`Unit::token`] back into a [`Unit`].
fn unit_from_token(token: &str) -> Option<Unit> {
    match token {
        "shares" => Some(Unit::Shares),
        "dollars" => Some(Unit::Dollars),
        _ => None,
    }
}

/// From a unit-button payload `"<symbol>|<qty>|<market_value>"`, return the
/// symbol and the modal prefill for `unit` (held shares, or the position's
/// dollar value). An empty prefill means "no default".
fn sell_prefill(payload: &str, unit: Unit) -> (&str, &str) {
    let mut parts = payload.splitn(3, '|');
    let symbol = parts.next().unwrap_or("");
    let qty = parts.next().unwrap_or("");
    let market_value = parts.next().unwrap_or("");
    let prefill = match unit {
        Unit::Shares => qty,
        Unit::Dollars => market_value,
    };
    (symbol, prefill)
}

/// Buy US stock by shares or dollars; limit price is shares-only (blank = market).
#[poise::command(slash_command, check = "owner_only")]
pub async fn buy(
    ctx: Context<'_>,
    #[description = "Ticker symbol (e.g. AAPL)"] symbol: String,
    #[description = "Size to buy (shares or dollars)"] size: f64,
    #[description = "Unit for size"] unit: Unit,
    #[description = "Limit price (shares only; blank = market)"] limit_price: Option<f64>,
) -> Result<(), Error> {
    place(ctx, OrderSide::Buy, symbol, size, unit, limit_price).await
}

/// Sell one of your Webull holdings — pick it from the list.
#[poise::command(slash_command, check = "owner_only")]
#[instrument(name = "cmd_sell", skip(ctx), fields(user_id = %ctx.author().id))]
pub async fn sell(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let Some(client) = ctx.data().webull.clone() else {
        ctx.say("Webull is not configured (no live access token).")
            .await?;
        return Ok(());
    };
    let account_id = match resolve_account_id(ctx.data(), &client).await {
        Ok(id) => id,
        Err(e) => {
            warn!(error = ?e, "could not resolve account");
            ctx.say(format!("Couldn't resolve a Webull account: {e}"))
                .await?;
            return Ok(());
        }
    };
    let positions = match client.account_positions(&account_id).await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = ?e, "account_positions failed");
            ctx.say(format!("Couldn't load positions: {e}")).await?;
            return Ok(());
        }
    };

    let holdings: Vec<_> = positions
        .into_iter()
        .filter(|p| p.quantity.parse::<f64>().is_ok_and(|q| q > 0.0))
        .collect();
    if holdings.is_empty() {
        ctx.say("You have no open positions to sell.").await?;
        return Ok(());
    }

    // Discord select menus cap at 25 options.
    let truncated = holdings.len() > 25;
    let options: Vec<serenity::CreateSelectMenuOption> = holdings
        .iter()
        .take(25)
        .map(|p| {
            let qty = fmt_num_str(&p.quantity);
            // Position market value (last price × quantity), for the $ prefill.
            let market_value = match (p.last_price.parse::<f64>(), p.quantity.parse::<f64>()) {
                (Ok(last), Ok(q)) => format!("{:.2}", last * q),
                _ => String::new(),
            };
            let mut desc = format!("last {}", p.last_price);
            if !market_value.is_empty() {
                desc.push_str(&format!(" · ≈ ${market_value}"));
            }
            if !p.unrealized_profit_loss.is_empty() {
                desc.push_str(&format!(" · P/L {}", p.unrealized_profit_loss));
            }
            desc.truncate(100);
            // Value carries symbol + held quantity + market value to prefill the
            // modal (shares → qty, dollars → market value).
            serenity::CreateSelectMenuOption::new(
                format!("{} — {} sh", p.symbol, qty),
                format!("{}|{}|{}", p.symbol, qty, market_value),
            )
            .description(desc)
        })
        .collect();

    let menu = serenity::CreateSelectMenu::new(
        SELL_SELECT_ID,
        serenity::CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a holding to sell");

    let content = if truncated {
        "Which holding do you want to sell? (showing the first 25)"
    } else {
        "Which holding do you want to sell?"
    };
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .components(vec![serenity::CreateActionRow::SelectMenu(menu)])
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Validate inputs, preview the order, and present the Confirm/Cancel buttons.
#[instrument(
    name = "cmd_buy",
    skip(ctx, size, unit, limit_price),
    fields(user_id = %ctx.author().id, side = %side, symbol = %symbol)
)]
async fn place(
    ctx: Context<'_>,
    side: OrderSide,
    symbol: String,
    size: f64,
    unit: Unit,
    limit_price: Option<f64>,
) -> Result<(), Error> {
    ctx.defer().await?;

    match prepare_order(
        ctx.data(),
        ctx.author().id.get(),
        side,
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
                    .components(vec![prepared.row]),
            )
            .await?;
        }
        Err(msg) => {
            ctx.say(msg).await?;
        }
    }

    Ok(())
}

/// A previewed order ready to show behind a Confirm/Cancel button row.
struct PreparedOrder {
    content: String,
    row: serenity::CreateActionRow,
}

/// Validate sizing, build and preview the order, stash it in `pending_trades`,
/// and return the confirmation message + button row. `Err(msg)` is a
/// user-facing reason it can't proceed (bad input, Webull unconfigured, etc.).
async fn prepare_order(
    data: &Data,
    user_id: u64,
    side: OrderSide,
    symbol: String,
    size: f64,
    unit: Unit,
    limit_price: Option<f64>,
) -> Result<PreparedOrder, String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err("Provide a ticker symbol.".to_string());
    }
    if size <= 0.0 || size.is_nan() {
        return Err("Size must be a positive number.".to_string());
    }
    if let Some(p) = limit_price {
        if p <= 0.0 {
            return Err("`limit_price` must be positive.".to_string());
        }
        if unit == Unit::Dollars {
            return Err(
                "Dollar-amount orders are market only — leave the limit price blank.".to_string(),
            );
        }
    }

    let Some(client) = data.webull.clone() else {
        return Err("Webull is not configured (no live access token).".to_string());
    };
    let account_id = resolve_account_id(data, &client)
        .await
        .map_err(|e| format!("Couldn't resolve a Webull account: {e}"))?;

    let order_type = if limit_price.is_some() {
        OrderType::Limit
    } else {
        OrderType::Market
    };
    let entrust_type = match unit {
        Unit::Shares => EntrustType::Qty,
        Unit::Dollars => EntrustType::Amount,
    };

    let client_order_id = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
    let mut order = NewOrder::new(
        client_order_id,
        &symbol,
        side,
        order_type,
        entrust_type,
        TimeInForce::Day,
    );
    order.support_trading_session = Some(TradingSession::Core);
    match unit {
        Unit::Shares => order.quantity = Some(size.to_string()),
        Unit::Dollars => order.total_cash_amount = Some(size.to_string()),
    }
    if let Some(p) = limit_price {
        order.limit_price = Some(p.to_string());
    }

    let request = OrderRequest {
        account_id: account_id.clone(),
        new_orders: vec![order.clone()],
    };
    let preview_line = match client.preview_order(&request, Category::UsStock).await {
        Ok(p) => {
            let mut parts = Vec::new();
            if !p.estimated_cost.is_empty() {
                parts.push(format!("est. cost ${}", p.estimated_cost));
            }
            if !p.estimated_transaction_fee.is_empty() {
                parts.push(format!("fee ${}", p.estimated_transaction_fee));
            }
            if parts.is_empty() {
                "Preview: no estimate returned.".to_string()
            } else {
                format!("Preview: {}", parts.join(" · "))
            }
        }
        Err(e) => {
            warn!(error = ?e, "preview_order failed");
            "Preview unavailable.".to_string()
        }
    };

    let sizing_str = match unit {
        Unit::Shares => format!("{size} sh"),
        Unit::Dollars => format!("${size}"),
    };
    let price_str = match limit_price {
        Some(p) => format!("LIMIT @ {p}"),
        None => "MARKET".to_string(),
    };
    let summary =
        format!("**{side} {symbol}** — {sizing_str} · {price_str} · DAY\nAccount `{account_id}`");

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

    let confirm_style = if side == OrderSide::Sell {
        serenity::ButtonStyle::Danger
    } else {
        serenity::ButtonStyle::Success
    };
    let row = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("{TRADE_CONFIRM_PREFIX}{req_id}"))
            .label("Confirm")
            .style(confirm_style),
        serenity::CreateButton::new(format!("{TRADE_CANCEL_PREFIX}{req_id}"))
            .label("Cancel")
            .style(serenity::ButtonStyle::Secondary),
    ]);

    Ok(PreparedOrder {
        content: format!("{summary}\n{preview_line}"),
        row,
    })
}

/// The account to trade in: `WEBULL_ACCOUNT_ID` if set, else the first account.
async fn resolve_account_id(data: &Data, client: &WebullClient) -> Result<String, Error> {
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

/// Edit the button message in place, clearing the buttons.
async fn update_message(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    content: &str,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content(content)
                    .components(vec![]),
            ),
        )
        .await?;
    Ok(())
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

/// Reply with a fresh ephemeral message, leaving the original untouched.
async fn respond_message(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    content: &str,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

/// Holding picked from the `/stock sell` dropdown: offer Shares/Dollars buttons.
#[instrument(name = "sell_select", skip(ctx, data, interaction), fields(user_id = %interaction.user.id))]
pub async fn handle_sell_select(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    if data.owner_id != Some(interaction.user.id.get()) {
        return respond_message(ctx, interaction, "❌ Only the bot owner can trade.").await;
    }

    let picked = match &interaction.data.kind {
        serenity::ComponentInteractionDataKind::StringSelect { values } => values.first().cloned(),
        _ => None,
    };
    let Some(picked) = picked else {
        return respond_message(ctx, interaction, "No holding selected.").await;
    };
    // Value is "SYMBOL|QTY" (see `sell`); carried through so the modal can
    // prefill the held quantity when sizing by shares.
    let (symbol, _) = picked.split_once('|').unwrap_or((picked.as_str(), ""));

    let row = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("{SELL_UNIT_PREFIX}shares_{picked}"))
            .label("Shares")
            .style(serenity::ButtonStyle::Primary),
        serenity::CreateButton::new(format!("{SELL_UNIT_PREFIX}dollars_{picked}"))
            .label("Dollars")
            .style(serenity::ButtonStyle::Primary),
    ]);

    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("Sell **{symbol}** — size by shares or dollars?"))
                    .components(vec![row]),
            ),
        )
        .await?;
    Ok(())
}

/// Unit button pressed: open the sell modal for the number (+ limit for shares).
#[instrument(name = "sell_unit", skip(ctx, data, interaction), fields(user_id = %interaction.user.id, rest = %rest))]
pub async fn handle_sell_unit(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    rest: &str,
) -> Result<(), Error> {
    if data.owner_id != Some(interaction.user.id.get()) {
        return respond_message(ctx, interaction, "❌ Only the bot owner can trade.").await;
    }

    // `rest` is "<unit>_<symbol>|<qty>|<market_value>".
    let (unit_token, payload) = rest.split_once('_').unwrap_or((rest, ""));
    let Some(unit) = unit_from_token(unit_token) else {
        return respond_message(ctx, interaction, "Invalid unit.").await;
    };
    // Prefill the held quantity (shares) or the position's dollar value (dollars).
    let (symbol, prefill) = sell_prefill(payload, unit);
    info!(unit = unit.token(), symbol = %symbol, prefill = %prefill, "opening sell modal");

    let mut size_input = serenity::CreateInputText::new(
        serenity::InputTextStyle::Short,
        match unit {
            Unit::Shares => "Shares",
            Unit::Dollars => "Amount ($)",
        },
        "size",
    )
    .required(true);
    if !prefill.is_empty() {
        size_input = size_input.value(prefill);
    }

    let mut rows = vec![serenity::CreateActionRow::InputText(size_input)];
    // Dollar (notional) orders are market-only, so only shares get a limit field.
    if unit == Unit::Shares {
        rows.push(serenity::CreateActionRow::InputText(
            serenity::CreateInputText::new(
                serenity::InputTextStyle::Short,
                "Limit price (blank = market)",
                "limit",
            )
            .required(false),
        ));
    }

    let modal = serenity::CreateModal::new(
        format!("{SELL_MODAL_PREFIX}{}_{symbol}", unit.token()),
        format!("Sell {symbol}"),
    )
    .components(rows);

    interaction
        .create_response(ctx, serenity::CreateInteractionResponse::Modal(modal))
        .await?;
    Ok(())
}

/// Sell modal submitted: parse size/unit/limit, preview, and present the
/// Confirm/Cancel buttons.
#[instrument(name = "sell_modal", skip(ctx, data, interaction), fields(user_id = %interaction.user.id))]
pub async fn handle_sell_modal(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ModalInteraction,
) -> Result<(), Error> {
    if data.owner_id != Some(interaction.user.id.get()) {
        return modal_reply(ctx, interaction, "❌ Only the bot owner can trade.").await;
    }

    // Custom id is "<unit>_<symbol>".
    let rest = interaction
        .data
        .custom_id
        .strip_prefix(SELL_MODAL_PREFIX)
        .unwrap_or_default();
    let (unit_token, symbol) = rest.split_once('_').unwrap_or((rest, ""));
    let Some(unit) = unit_from_token(unit_token) else {
        return modal_reply(ctx, interaction, "Invalid order.").await;
    };
    let symbol = symbol.to_string();

    let size = match modal_input(interaction, "size").trim().parse::<f64>() {
        Ok(n) if n > 0.0 => n,
        _ => return modal_reply(ctx, interaction, "Size must be a positive number.").await,
    };
    // Only shares carry a limit field; dollar orders are market-only.
    let limit_price = if unit == Unit::Shares {
        match modal_input(interaction, "limit").trim() {
            "" => None,
            s => match s.parse::<f64>() {
                Ok(p) if p > 0.0 => Some(p),
                _ => {
                    return modal_reply(
                        ctx,
                        interaction,
                        "Limit price must be a positive number, or blank for a market order.",
                    )
                    .await;
                }
            },
        }
    } else {
        None
    };

    // Ack the modal within 3s, then preview (network) and follow up with the
    // Confirm/Cancel buttons.
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Defer(
                serenity::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await?;

    let followup = match prepare_order(
        data,
        interaction.user.id.get(),
        OrderSide::Sell,
        symbol,
        size,
        unit,
        limit_price,
    )
    .await
    {
        Ok(prepared) => serenity::CreateInteractionResponseFollowup::new()
            .content(prepared.content)
            .components(vec![prepared.row])
            .ephemeral(true),
        Err(msg) => serenity::CreateInteractionResponseFollowup::new()
            .content(msg)
            .ephemeral(true),
    };
    interaction.create_followup(ctx, followup).await?;
    Ok(())
}

/// Read a modal text input's value by its custom id.
fn modal_input(interaction: &serenity::ModalInteraction, id: &str) -> String {
    interaction
        .data
        .components
        .iter()
        .flat_map(|row| &row.components)
        .find_map(|c| match c {
            serenity::ActionRowComponent::InputText(input) if input.custom_id == id => {
                input.value.clone()
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// Reply to a modal submit with a fresh ephemeral message.
async fn modal_reply(
    ctx: &serenity::Context,
    interaction: &serenity::ModalInteraction,
    content: &str,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

/// Format a numeric string from Webull (e.g. `"10.00000000"`) compactly, falling
/// back to the trimmed original if it isn't a number.
fn fmt_num_str(raw: &str) -> String {
    raw.parse::<f64>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| raw.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sell_prefill_picks_qty_or_market_value() {
        // Live-shaped payload: symbol | held qty | position $ value.
        assert_eq!(
            sell_prefill("SFL|0.88339|10.95", Unit::Shares),
            ("SFL", "0.88339")
        );
        assert_eq!(
            sell_prefill("SFL|0.88339|10.95", Unit::Dollars),
            ("SFL", "10.95")
        );
        // Missing/blank market value → no dollar prefill (not garbage).
        assert_eq!(sell_prefill("AAPL|1|", Unit::Dollars), ("AAPL", ""));
        assert_eq!(sell_prefill("AAPL|1", Unit::Dollars), ("AAPL", ""));
    }

    #[test]
    fn modal_input_serializes_prefill_value() {
        // Proves serenity puts the prefill on the wire as `value`.
        let input =
            serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Amount ($)", "size")
                .value("10.95");
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["custom_id"], "size");
        assert_eq!(json["value"], "10.95");
    }
}
