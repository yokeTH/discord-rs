//! Bootstrap a Webull access token.
//!
//! Webull requires an `x-access-token` on every request (including market
//! data). This tool creates one, waits for you to verify it via SMS 2FA in the
//! Webull app, then prints the `NORMAL` token to set as `WEBULL_ACCESS_TOKEN`.
//!
//! Run (reads WEBULL_APP_KEY / WEBULL_APP_SECRET / WEBULL_BASE_URL from `.env`
//! or the environment):
//!
//! ```sh
//! nix develop -c cargo run -p webull --example create_token
//! ```

use std::io::Write as _;
use std::time::{Duration, Instant};

use webull::{TokenStatus, WebullClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let client = WebullClient::from_env()?;

    let token = client.create_token().await?;
    println!(
        "Created token (status: {:?}):\n  {}\n",
        token.status, token.token
    );

    if token.status == TokenStatus::Normal {
        // UAT auto-activates tokens; nothing to verify.
        print_success(&token.token, token.expires);
        return Ok(());
    }

    println!("Verify it within 5 minutes in the Webull app:");
    println!(
        "  Menu → Messages → OpenAPI Notifications → (select) → Check Now → enter the SMS code"
    );
    print!("\nWaiting for verification");
    std::io::stdout().flush().ok();

    let deadline = Instant::now() + Duration::from_secs(5 * 60);
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match client.check_token(&token.token).await?.status {
            TokenStatus::Normal => {
                println!();
                print_success(&token.token, token.expires);
                return Ok(());
            }
            TokenStatus::Pending => {
                if Instant::now() >= deadline {
                    anyhow::bail!(
                        "\nverification window (5 min) elapsed — re-run to create a new token"
                    );
                }
                print!(".");
                std::io::stdout().flush().ok();
            }
            other => anyhow::bail!("\ntoken became {other:?} — re-run to create a new one"),
        }
    }
}

fn print_success(token: &str, expires: i64) {
    println!("\n✅ Token is NORMAL. Add this to your .env (valid ~15 days):\n");
    println!("WEBULL_ACCESS_TOKEN={token}");
    println!("\n(expires: {expires} unix-ms)");
}
