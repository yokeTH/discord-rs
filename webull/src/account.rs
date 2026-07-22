//! Account queries: list accounts, balances, and positions.

use serde::Deserialize;

#[cfg(feature = "http")]
use crate::client::WebullClient;
#[cfg(feature = "http")]
use anyhow::Result;
#[cfg(feature = "http")]
use tracing::instrument;

/// A brokerage account belonging to the authenticated user.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Account {
    pub user_id: String,
    pub account_id: String,
    pub account_number: String,
    /// e.g. `CASH`, `MARGIN`.
    pub account_type: String,
    /// e.g. `INDIVIDUAL_CASH`.
    pub account_class: String,
    pub account_label: String,
}

/// Per-currency balance detail within a [`Balance`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CurrencyAsset {
    pub currency: String,
    pub cash_balance: String,
    pub market_value: String,
    pub buying_power: String,
    pub unrealized_profit_loss: String,
}

/// Account balance summary.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Balance {
    pub total_asset_currency: String,
    pub total_cash_balance: String,
    pub total_market_value: String,
    pub total_unrealized_profit_loss: String,
    pub account_currency_assets: Vec<CurrencyAsset>,
}

/// An open position.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Position {
    pub position_id: String,
    pub currency: String,
    pub quantity: String,
    pub symbol: String,
    pub instrument_type: String,
    pub last_price: String,
    pub cost_price: String,
    pub unrealized_profit_loss: String,
}

#[cfg(feature = "http")]
impl WebullClient {
    /// List the authenticated user's accounts.
    #[instrument(name = "webull_account_list", skip_all)]
    pub async fn account_list(&self) -> Result<Vec<Account>> {
        self.get("/openapi/account/list", &[], true).await
    }

    /// Balance summary for one account.
    #[instrument(name = "webull_account_balance", skip(self))]
    pub async fn account_balance(&self, account_id: &str) -> Result<Balance> {
        let query = vec![("account_id".to_string(), account_id.to_string())];
        self.get("/openapi/assets/balance", &query, true).await
    }

    /// Open positions for one account.
    #[instrument(name = "webull_account_positions", skip(self))]
    pub async fn account_positions(&self, account_id: &str) -> Result<Vec<Position>> {
        let query = vec![("account_id".to_string(), account_id.to_string())];
        self.get("/openapi/assets/positions", &query, true).await
    }

    /// Detailed lots for a single position.
    #[instrument(name = "webull_position_details", skip(self))]
    pub async fn account_position_details(
        &self,
        account_id: &str,
        instrument_id: &str,
        page_size: Option<u32>,
        last_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut query = vec![
            ("account_id".to_string(), account_id.to_string()),
            ("instrument_id".to_string(), instrument_id.to_string()),
        ];
        if let Some(n) = page_size {
            query.push(("page_size".to_string(), n.to_string()));
        }
        if let Some(id) = last_id {
            query.push(("last_id".to_string(), id.to_string()));
        }
        self.get("/openapi/assets/position/details", &query, true)
            .await
    }

    /// Account profile (legacy v1 endpoint).
    #[instrument(name = "webull_account_profile", skip(self))]
    pub async fn account_profile(&self, account_id: &str) -> Result<serde_json::Value> {
        let query = vec![("account_id".to_string(), account_id.to_string())];
        self.get("/account/profile", &query, true).await
    }

    /// List the app's market-data / trading subscriptions.
    #[instrument(name = "webull_app_subscriptions", skip(self))]
    pub async fn app_subscriptions(
        &self,
        subscription_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut query = Vec::new();
        if let Some(id) = subscription_id {
            query.push(("subscription_id".to_string(), id.to_string()));
        }
        self.get("/app/subscriptions/list", &query, true).await
    }
}
