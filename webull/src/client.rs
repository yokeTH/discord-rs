//! The [`WebullClient`] and its signed-request plumbing.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, Method};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::{debug, info, instrument};

use crate::error;
use crate::signature::{self, SignParts};

/// Production base URL for the Thailand region (trading + market data).
pub const PROD_BASE_URL: &str = "https://api.webull.co.th";
/// UAT / test base URL. The UAT environment bypasses SMS 2FA on tokens.
pub const UAT_BASE_URL: &str = "https://th-api.uat.webullbroker.com";

/// A client for the Webull OpenAPI.
///
/// Cheap to [`Clone`]; clones share the same access-token store, so a token
/// updated via [`WebullClient::set_access_token`] (e.g. by a background refresh)
/// is seen by every clone. Construct with [`WebullClient::new`] or
/// [`WebullClient::from_env`].
#[derive(Clone)]
pub struct WebullClient {
    client: Client,
    /// Base URL with no trailing slash, e.g. `https://api.webull.co.th`.
    base_url: String,
    /// Host portion of `base_url`, signed and sent as the `Host` header.
    host: String,
    app_key: String,
    app_secret: String,
    /// Shared, mutable access token, updated in place by token refresh.
    access_token: Arc<RwLock<Option<String>>>,
}

impl WebullClient {
    /// Build a client against `base_url` (e.g. [`PROD_BASE_URL`]).
    ///
    /// `access_token` may be `None` — token creation ([`WebullClient::create_token`])
    /// works without one, but every other endpoint requires it.
    #[instrument(name = "webull_new", skip(app_key, app_secret, access_token), fields(base_url = %base_url))]
    pub fn new(
        base_url: String,
        app_key: String,
        app_secret: String,
        access_token: Option<String>,
    ) -> Result<Self> {
        let host = reqwest::Url::parse(&base_url)
            .with_context(|| format!("invalid WEBULL base_url: {base_url}"))?
            .host_str()
            .with_context(|| format!("WEBULL base_url has no host: {base_url}"))?
            .to_string();

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()?;

        info!("webull client initialized");
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            host,
            app_key,
            app_secret,
            access_token: Arc::new(RwLock::new(access_token)),
        })
    }

    /// Build a client from environment variables:
    /// `WEBULL_APP_KEY`, `WEBULL_APP_SECRET` (required), `WEBULL_ACCESS_TOKEN`
    /// and `WEBULL_BASE_URL` (optional; base URL defaults to [`PROD_BASE_URL`]).
    #[instrument(name = "webull_from_env", skip_all)]
    pub fn from_env() -> Result<Self> {
        let base_url =
            std::env::var("WEBULL_BASE_URL").unwrap_or_else(|_| PROD_BASE_URL.to_string());
        let app_key = std::env::var("WEBULL_APP_KEY")?;
        let app_secret = std::env::var("WEBULL_APP_SECRET")?;
        let access_token = std::env::var("WEBULL_ACCESS_TOKEN")
            .ok()
            .filter(|t| !t.is_empty());

        debug!(base_url = %base_url, has_token = access_token.is_some(), "loaded webull env vars");
        Self::new(base_url, app_key, app_secret, access_token)
    }

    /// Return this client configured to use `token` for authenticated calls.
    pub fn with_access_token(self, token: String) -> Self {
        self.set_access_token(token);
        self
    }

    /// Replace the access token used for authenticated calls. The change is
    /// shared with every clone of this client.
    pub fn set_access_token(&self, token: String) {
        *self
            .access_token
            .write()
            .expect("access-token lock poisoned") = Some(token);
    }

    /// The currently configured access token, if any.
    pub fn access_token(&self) -> Option<String> {
        self.access_token
            .read()
            .expect("access-token lock poisoned")
            .clone()
    }

    /// Signed `GET` returning a decoded JSON body.
    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        authed: bool,
    ) -> Result<T> {
        self.request(Method::GET, path, query, None, authed).await
    }

    /// Signed `POST` with a JSON body, returning a decoded JSON body.
    pub(crate) async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        authed: bool,
    ) -> Result<T> {
        let json = serde_json::to_string(body).context("serializing webull request body")?;
        self.request(Method::POST, path, &[], Some(json), authed)
            .await
    }

    /// Signed `POST` with no body (used by token creation).
    pub(crate) async fn post_empty<T: DeserializeOwned>(
        &self,
        path: &str,
        authed: bool,
    ) -> Result<T> {
        self.request(Method::POST, path, &[], None, authed).await
    }

    /// Core request pipeline: sign, attach headers, send, map errors, decode.
    ///
    /// `body` is the exact compact-JSON string to send (and hash) — signing and
    /// the wire body must use identical bytes.
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        body: Option<String>,
        authed: bool,
    ) -> Result<T> {
        let access_token = self.access_token();
        if authed && access_token.is_none() {
            anyhow::bail!(
                "webull: {path} requires an access token, but none is configured \
                 (set WEBULL_ACCESS_TOKEN or call with_access_token)"
            );
        }

        let timestamp = signature::current_timestamp();
        let nonce = signature::generate_nonce();
        let sig = signature::sign(
            path,
            &SignParts {
                app_key: &self.app_key,
                app_secret: &self.app_secret,
                host: &self.host,
                timestamp: &timestamp,
                nonce: &nonce,
                query,
                body: body.as_deref(),
            },
        );

        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .client
            .request(method, &url)
            .header("x-app-key", &self.app_key)
            .header("x-timestamp", &timestamp)
            .header("x-signature-version", signature::SIGN_VERSION)
            .header("x-signature-algorithm", signature::SIGN_ALGORITHM)
            .header("x-signature-nonce", &nonce)
            .header("x-signature", &sig)
            .header("x-version", "v2");

        if !query.is_empty() {
            req = req.query(query);
        }
        if authed && let Some(token) = &access_token {
            req = req.header("x-access-token", token);
        }
        if let Some(body) = body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }

        debug!(%url, "sending webull request");
        let resp = req.send().await.context("webull request failed")?;
        let resp = error::check(resp).await?;
        resp.json::<T>().await.context("decoding webull response")
    }
}
