use std::time::Duration;

use anyhow::Error;
use fred::prelude::*;
use log::error;

#[derive(Clone)]
pub struct SymbolStore {
    client: Client,
    key_prefix: String,
}

impl SymbolStore {
    pub async fn new(redis_url: &str, key: impl Into<String>) -> Result<Self, Error> {
        let config = Config::from_url(redis_url)?;

        let client = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(5);
                config.tcp = TcpConfig {
                    nodelay: Some(true),
                    ..Default::default()
                };
            })
            .build()?;

        client.on_error(|(error, server)| async move {
            error!("{:?}: Redis connection error: {:?}", server, error);
            Ok(())
        });

        client.connect();
        client.wait_for_connect().await?;

        Ok(Self {
            client,
            key_prefix: key.into(),
        })
    }

    /// Create a new SymbolStore from environment variables.
    /// Expects REDIS_URL and REDIS_KEY_PREFIX to be set.
    pub async fn from_env() -> Result<Self, Error> {
        use std::env;

        let redis_url = env::var("REDIS_URL")
            .map_err(|_| Error::msg("REDIS_URL environment variable not set"))?;
        let key_prefix = env::var("REDIS_KEY_PREFIX")
            .map_err(|_| Error::msg("REDIS_KEY_PREFIX environment variable not set"))?;

        Self::new(&redis_url, key_prefix).await
    }

    fn normalize(symbol: &str) -> String {
        symbol.trim().to_uppercase()
    }

    fn watchlist_key(&self) -> String {
        format!("{}:watchlist", self.key_prefix)
    }

    fn pending_del_key(&self, request_id: String) -> String {
        format!("{}:pending_del:{}", self.key_prefix, request_id)
    }

    /// Add a stock symbol
    /// Returns true if it was newly added
    pub async fn add(&self, symbol: &str) -> Result<bool, Error> {
        let added: i64 = self
            .client
            .sadd(self.watchlist_key(), Self::normalize(symbol))
            .await?;

        Ok(added == 1)
    }

    /// Remove a stock symbol
    /// Returns true if it existed
    pub async fn remove(&self, symbol: &str) -> Result<bool, Error> {
        let removed: i64 = self
            .client
            .srem(self.watchlist_key(), Self::normalize(symbol))
            .await?;

        Ok(removed == 1)
    }

    /// Get all symbols
    pub async fn list(&self) -> Result<Vec<String>, Error> {
        self.client
            .smembers(self.watchlist_key())
            .await
            .map_err(Error::from)
    }

    /// Total number of tracked symbols
    pub async fn len(&self) -> Result<usize, Error> {
        let count: i64 = self.client.scard(self.watchlist_key()).await?;
        Ok(count as usize)
    }

    /// Returns true if there are no tracked symbols
    pub async fn is_empty(&self) -> Result<bool, Error> {
        Ok(self.len().await? == 0)
    }

    /// Set Pending Delete
    pub async fn set_pending_delete(&self, id: String, symbols: Vec<String>) -> Result<i64, Error> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| Self::normalize(&s)).collect();

        let _: i64 = self.client.del(self.pending_del_key(id.clone())).await?;

        let added = if symbols.is_empty() {
            0
        } else {
            self.client
                .sadd(self.pending_del_key(id.clone()), symbols)
                .await?
        };

        let _: i64 = self
            .client
            .expire(self.pending_del_key(id), 300, None)
            .await?;

        Ok(added)
    }

    /// Get Pending Delete
    pub async fn get_pending_delete(&self, id: String) -> Result<Option<Vec<String>>, Error> {
        let members: Vec<String> = self.client.smembers(self.pending_del_key(id)).await?;

        if members.is_empty() {
            Ok(None)
        } else {
            Ok(Some(members))
        }
    }
}
