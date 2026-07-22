//! Webull API regions and their REST host domains.
//!
//! Ported from the SDK's `endpoints.json` region map (the `api` key, used for
//! every REST call). Always compiled — no dependency on the HTTP client.

/// A Webull region, selecting the REST API host domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Region {
    Us,
    Hk,
    Jp,
    Sg,
    /// Thailand (this crate's default).
    #[default]
    Th,
    Au,
    My,
    Uk,
    Br,
    Mx,
    Za,
    Eu,
}

impl Region {
    /// The REST API host for this region (the `api` endpoint key).
    #[must_use]
    pub fn api_host(self) -> &'static str {
        match self {
            Region::Us | Region::Br | Region::Mx => "api.webull.com",
            Region::Hk => "api.webull.hk",
            Region::Jp => "api.webull.co.jp",
            Region::Sg => "api.webull.com.sg",
            Region::Th => "api.webull.co.th",
            Region::Au | Region::Za => "api.webull.com.au",
            Region::My => "api.webull.com.my",
            Region::Uk => "api.webull-uk.com",
            Region::Eu => "api.webull.eu",
        }
    }

    /// The full `https://` base URL for this region.
    #[must_use]
    pub fn base_url(self) -> String {
        format!("https://{}", self.api_host())
    }
}
