//! Background task that keeps the Webull access token alive.
//!
//! Webull tokens expire ~15 days after their last use, but can be extended
//! without repeating 2FA via the refresh endpoint. This task refreshes the
//! in-memory token on an interval so it never lapses while the bot is running.
//! (If the bot is down longer than the expiry window, the token goes invalid
//! and must be re-issued with the `create_token` example.)

use std::sync::Arc;
use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};
use webull::WebullClient;

/// How often to refresh — comfortably within Webull's ~15-day expiry.
const REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

/// Spawn the refresh loop. The first refresh runs immediately, doubling as a
/// startup validity check for the configured token.
pub fn spawn(client: Arc<WebullClient>) {
    tokio::spawn(async move {
        let mut ticker = interval(REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match client.refresh_and_store().await {
                Ok(t) => info!(status = ?t.status, expires = t.expires, "webull token refreshed"),
                Err(e) => warn!(
                    error = ?e,
                    "webull token refresh failed; re-run the create_token example if it expired",
                ),
            }
        }
    });
}
