use std::env::var;

#[derive(Clone)]
pub struct Config {
    pub discord_token: String,
    pub version: String,
    /// Discord user id allowed to run trading commands. When unset, trading is
    /// disabled for everyone.
    pub owner_id: Option<u64>,
    /// Webull account to trade in. When unset, the first account is used.
    pub webull_account_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set"),
            version: var("APP_VERSION").unwrap_or_else(|_| "Unknown".to_string()),
            owner_id: var("OWNER_ID").ok().and_then(|s| s.trim().parse().ok()),
            webull_account_id: var("WEBULL_ACCOUNT_ID")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        }
    }
}
