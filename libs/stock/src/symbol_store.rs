use std::time::Duration;

use anyhow::Error;
use fred::{prelude::*, socket2::TcpKeepalive};

use tracing::{debug, error, info, instrument, warn};

#[derive(Clone)]
pub struct SymbolStore {
    client: Client,
    key_prefix: String,
}

impl SymbolStore {
    #[instrument(name = "symbol_store_new", skip(redis_url), fields(key_prefix = %key_prefix))]
    pub async fn new(redis_url: &str, key_prefix: String) -> Result<Self, Error> {
        debug!("building redis config");
        let config = Config::from_url(redis_url)?;

        let client = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(5);
                config.unresponsive.max_timeout = Some(Duration::from_secs(30));
                config.unresponsive.interval = Duration::from_secs(2);

                config.tcp = TcpConfig {
                    nodelay: Some(true),
                    keepalive: Some(
                        TcpKeepalive::new()
                            .with_time(Duration::from_secs(30))
                            .with_interval(Duration::from_secs(10)),
                    ),
                    ..Default::default()
                };
            })
            .with_performance_config(|p| {
                p.default_command_timeout = Duration::from_secs(10);
            })
            .set_policy(ReconnectPolicy::Exponential {
                attempts: 3,
                max_attempts: 10,
                min_delay: 1,
                max_delay: 30,
                base: 2,
                jitter: 500,
            })
            .build()?;

        client.on_error(|(err, server)| async move {
            error!(server = ?server, error = ?err, "redis client error");
            Ok(())
        });

        client.on_reconnect(|server| async move {
            info!("Reconnected to {}", server);
            Ok(())
        });

        info!("connecting to redis");
        client.init().await?;
        info!("redis connected");

        Ok(Self { client, key_prefix })
    }

    /// Create a new SymbolStore from environment variables.
    /// Expects REDIS_URL and REDIS_KEY_PREFIX to be set.
    #[instrument(name = "symbol_store_from_env", skip_all)]
    pub async fn from_env() -> Result<Self, Error> {
        use std::env;

        let redis_url = env::var("REDIS_URL")
            .map_err(|_| Error::msg("REDIS_URL environment variable not set"))?;
        let key_prefix = env::var("REDIS_KEY_PREFIX")
            .map_err(|_| Error::msg("REDIS_KEY_PREFIX environment variable not set"))?;

        info!(key_prefix = %key_prefix, "creating SymbolStore from env");
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
    #[instrument(name = "symbol_store_add", skip(self), fields(symbol = %symbol))]
    pub async fn add(&self, symbol: &str) -> Result<bool, Error> {
        let normalized = Self::normalize(symbol);
        let added: i64 = self.client.sadd(self.watchlist_key(), normalized).await?;
        debug!(added, "sadd done");
        Ok(added == 1)
    }

    /// Remove a stock symbol
    /// Returns true if it existed
    #[instrument(name = "symbol_store_remove", skip(self), fields(symbol = %symbol))]
    pub async fn remove(&self, symbol: &str) -> Result<bool, Error> {
        let normalized = Self::normalize(symbol);
        let removed: i64 = self.client.srem(self.watchlist_key(), normalized).await?;
        debug!(removed, "srem done");
        Ok(removed == 1)
    }

    /// Get all symbols
    #[instrument(name = "symbol_store_list", skip(self))]
    pub async fn list(&self) -> Result<Vec<String>, Error> {
        let members: Vec<String> = self.client.smembers(self.watchlist_key()).await?;
        debug!(count = members.len(), "smembers done");
        Ok(members)
    }

    /// Total number of tracked symbols
    #[instrument(name = "symbol_store_len", skip(self))]
    pub async fn len(&self) -> Result<usize, Error> {
        let count: i64 = self.client.scard(self.watchlist_key()).await?;
        Ok(count as usize)
    }

    /// Returns true if there are no tracked symbols
    #[instrument(name = "symbol_store_is_empty", skip(self))]
    pub async fn is_empty(&self) -> Result<bool, Error> {
        Ok(self.len().await? == 0)
    }

    /// Set Pending Delete
    #[instrument(
        name = "symbol_store_set_pending_delete",
        skip(self, symbols),
        fields(req_id = %id, symbol_count = symbols.len())
    )]
    pub async fn set_pending_delete(&self, id: String, symbols: Vec<String>) -> Result<i64, Error> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| Self::normalize(&s)).collect();

        let del_key = self.pending_del_key(id.clone());
        let _: i64 = self.client.del(del_key.clone()).await?;

        let added = if symbols.is_empty() {
            warn!("no symbols provided for pending delete");
            0
        } else {
            let added: i64 = self.client.sadd(del_key.clone(), symbols).await?;
            added
        };

        let _: i64 = self.client.expire(del_key, 300, None).await?;
        debug!(added, "pending delete set");

        Ok(added)
    }

    /// Get Pending Delete
    #[instrument(name = "symbol_store_get_pending_delete", skip(self), fields(req_id = %id))]
    pub async fn get_pending_delete(&self, id: String) -> Result<Option<Vec<String>>, Error> {
        let members: Vec<String> = self.client.smembers(self.pending_del_key(id)).await?;
        if members.is_empty() {
            Ok(None)
        } else {
            debug!(count = members.len(), "pending delete loaded");
            Ok(Some(members))
        }
    }
}
