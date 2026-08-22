//! Interaction router: dispatches component and modal interactions to the
//! command module that owns them, and holds the response helpers they share.

use poise::serenity_prelude as serenity;
use tracing::{debug, instrument};

use crate::command::stock::delete;
use crate::command::webull::{self, sell};
use crate::{Data, Error};

#[instrument(
    name = "handle_component",
    skip(ctx, data, interaction),
    fields(custom_id = %interaction.data.custom_id, user_id = %interaction.user.id)
)]
pub async fn handle_component(
    ctx: &serenity::Context,
    data: &Data,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let id = interaction.data.custom_id.as_str();

    if let Some(req_id) = id.strip_prefix(webull::TRADE_CONFIRM_PREFIX) {
        return webull::confirm(ctx, data, interaction, req_id).await;
    }

    if let Some(req_id) = id.strip_prefix(webull::TRADE_CANCEL_PREFIX) {
        return webull::cancel(ctx, data, interaction, req_id).await;
    }

    if id == sell::SELL_SELECT_ID {
        return sell::handle_sell_select(ctx, data, interaction).await;
    }

    if let Some(rest) = id.strip_prefix(sell::SELL_UNIT_PREFIX) {
        return sell::handle_sell_unit(ctx, data, interaction, rest).await;
    }

    if id == delete::SELECT_DELETE_ID {
        return delete::handle_select(ctx, data, interaction).await;
    }

    if let Some(req_id) = id.strip_prefix(delete::CANCEL_PREFIX) {
        return delete::handle_cancel(ctx, data, interaction, req_id).await;
    }

    if let Some(req_id) = id.strip_prefix(delete::CONFIRM_PREFIX) {
        return delete::handle_confirm(ctx, data, interaction, req_id).await;
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
        .starts_with(sell::SELL_MODAL_PREFIX)
    {
        return sell::handle_sell_modal(ctx, data, interaction).await;
    }

    debug!("ignored unrelated modal interaction");
    Ok(())
}

/// Edit the interaction's message in place, clearing its components.
pub(crate) async fn update_message(
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

/// Edit the interaction's message in place, replacing its components.
pub(crate) async fn update_components(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    content: &str,
    components: Vec<serenity::CreateActionRow>,
) -> Result<(), Error> {
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::new()
                    .content(content)
                    .components(components),
            ),
        )
        .await?;
    Ok(())
}

/// Reply with a fresh ephemeral message, leaving the original untouched.
pub(crate) async fn respond_message(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `handle_component` dispatches on prefixes, so every custom id must match
    /// exactly one branch. An id that shadows another silently steals its
    /// interactions — the buttons just stop working.
    #[test]
    fn custom_id_namespaces_are_disjoint() {
        let prefixes = [
            webull::TRADE_CONFIRM_PREFIX,
            webull::TRADE_CANCEL_PREFIX,
            sell::SELL_UNIT_PREFIX,
            delete::CONFIRM_PREFIX,
            delete::CANCEL_PREFIX,
        ];
        let exact = [sell::SELL_SELECT_ID, delete::SELECT_DELETE_ID];

        for (i, a) in prefixes.iter().enumerate() {
            for b in prefixes.iter().skip(i + 1) {
                assert!(
                    !a.starts_with(b) && !b.starts_with(a),
                    "prefixes {a:?} and {b:?} overlap"
                );
            }
            for id in exact {
                assert!(!id.starts_with(a), "exact id {id:?} is shadowed by {a:?}");
            }
        }

        assert_ne!(exact[0], exact[1]);
    }
}
