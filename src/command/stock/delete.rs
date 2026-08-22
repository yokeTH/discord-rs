//! `/stock delete` — pick symbols off the channel watchlist, then confirm.
//!
//! The command shows the picker; the select, confirm and cancel interactions it
//! spawns are handled here too (routed in [`crate::component`]).

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::bail;
use poise::serenity_prelude as serenity;
use serenity::all::{
    CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use tracing::{debug, error, info, instrument, warn};

use crate::component::{respond_message, update_components, update_message};
use crate::{Context, Data, Error};

/// Custom id of the watchlist picker.
pub const SELECT_DELETE_ID: &str = "select_delete";
/// Prefixes of the confirm/cancel buttons; the rest is the request id.
pub const CONFIRM_PREFIX: &str = "confirm_del_";
pub const CANCEL_PREFIX: &str = "cancel_del_";

#[poise::command(slash_command)]
#[instrument(name = "cmd_delete", skip(ctx), fields(user_id = %ctx.author().id))]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    debug!("deferred reply");

    let symbol_store = ctx.data().symbol_store.clone();
    let channel_id = ctx.channel_id().get() as i64;

    let symbols: Vec<String> = symbol_store.list(channel_id).await?;
    if symbols.is_empty() {
        info!("attempted delete from empty watchlist");
        bail!("Watchlist is empty.");
    }

    // Discord select menus cap at 25 options.
    let limit = symbols.len().min(25);

    let opts: Vec<CreateSelectMenuOption> = symbols
        .into_iter()
        .take(limit)
        .map(|sym: String| CreateSelectMenuOption::new(sym.clone(), sym))
        .collect();

    let menu = CreateSelectMenu::new(
        SELECT_DELETE_ID,
        CreateSelectMenuKind::String { options: opts },
    )
    .placeholder("Choose symbols...")
    .min_values(1)
    .max_values(limit as u8);

    info!(limit, "presenting symbols for deletion");

    ctx.send(
        poise::CreateReply::default()
            .content("Select symbols to delete (you can pick multiple):")
            .components(vec![CreateActionRow::SelectMenu(menu)]),
    )
    .await?;

    info!("sent selection menu");
    Ok(())
}

/// Symbols picked: stash them as a pending delete and ask for confirmation.
#[instrument(
    name = "delete_select",
    skip(ctx, data, interaction),
    fields(user_id = %interaction.user.id)
)]
pub async fn handle_select(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let values = match &interaction.data.kind {
        serenity::ComponentInteractionDataKind::StringSelect { values } => values.clone(),
        _ => vec![],
    };

    if values.is_empty() {
        debug!("empty selection submitted");
        return Ok(());
    }

    let user_id = interaction.user.id.get();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let req_id = format!("{user_id}-{ts}");

    data.symbol_store
        .set_pending_delete(req_id.clone(), user_id as i64, values.clone())
        .await?;

    info!(
        req_id = %req_id,
        count = values.len(),
        symbols = %values.join(", "),
        "initiated delete confirmation"
    );

    let row = CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("{CONFIRM_PREFIX}{req_id}"))
            .label("Confirm")
            .style(serenity::ButtonStyle::Danger),
        serenity::CreateButton::new(format!("{CANCEL_PREFIX}{req_id}"))
            .label("Cancel")
            .style(serenity::ButtonStyle::Secondary),
    ]);

    let msg = format!(
        "Are you sure you want to delete **{}** symbols?\n> {}",
        values.len(),
        values.join(", ")
    );

    update_components(ctx, interaction, &msg, vec![row]).await?;
    debug!(req_id = %req_id, "updated message to confirmation UI");

    Ok(())
}

/// Cancel button: mark the request cancelled and clear the buttons.
#[instrument(name = "delete_cancel", skip(ctx, data, interaction), fields(req_id = %req_id))]
pub async fn handle_cancel(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    req_id: &str,
) -> Result<(), Error> {
    info!("cancelled delete operation");

    if let Err(e) = data
        .symbol_store
        .cancel_pending_delete(req_id.to_string())
        .await
    {
        error!(error = ?e, "failed to mark pending delete cancelled");
    }

    update_message(ctx, interaction, "Cancelled.").await
}

/// Confirm button: remove the stashed symbols from the channel's watchlist.
#[instrument(
    name = "delete_confirm",
    skip(ctx, data, interaction),
    fields(req_id = %req_id, user_id = %interaction.user.id)
)]
pub async fn handle_confirm(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
    req_id: &str,
) -> Result<(), Error> {
    // The request id is "<user_id>-<ts>", so only its author may confirm it.
    if let Some(owner) = req_id.split('-').next()
        && owner != interaction.user.id.get().to_string()
    {
        warn!(owner = %owner, "attempted to confirm someone else's delete");
        return respond_message(
            ctx,
            interaction,
            "❌ You can't confirm someone else's delete.",
        )
        .await;
    }

    let Some(symbols) = data
        .symbol_store
        .get_pending_delete(req_id.to_string())
        .await?
    else {
        warn!("session expired or not found");
        return respond_message(ctx, interaction, "❌ Session expired. Run /delete again.").await;
    };

    info!(
        count = symbols.len(),
        symbols = %symbols.join(", "),
        "confirmed deletion"
    );

    let channel_id = interaction.channel_id.get() as i64;
    for sym in &symbols {
        match data.symbol_store.remove(channel_id, sym).await {
            Ok(_) => info!(symbol = %sym, "deleted symbol"),
            Err(e) => error!(symbol = %sym, error = ?e, "failed to delete symbol"),
        }
    }

    if let Err(e) = data
        .symbol_store
        .confirm_pending_delete(req_id.to_string())
        .await
    {
        error!(error = ?e, "failed to mark pending delete confirmed");
    }

    update_message(
        ctx,
        interaction,
        &format!("{} was deleted.", symbols.join(", ")),
    )
    .await
}
