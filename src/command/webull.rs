//! `/webull` commands — currently just `login`, which (re)issues a Webull access
//! token and DMs the owner the verification steps.

use crate::command::auth::owner_only;
use crate::webull_session;
use crate::{Context, Error};

use tracing::{info, instrument};

#[poise::command(slash_command, rename = "webull", subcommands("login"))]
pub async fn webull_command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Issue a fresh Webull token and DM you the SMS verification steps.
#[poise::command(slash_command, check = "owner_only")]
#[instrument(name = "cmd_webull_login", skip(ctx), fields(user_id = %ctx.author().id))]
async fn login(ctx: Context<'_>) -> Result<(), Error> {
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
