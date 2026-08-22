use poise::serenity_prelude as serenity;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, instrument, warn};

use crate::command::stock::trade::{self, TRADE_CANCEL_PREFIX, TRADE_CONFIRM_PREFIX};
use crate::{Data, Error};

pub const SELECT_DELETE_ID: &str = "select_delete";
pub const CONFIRM_PREFIX: &str = "confirm_del_";
pub const CANCEL_PREFIX: &str = "cancel_del_";

#[instrument(
    name = "component_delete",
    skip(ctx, data, interaction),
    fields(custom_id = %interaction.data.custom_id, user_id = %interaction.user.id)
)]
pub async fn handle_component(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let id = interaction.data.custom_id.as_str();

    if let Some(req_id) = id.strip_prefix(TRADE_CONFIRM_PREFIX) {
        return trade::confirm(ctx, data, interaction, req_id).await;
    }

    if let Some(req_id) = id.strip_prefix(TRADE_CANCEL_PREFIX) {
        return trade::cancel(ctx, data, interaction, req_id).await;
    }

    if id == trade::SELL_SELECT_ID {
        return trade::handle_sell_select(ctx, data, interaction).await;
    }

    if let Some(rest) = id.strip_prefix(trade::SELL_UNIT_PREFIX) {
        return trade::handle_sell_unit(ctx, data, interaction, rest).await;
    }

    if id == SELECT_DELETE_ID {
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

        let msg = format!(
            "Are you sure you want to delete **{}** symbols?\n> {}",
            values.len(),
            values.join(", ")
        );

        let row = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(format!("{CONFIRM_PREFIX}{req_id}"))
                .label("Confirm")
                .style(serenity::ButtonStyle::Danger),
            serenity::CreateButton::new(format!("{CANCEL_PREFIX}{req_id}"))
                .label("Cancel")
                .style(serenity::ButtonStyle::Secondary),
        ]);

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(msg)
                        .components(vec![row]),
                ),
            )
            .await?;

        debug!(req_id = %req_id, "updated message to confirmation UI");
        return Ok(());
    }

    if let Some(req_id) = id.strip_prefix(CANCEL_PREFIX) {
        info!(req_id = %req_id, "cancelled delete operation");

        if let Err(e) = data
            .symbol_store
            .cancel_pending_delete(req_id.to_string())
            .await
        {
            error!(req_id = %req_id, error = ?e, "failed to mark pending delete cancelled");
        }

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("Cancelled.")
                        .components(vec![]),
                ),
            )
            .await?;

        return Ok(());
    }

    if let Some(req_id) = id.strip_prefix(CONFIRM_PREFIX) {
        if let Some(owner) = req_id.split('-').next()
            && owner != interaction.user.id.get().to_string()
        {
            warn!(owner = %owner, req_id = %req_id, "attempted to confirm request");

            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("❌ You can't confirm someone else's delete.")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }

        let symbols: Vec<String> = match data
            .symbol_store
            .get_pending_delete(req_id.to_string())
            .await?
        {
            Some(s) => s,
            None => {
                warn!(req_id = %req_id, "session expired or not found");

                interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("❌ Session expired. Run /delete again.")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
        };

        info!(
            req_id = %req_id,
            count = symbols.len(),
            symbols = %symbols.join(", "),
            "confirmed deletion"
        );

        // delete each symbol
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
            error!(req_id = %req_id, error = ?e, "failed to mark pending delete confirmed");
        }

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("{} was deleted.", symbols.join(", ")))
                        .components(vec![]),
                ),
            )
            .await?;

        debug!("updated message to final result");
        return Ok(());
    }

    debug!("ignored unrelated component interaction");
    Ok(())
}

/// Route a submitted modal (currently just the `/stock sell` quantity modal).
#[instrument(
    name = "handle_modal",
    skip(ctx, data, interaction),
    fields(custom_id = %interaction.data.custom_id, user_id = %interaction.user.id)
)]
pub async fn handle_modal(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ModalInteraction,
) -> Result<(), Error> {
    if interaction
        .data
        .custom_id
        .starts_with(trade::SELL_MODAL_PREFIX)
    {
        return trade::handle_sell_modal(ctx, data, interaction).await;
    }

    debug!("ignored unrelated modal interaction");
    Ok(())
}
