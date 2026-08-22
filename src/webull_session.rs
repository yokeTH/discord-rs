//! Webull access-token bootstrap.
//!
//! A Webull token must be created from the app key/secret and then verified via
//! an SMS code **entered in the Webull mobile app** (the OpenAPI has no endpoint
//! to submit the code). So the bot can't collect the OTP itself — instead it
//! creates the token, DMs the owner the in-app verification steps, and polls
//! [`WebullClient::check_token`] until the token flips to `NORMAL`. The token is
//! held in memory only; a restart with no `WEBULL_ACCESS_TOKEN` re-issues one. A
//! freshly activated token is logged as `WEBULL_ACCESS_TOKEN=…` so it can be
//! pinned in `.env` to skip re-verifying after a restart.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serenity::all::{Http, UserId};
use tokio::time::{Instant, sleep};
use tracing::{info, instrument, warn};
use webull::{TokenStatus, WebullClient};

/// How long Webull allows for SMS verification of a freshly created token.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// How often to poll the token's status while waiting for verification.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

const VERIFY_STEPS: &str = "🔐 **Webull verification needed**\n\
    Webull just sent an SMS code to your phone. Within **5 minutes**, open the Webull app:\n\
    **Menu → Messages → OpenAPI Notifications → (select the request) → Check Now → enter the SMS code**\n\
    I'll confirm here once it's verified.";

/// Spawn the startup bootstrap: keep a configured token if it's still valid,
/// otherwise issue and verify a new one.
pub fn spawn_bootstrap(client: Arc<WebullClient>, http: Arc<Http>, owner_id: Option<u64>) {
    tokio::spawn(async move {
        if let Err(e) = bootstrap(&client, &http, owner_id).await {
            warn!(error = ?e, "webull bootstrap failed");
        }
    });
}

/// Spawn a fresh token issue + verify (used by `/webull login`).
pub fn spawn_issue(client: Arc<WebullClient>, http: Arc<Http>, owner_id: Option<u64>) {
    tokio::spawn(async move {
        if let Err(e) = issue(&client, &http, owner_id).await {
            warn!(error = ?e, "webull token issue failed");
        }
    });
}

#[instrument(name = "webull_bootstrap", skip_all)]
async fn bootstrap(client: &WebullClient, http: &Arc<Http>, owner_id: Option<u64>) -> Result<()> {
    // Keep an already-configured (env) token if it's still usable.
    if let Some(token) = client.access_token() {
        match client.check_token(&token).await {
            Ok(t) if t.status == TokenStatus::Normal => {
                info!("configured webull token is valid; trading enabled");
                return Ok(());
            }
            Ok(t) => warn!(status = ?t.status, "configured webull token unusable; re-issuing"),
            Err(e) => warn!(error = ?e, "could not verify configured webull token; re-issuing"),
        }
    }
    issue(client, http, owner_id).await.map(|_| ())
}

/// Create a token, DM the owner the verification steps, and poll until it is
/// `NORMAL`. On success the token is stored on `client`. Returns the final
/// status reached.
#[instrument(name = "webull_issue", skip_all)]
pub async fn issue(
    client: &WebullClient,
    http: &Arc<Http>,
    owner_id: Option<u64>,
) -> Result<TokenStatus> {
    let Some(owner) = owner_id else {
        warn!("OWNER_ID unset; cannot run Webull 2FA bootstrap");
        return Ok(TokenStatus::Pending);
    };

    info!("creating webull token");
    let created = client.create_token().await?;

    // UAT auto-activates; production returns PENDING and needs SMS 2FA.
    if created.status == TokenStatus::Normal {
        store_and_announce(client, &created.token, created.expires);
        dm(
            http,
            owner,
            "✅ Webull token issued and active. Trading is enabled.",
        )
        .await;
        return Ok(TokenStatus::Normal);
    }

    dm(http, owner, VERIFY_STEPS).await;

    let deadline = Instant::now() + VERIFY_TIMEOUT;
    loop {
        sleep(POLL_INTERVAL).await;
        match client.check_token(&created.token).await?.status {
            TokenStatus::Normal => {
                store_and_announce(client, &created.token, created.expires);
                dm(http, owner, "✅ Webull verified. Trading is enabled.").await;
                return Ok(TokenStatus::Normal);
            }
            TokenStatus::Pending => {
                if Instant::now() >= deadline {
                    warn!("webull verification window elapsed");
                    dm(
                        http,
                        owner,
                        "⌛ Verification window (5 min) elapsed. Run `/webull login` to try again.",
                    )
                    .await;
                    return Ok(TokenStatus::Pending);
                }
            }
            other => {
                warn!(status = ?other, "webull token verification failed");
                dm(
                    http,
                    owner,
                    &format!("❌ Token became {other:?}. Run `/webull login` to try again."),
                )
                .await;
                return Ok(other);
            }
        }
    }
}

/// Store the token on the client and print it so it can be pinned in `.env`
/// (`WEBULL_ACCESS_TOKEN=…`) to skip re-verification after a restart.
fn store_and_announce(client: &WebullClient, token: &str, expires: i64) {
    client.set_access_token(token.to_string());
    info!(
        expires,
        "webull token active — pin it in .env to reuse across restarts: WEBULL_ACCESS_TOKEN={token}"
    );
}

/// Best-effort DM to the owner; logs on failure rather than propagating.
async fn dm(http: &Arc<Http>, owner: u64, content: &str) {
    match UserId::new(owner).create_dm_channel(http).await {
        Ok(channel) => {
            if let Err(e) = channel.say(http, content).await {
                warn!(error = ?e, "failed to DM owner");
            }
        }
        Err(e) => warn!(error = ?e, "failed to open owner DM channel"),
    }
}
