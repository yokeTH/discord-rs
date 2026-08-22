//! Shared command checks.

use crate::{Context, Error};

/// Poise check gating a command to the configured owner (`OWNER_ID`). Replies
/// with an ephemeral refusal when someone else invokes it.
pub async fn owner_only(ctx: Context<'_>) -> Result<bool, Error> {
    if ctx.data().owner_id == Some(ctx.author().id.get()) {
        return Ok(true);
    }
    ctx.send(
        poise::CreateReply::default()
            .content("❌ Only the bot owner can do that.")
            .ephemeral(true),
    )
    .await?;
    Ok(false)
}
