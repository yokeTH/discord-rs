//! `/webull sell` — sell one of the account's current holdings.
//!
//! Unlike buy, the symbol isn't typed: the command shows a dropdown of live
//! holdings, picking one shows Shares/Dollars buttons (a modal can't hold a
//! dropdown), and the chosen unit opens a modal for the number (plus a limit for
//! shares). The submitted modal then joins the shared preview + Confirm flow in
//! [`super`].

use poise::serenity_prelude as serenity;
use tracing::{info, instrument, warn};
use webull::OrderSide;

use crate::auth::owner_only;
use crate::command::webull::{prepare_order, resolve_account_id};
use crate::component::respond_message;
use crate::order::Unit;
use crate::{Context, Data, Error};

/// Custom id of the holdings dropdown.
pub const SELL_SELECT_ID: &str = "sell_select";
/// Prefix of the unit buttons shown after a holding is picked; the rest is
/// `<unit>_<symbol>|<qty>|<market_value>`.
pub const SELL_UNIT_PREFIX: &str = "sell_unit_";
/// Prefix of the sell modal's custom id; the rest is `<unit>_<symbol>`.
pub const SELL_MODAL_PREFIX: &str = "sell_modal_";

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

/// Holding picked from the dropdown: offer Shares/Dollars buttons.
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
    let Some(unit) = Unit::from_token(unit_token) else {
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
    let Some(unit) = Unit::from_token(unit_token) else {
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
