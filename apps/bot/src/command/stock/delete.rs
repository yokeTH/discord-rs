use ::serenity::all::{
    CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use anyhow::bail;
use poise::serenity_prelude as serenity;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, instrument, warn};

use crate::{Context, Data, Error};

const SELECT_DELETE_ID: &str = "select_delete";
const CONFIRM_PREFIX: &str = "confirm_del_";
const CANCEL_ID: &str = "cancel_del";

#[poise::command(slash_command)]
#[instrument(name = "cmd_delete", skip(ctx), fields(user_id = %ctx.author().id))]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    debug!("deferred reply");

    let symbol_store = ctx.data().symbol_store.clone();

    let symbols: Vec<String> = symbol_store.list().await?;
    if symbols.is_empty() {
        info!("attempted delete from empty watchlist");
        bail!("Watchlist is empty.");
    }

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

    let components = vec![CreateActionRow::SelectMenu(menu)];

    info!(limit, "presenting symbols for deletion");

    ctx.send(
        poise::CreateReply::default()
            .content("Select symbols to delete (you can pick multiple):")
            .components(components),
    )
    .await?;

    info!("sent selection menu");
    Ok(())
}

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
            .set_pending_delete(req_id.clone(), values.clone())
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
            serenity::CreateButton::new(CANCEL_ID)
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

    if id == CANCEL_ID {
        info!("cancelled delete operation");

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
                            .content("❌ You can’t confirm someone else’s delete.")
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
        for sym in &symbols {
            match data.symbol_store.remove(sym).await {
                Ok(_) => info!(symbol = %sym, "deleted symbol"),
                Err(e) => error!(symbol = %sym, error = ?e, "failed to delete symbol"),
            }
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
