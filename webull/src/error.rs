//! Maps non-2xx Webull responses into descriptive errors.
//!
//! Webull returns a `{ "error_code": ..., "message": ... }` body on failure,
//! and the `error_code` is actionable (e.g. `OPENAPI_NO_NIGHT_TRADING_TIME`
//! when an order is rejected), so it's surfaced rather than discarded by a
//! bare `error_for_status()`.

use anyhow::{Result, anyhow};
use reqwest::Response;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ApiError {
    error_code: Option<String>,
    message: Option<String>,
}

/// Return the response unchanged on 2xx; otherwise read the body and build an
/// error carrying the HTTP status and any Webull `error_code` / `message`.
pub(crate) async fn check(resp: Response) -> Result<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }

    let body = resp.text().await.unwrap_or_default();
    match serde_json::from_str::<ApiError>(&body) {
        Ok(ApiError {
            error_code: Some(code),
            message,
        }) => Err(anyhow!(
            "webull API error {status}: {code}{}",
            message.map(|m| format!(" — {m}")).unwrap_or_default()
        )),
        _ => Err(anyhow!("webull API error {status}: {body}")),
    }
}
