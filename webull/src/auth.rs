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
}
