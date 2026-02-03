use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    ctx.say("Unimplemented").await?;

    Ok(())
}
