//! Token lifecycle: create and check access tokens.
//!
//! A freshly created token is `PENDING` and must be verified via SMS 2FA in the
//! Webull app within 5 minutes before it becomes `NORMAL` (the UAT environment
//! auto-activates it). A `NORMAL` token is reusable for ~15 days. Neither of
//! these calls requires an existing access token.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::client::WebullClient;
use crate::types::TokenStatus;

/// An access token and its lifecycle metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    /// 32-char hex token used in the `x-access-token` header.
    pub token: String,
    /// Unix timestamp in milliseconds at which the token becomes invalid.
    pub expires: i64,
    pub status: TokenStatus,
}

#[derive(Serialize)]
struct CheckTokenBody<'a> {
    token: &'a str,
}

impl WebullClient {
    /// Create a new access token. The returned token starts `PENDING` and needs
    /// 2FA verification (see the module docs) before it can authenticate calls.
    #[instrument(name = "webull_create_token", skip_all)]
    pub async fn create_token(&self) -> Result<Token> {
        self.post_empty("/openapi/auth/token/create", false).await
    }

    /// Check the status of an existing token.
    #[instrument(name = "webull_check_token", skip_all)]
    pub async fn check_token(&self, token: &str) -> Result<Token> {
        self.post(
            "/openapi/auth/token/check",
            &CheckTokenBody { token },
            false,
        )
        .await
    }

    /// Refresh (extend) an existing token without re-doing 2FA, returning the
    /// refreshed token and its new expiry. This works while the current token
    /// is still valid; a fully `EXPIRED`/`INVALID` token must be re-created via
    /// [`WebullClient::create_token`] (which needs 2FA again).
    #[instrument(name = "webull_refresh_token", skip_all)]
    pub async fn refresh_token(&self, token: &str) -> Result<Token> {
        self.post(
            "/openapi/auth/token/refresh",
            &CheckTokenBody { token },
            true,
        )
        .await
    }

    /// Refresh the client's current in-memory token and store the result back
    /// on the client, keeping it alive without repeating 2FA. Errors if no
    /// token is currently set.
    #[instrument(name = "webull_refresh_and_store", skip_all)]
    pub async fn refresh_and_store(&self) -> Result<Token> {
        let current = self
            .access_token()
            .ok_or_else(|| anyhow::anyhow!("webull: no access token to refresh"))?;
        let refreshed = self.refresh_token(&current).await?;
        self.set_access_token(refreshed.token.clone());
        Ok(refreshed)
    }
}
