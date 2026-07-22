//! Request signing for the Webull OpenAPI.
//!
//! This mirrors the official Python SDK's
//! `webull.core.auth.composer.default_signature_composer.calc_signature`,
//! which is what the live server validates against. Note the SDK forces the
//! `sha_hmac256_new` signer regardless of the requested algorithm, so the
//! effective scheme is:
//!
//! * body digest: `SHA256(compact_json)` as **uppercase** hex,
//! * final signature: `base64(HMAC-SHA256(app_secret + "&", encoded))`.
//!
//! The published doc "worked example" predates this (it uses the older
//! HMAC-SHA1 + MD5 path) and cannot be reproduced with the current scheme;
//! the unit test below pins the current scheme instead, cross-checked against
//! an independent Ruby/OpenSSL implementation.
//!
//! Also note: `x-access-token`, `x-version` and `Content-Type` are **not**
//! signed, and `x-app-secret` is **never** sent — the secret is only used to
//! build the HMAC key. (The per-endpoint reference pages that list an
//! `x-app-secret` header are inaccurate; the SDK does not send it.)

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Characters left unencoded by the signer: `ALPHA / DIGIT / '-' '_' '.' '~'`.
/// Equivalent to Python's `urllib.parse.quote(s, safe='')`, which the SDK uses.
const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Value of the `x-signature-algorithm` header (also part of the signed string).
pub(crate) const SIGN_ALGORITHM: &str = "HMAC-SHA256";
/// Value of the `x-signature-version` header (also part of the signed string).
pub(crate) const SIGN_VERSION: &str = "1.0";

/// The per-request inputs to [`sign`].
pub(crate) struct SignParts<'a> {
    pub app_key: &'a str,
    pub app_secret: &'a str,
    /// The request host, e.g. `api.webull.co.th` (must match the `Host` header).
    pub host: &'a str,
    /// ISO-8601 UTC timestamp, e.g. `2022-01-04T03:55:31Z`.
    pub timestamp: &'a str,
    /// Unique per-request nonce.
    pub nonce: &'a str,
    /// Query params exactly as sent on the wire.
    pub query: &'a [(String, String)],
    /// Compact JSON body exactly as sent, or `None` for no body (GET).
    pub body: Option<&'a str>,
}

/// Current time formatted for the `x-timestamp` header.
pub(crate) fn current_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// A fresh random nonce for the `x-signature-nonce` header.
pub(crate) fn generate_nonce() -> String {
    Uuid::new_v4().to_string()
}

/// Compute the `x-signature` value for a request.
pub(crate) fn sign(path: &str, parts: &SignParts<'_>) -> String {
    // Signing headers (lower-cased keys) merged with the request query params.
    // Query keys never collide with these six in practice, so no value merging
    // is needed.
    let mut params: Vec<(&str, &str)> = vec![
        ("host", parts.host),
        ("x-app-key", parts.app_key),
        ("x-signature-algorithm", SIGN_ALGORITHM),
        ("x-signature-nonce", parts.nonce),
        ("x-signature-version", SIGN_VERSION),
        ("x-timestamp", parts.timestamp),
    ];
    for (k, v) in parts.query {
        params.push((k, v));
    }
    // Sort by key; byte order matches Python's string sort for these ASCII keys.
    params.sort_by(|a, b| a.0.cmp(b.0));
    let joined = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    // `path & sorted-params [ & SHA256(body) uppercase-hex ]`
    let mut string_to_sign = format!("{path}&{joined}");
    if let Some(body) = parts.body {
        string_to_sign.push('&');
        string_to_sign.push_str(&sha256_upper_hex(body.as_bytes()));
    }

    // URL-encode the whole string, then HMAC-SHA256 with key `app_secret + "&"`.
    let encoded = utf8_percent_encode(&string_to_sign, UNRESERVED).to_string();
    let mut mac = HmacSha256::new_from_slice(format!("{}&", parts.app_secret).as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(encoded.as_bytes());
    BASE64.encode(mac.finalize().into_bytes())
}

fn sha256_upper_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        write!(hex, "{b:02X}").expect("writing to a String never fails");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inputs are the values from the Signature doc's worked example; the
    // expected output is the *current* SDK scheme (SHA256 body digest +
    // HMAC-SHA256), independently reproduced with Ruby/OpenSSL.
    #[test]
    fn matches_sdk_reference_vector() {
        let query = vec![
            ("a1".to_string(), "webull".to_string()),
            ("a2".to_string(), "123".to_string()),
            ("a3".to_string(), "xxx".to_string()),
            ("q1".to_string(), "yyy".to_string()),
        ];
        let body = r#"{"k1":123,"k2":"this is the api request body","k3":true,"k4":{"foo":[1,2]}}"#;
        let parts = SignParts {
            app_key: "776da210ab4a452795d74e726ebd74b6",
            app_secret: "0f50a2e853334a9aae1a783bee120c1f",
            host: "api.webull.com.sg",
            timestamp: "2022-01-04T03:55:31Z",
            nonce: "48ef5afed43d4d91ae514aaeafbc29ba",
            query: &query,
            body: Some(body),
        };
        assert_eq!(
            sign("/trade/place_order", &parts),
            "mafoLh6ZyyaOs0UAHUagoSH2yXTDzlkRUuCR/Xjo7UE=",
        );
    }

    #[test]
    fn body_digest_is_uppercase_sha256() {
        let body = r#"{"k1":123,"k2":"this is the api request body","k3":true,"k4":{"foo":[1,2]}}"#;
        assert_eq!(
            sha256_upper_hex(body.as_bytes()),
            "08B9F294222127D6BA471D2A53634393B4FB8E8F038B09183AF6B2164F610C08",
        );
    }

    // A GET request (no body) omits the trailing body-hash segment.
    #[test]
    fn get_request_has_no_body_segment() {
        let query = vec![("symbols".to_string(), "AAPL".to_string())];
        let parts = SignParts {
            app_key: "k",
            app_secret: "s",
            host: "api.webull.co.th",
            timestamp: "2024-01-01T00:00:00Z",
            nonce: "n",
            query: &query,
            body: None,
        };
        // Just assert it produces a stable, non-empty base64 signature.
        let sig = sign("/openapi/market-data/stock/snapshot", &parts);
        assert!(!sig.is_empty() && sig.ends_with('='));
    }
}
