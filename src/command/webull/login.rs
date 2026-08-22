//! `/webull login` — (re)issue a Webull access token and DM the owner the
//! verification steps.

use tracing::{info, instrument};

use crate::auth::owner_only;
use crate::webull_session;
use crate::{Context, Error};

/// Issue a fresh Webull token and DM you the SMS verification steps.
#[poise::command(slash_command, check = "owner_only")]
#[instrument(name = "cmd_webull_login", skip(ctx), fields(user_id = %ctx.author().id))]
pub async fn login(ctx: Context<'_>) -> Result<(), Error> {
    let Some(client) = ctx.data().webull.clone() else {
        ctx.send(
            poise::CreateReply::default()
                .content("Webull is not configured (set WEBULL_APP_KEY / WEBULL_APP_SECRET).")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    // Token verification can take up to 5 minutes, so run it in the background
    // and report progress via DM rather than blocking the interaction.
    let http = ctx.serenity_context().http.clone();
    webull_session::spawn_issue(client, http, ctx.data().owner_id);
    info!("spawned webull token issue");

    ctx.send(
        poise::CreateReply::default()
            .content("Issuing a Webull token — check your DMs for the verification steps.")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
