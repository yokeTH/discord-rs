//! Background task that keeps the Webull access token alive.
//!
//! Webull tokens expire ~15 days after their last use, but can be extended
//! without repeating 2FA via the refresh endpoint. This task refreshes the
//! in-memory token on an interval so it never lapses while the bot is running.
//! (If the bot is down longer than the expiry window, the token goes invalid
//! and must be re-issued with the `create_token` example.)

use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Instant, MissedTickBehavior, interval_at};
use tracing::{debug, info, warn};
use webull::WebullClient;

/// How often to refresh — comfortably within Webull's ~15-day expiry.
const REFRESH_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

/// Spawn the refresh loop. It ticks every [`REFRESH_INTERVAL`] and refreshes the
/// in-memory token in place; ticks are skipped while no token is set yet (the
/// bootstrap may still be verifying one). The first tick is delayed a full
/// interval so it never races a token the bootstrap just installed.
pub fn spawn(client: Arc<WebullClient>) {
    tokio::spawn(async move {
        let mut ticker = interval_at(Instant::now() + REFRESH_INTERVAL, REFRESH_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if client.access_token().is_none() {
                debug!("no webull token set; skipping refresh");
                continue;
            }
            match client.refresh_and_store().await {
                Ok(t) => info!(status = ?t.status, expires = t.expires, "webull token refreshed"),
                Err(e) => warn!(
                    error = ?e,
                    "webull token refresh failed; will retry (run /webull login if it expired)",
                ),
            }
        }
    });
}
