use std::env::var;

#[derive(Clone)]
pub struct Config {
    pub discord_token: String,
    pub version: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            discord_token: var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set"),
            version: var("APP_VERSION").unwrap_or_else(|_| "Unknown".to_string()),
        }
    }
}
