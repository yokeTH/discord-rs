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
    let user_id = ctx.author().id.get();

    info!("delete: invoked user_id={}", user_id);

    let symbols: Vec<String> = symbol_store.list().await?;
    if symbols.is_empty() {
        info!("delete: watchlist empty user_id={}", user_id);
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
        "delete: presenting options user_id={} count={}",
        user_id, limit
    );

    ctx.send(
        poise::CreateReply::default()
            .content("Select symbols to delete (you can pick multiple):")
            .components(components),
    )
    .await?;

    info!("delete: message sent user_id={}", user_id);
    Ok(())
}

pub async fn handle_component(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let id = interaction.data.custom_id.as_str();
    let user_id = interaction.user.id.get();

    debug!(
        "delete: component received user_id={} custom_id={}",
        user_id, id
    );

    if id == SELECT_DELETE_ID {
        let values = match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => values.clone(),
            _ => vec![],
        };

        if values.is_empty() {
            debug!("delete: empty selection user_id={}", user_id);
            return Ok(());
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let req_id = format!("{user_id}-{ts}");

        data.symbol_store
            .set_pending_delete(req_id.to_string(), values.clone())
            .await?;

        info!(
            "delete: confirmation created user_id={} req_id={} symbols=[{}]",
            user_id,
            req_id,
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

        debug!(
            "delete: confirmation prompt shown user_id={} req_id={}",
            user_id, req_id
        );
        return Ok(());
    }

    if id == CANCEL_ID {
        info!("delete: cancelled user_id={}", user_id);

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

        debug!("delete: cancel response sent user_id={}", user_id);
        return Ok(());
    }

    if let Some(req_id) = id.strip_prefix(CONFIRM_PREFIX) {
        let owner = req_id.split('-').next().unwrap_or_default();

        if owner != user_id.to_string() {
            warn!(
                "delete: confirm denied user_id={} req_id={} owner={}",
                user_id, req_id, owner
            );

            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("You can’t confirm someone else’s delete.")
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
                    "delete: confirm expired user_id={} req_id={}",
                    user_id, req_id
                );

                interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("Session expired. Run /delete again.")
                                .ephemeral(true),
                        ),
                    )
                    .await?;

                return Ok(());
            }
        };

        info!(
            "delete: confirm accepted user_id={} req_id={} count={} symbols=[{}]",
            user_id,
            req_id,
            symbols.len(),
            symbols.join(", ")
        );

        let mut ok = 0usize;
        let mut fail = 0usize;

        for sym in &symbols {
            match data.symbol_store.remove(sym).await {
                Ok(_) => {
                    ok += 1;
                    debug!("delete: removed user_id={} symbol={}", user_id, sym);
                }
                Err(e) => {
                    fail += 1;
                    error!(
                        "delete: remove failed user_id={} symbol={} err={:?}",
                        user_id, sym, e
                    );
                }
            }
        }

        info!(
            "delete: completed user_id={} req_id={} ok={} fail={}",
            user_id, req_id, ok, fail
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

        debug!(
            "delete: final response sent user_id={} req_id={}",
            user_id, req_id
        );
        return Ok(());
    }

    debug!(
        "delete: ignored component user_id={} custom_id={}",
        user_id, id
    );
    Ok(())
}
