//! Live integration test against the Webull UAT environment.
//!
//! Ignored by default — it needs real credentials and network access. Run with:
//!
//! ```sh
//! WEBULL_BASE_URL=https://th-api.uat.webullbroker.com \
//! WEBULL_APP_KEY=... WEBULL_APP_SECRET=... WEBULL_ACCESS_TOKEN=... \
//! cargo test -p webull --test uat -- --ignored --nocapture
//! ```
//!
//! UAT bypasses SMS 2FA, so a created token activates automatically.

use webull::{Category, WebullClient};

#[tokio::test]
#[ignore = "requires live UAT credentials"]
async fn snapshot_round_trip() {
    let client = WebullClient::from_env().expect("WEBULL_* env vars must be set");
    let snaps = client
        .snapshot(&["AAPL"], Category::UsStock, false, false)
        .await
        .expect("snapshot call should succeed");
    assert!(!snaps.is_empty(), "expected at least one snapshot");
    println!("AAPL snapshot: {:#?}", snaps[0]);
}

#[tokio::test]
#[ignore = "requires live UAT credentials"]
async fn create_and_check_token() {
    let client = WebullClient::from_env().expect("WEBULL_* env vars must be set");
    let token = client.create_token().await.expect("create_token");
    println!("created token status: {:?}", token.status);
    let checked = client.check_token(&token.token).await.expect("check_token");
    println!("checked token status: {:?}", checked.status);
}
