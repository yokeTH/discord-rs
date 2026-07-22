//! End-to-end UAT smoke test against Webull's public shared test account #1
//! (credentials published at developer.webull.co.th/apis/docs/sdk).
//!
//! UAT serves market data without a subscription, so unlike production it can
//! exercise the snapshot/bars/quotes endpoints. Run:
//!
//! ```sh
//! nix develop -c cargo run -p webull --example uat_smoke
//! ```

use webull::{Category, Timespan, TokenStatus, UAT_BASE_URL, WebullClient};

// Public shared UAT test account #1 (not secrets — documented for everyone).
const APP_KEY: &str = "86d0dac12b1b28de7539f087b2c1dca7";
const APP_SECRET: &str = "28dfb45a1be192a04242efd1aeba20f6";
const ACCOUNT_ID: &str = "1249711087713001472";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = WebullClient::new(
        UAT_BASE_URL.to_string(),
        APP_KEY.to_string(),
        APP_SECRET.to_string(),
        None,
    )?;

    let token = client.create_token().await?;
    println!("create_token: status={:?}", token.status);
    let token = if token.status == TokenStatus::Normal {
        token
    } else {
        client.check_token(&token.token).await?
    };
    client.set_access_token(token.token);
    println!("--- authed calls ---");

    report(
        "account_list",
        client
            .account_list()
            .await
            .map(|a| format!("{} accounts", a.len())),
    );
    report(
        "snapshot(AAPL)",
        client
            .snapshot(&["AAPL"], Category::UsStock, false, false)
            .await
            .map(|s| format!("price={:?}", s.first().map(|x| x.price.clone()))),
    );
    report(
        "bars(AAPL)",
        client
            .bars("AAPL", Category::UsStock, Timespan::Day, 3, true)
            .await
            .map(|b| format!("{} bars", b.len())),
    );
    report(
        "quotes(AAPL)",
        client
            .quotes("AAPL", Category::UsStock, 1, false)
            .await
            .map(|q| format!("bids={} asks={}", q.bids.len(), q.asks.len())),
    );
    report(
        "account_balance",
        client
            .account_balance(ACCOUNT_ID)
            .await
            .map(|b| format!("total_cash={}", b.total_cash_balance)),
    );
    report(
        "account_positions",
        client
            .account_positions(ACCOUNT_ID)
            .await
            .map(|p| format!("{} positions", p.len())),
    );
    report(
        "order_history",
        client
            .order_history(ACCOUNT_ID, Some("2026-01-01"), Some(10), None)
            .await
            .map(|o| format!("{} records", o.len())),
    );
    Ok(())
}

fn report(name: &str, result: anyhow::Result<String>) {
    match result {
        Ok(v) => println!("  OK   {name}: {v}"),
        Err(e) => println!("  FAIL {name}: {e:#}"),
    }
}
