//! List your Webull accounts, open orders, and recent order history.
//!
//! ```sh
//! nix develop -c cargo run -p webull --example list_orders
//! ```

use webull::WebullClient;

fn fmt_leg(o: &webull::OrderLeg) -> String {
    format!(
        "{sym:<6} {side:<4} {otype:<14} qty {filled}/{total} @ {price:<8} {status} {when}",
        sym = o.symbol,
        side = o.side.map(|s| format!("{s:?}")).unwrap_or_default(),
        otype = o.order_type.map(|t| format!("{t:?}")).unwrap_or_default(),
        filled = o.filled_quantity.as_deref().unwrap_or("0"),
        total = o.total_quantity.as_deref().unwrap_or("-"),
        price = o
            .filled_price
            .as_deref()
            .or(o.limit_price.as_deref())
            .unwrap_or("mkt"),
        status = o.status.map(|s| format!("{s:?}")).unwrap_or_default(),
        when = o.place_time_at.as_deref().unwrap_or(""),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let client = WebullClient::from_env()?;

    let accounts = client.account_list().await?;
    if accounts.is_empty() {
        println!("No accounts found.");
        return Ok(());
    }

    for acct in &accounts {
        println!(
            "\n=== Account {} ({}, {}) ===",
            acct.account_id, acct.account_number, acct.account_type
        );

        let open = client
            .open_orders(&acct.account_id, Some(100), None)
            .await?;
        let open_count: usize = open.iter().map(|r| r.orders.len()).sum();
        println!("-- Open orders ({open_count}) --");
        for rec in &open {
            for o in &rec.orders {
                println!("  {}", fmt_leg(o));
            }
        }

        // History defaults to the last 7 days; widen to ~6 months (the max).
        let history = client
            .order_history(&acct.account_id, Some("2026-01-22"), Some(100), None)
            .await?;
        let hist_count: usize = history.iter().map(|r| r.orders.len()).sum();
        println!("-- Order history, since 2026-01-22 ({hist_count}) --");
        for rec in &history {
            for o in &rec.orders {
                println!("  {}", fmt_leg(o));
            }
        }
    }

    Ok(())
}
