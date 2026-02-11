use ::serenity::all::{
    CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use anyhow::bail;
use log::{debug, error, info, warn};
use poise::serenity_prelude as serenity;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Context, Data, Error};

const SELECT_DELETE_ID: &str = "select_delete";
const CONFIRM_PREFIX: &str = "confirm_del_";
const CANCEL_ID: &str = "cancel_del";

#[poise::command(slash_command)]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let symbol_store = ctx.data().symbol_store.clone();

    let symbols: Vec<String> = symbol_store.list().await?;
    if symbols.is_empty() {
        info!(
            "User {} attempted to delete from an empty watchlist.",
            ctx.author().id
        );
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

    info!(
        "User {} invoked /delete. Presenting {} symbols for deletion.",
        ctx.author().id,
        limit
    );

    ctx.send(
        poise::CreateReply::default()
            .content("Select symbols to delete (you can pick multiple):")
            .components(components),
    )
    .await?;

    Ok(())
}

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
            debug!(
                "User {} submitted an empty selection for deletion.",
                interaction.user.id
            );
            return Ok(());
        }

        let user_id = interaction.user.id.get();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let req_id = format!("{user_id}-{ts}");

        let _ = data
            .symbol_store
            .set_pending_delete(req_id.to_string(), values.clone())
            .await?;

        info!(
            "User {} initiated delete confirmation for symbols: [{}]",
            user_id,
            values.join(", ")
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

        return Ok(());
    }

    if id == CANCEL_ID {
        info!(
            "User {} cancelled the delete operation.",
            interaction.user.id
        );
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
            warn!(
                "User {} attempted to confirm deletion for someone else's request (owner: {}).",
                interaction.user.id, owner
            );
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
                warn!(
                    "User {} tried to confirm deletion, but session expired or not found (req_id: {}).",
                    interaction.user.id, req_id
                );
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

        for sym in &symbols {
            match data.symbol_store.remove(sym).await {
                Ok(_) => info!("User {} deleted symbol '{}'.", interaction.user.id, sym),
                Err(e) => error!(
                    "Error deleting symbol '{}' for user {}: {:?}",
                    sym, interaction.user.id, e
                ),
            }
        }

        info!(
            "User {} confirmed deletion of symbols: [{}]",
            interaction.user.id,
            symbols.join(", ")
        );

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

        return Ok(());
    }

    Ok(())
}
