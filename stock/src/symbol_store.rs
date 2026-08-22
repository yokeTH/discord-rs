use anyhow::Error;
use sqlx::postgres::{PgPool, PgPoolOptions};

use tracing::{debug, info, instrument, warn};

#[derive(Clone)]
pub struct SymbolStore {
    pool: PgPool,
}

impl SymbolStore {
    #[instrument(name = "symbol_store_new", skip_all)]
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        info!("connecting to database");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(database_url)
            .await?;
        info!("database connected");

        Ok(Self { pool })
    }

    /// Create a new SymbolStore from environment variables.
    /// Expects DATABASE_URL to be set.
    #[instrument(name = "symbol_store_from_env", skip_all)]
    pub async fn from_env() -> Result<Self, Error> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| Error::msg("DATABASE_URL environment variable not set"))?;

        info!("creating SymbolStore from env");
        Self::new(&database_url).await
    }

    fn normalize(symbol: &str) -> String {
        symbol.trim().to_uppercase()
    }

    /// Add a stock symbol to a channel's watchlist.
    /// Returns true if it was newly added.
    #[instrument(name = "symbol_store_add", skip(self), fields(channel_id, created_by, symbol = %symbol))]
    pub async fn add(&self, channel_id: i64, created_by: i64, symbol: &str) -> Result<bool, Error> {
        let normalized = Self::normalize(symbol);
        let res = sqlx::query!(
            "INSERT INTO watchlist (channel_id, symbol, created_by) VALUES ($1, $2, $3) \
             ON CONFLICT (channel_id, symbol) DO NOTHING",
            channel_id,
            normalized,
            created_by,
        )
        .execute(&self.pool)
        .await?;
        let added = res.rows_affected() == 1;
        debug!(added, "insert done");
        Ok(added)
    }

    /// Remove a stock symbol from a channel's watchlist.
    /// Returns true if it existed.
    #[instrument(name = "symbol_store_remove", skip(self), fields(channel_id, symbol = %symbol))]
    pub async fn remove(&self, channel_id: i64, symbol: &str) -> Result<bool, Error> {
        let normalized = Self::normalize(symbol);
        let res = sqlx::query!(
            "DELETE FROM watchlist WHERE channel_id = $1 AND symbol = $2",
            channel_id,
            normalized,
        )
        .execute(&self.pool)
        .await?;
        let removed = res.rows_affected() == 1;
        debug!(removed, "delete done");
        Ok(removed)
    }

    /// Get all symbols watched in a channel.
    #[instrument(name = "symbol_store_list", skip(self), fields(channel_id))]
    pub async fn list(&self, channel_id: i64) -> Result<Vec<String>, Error> {
        let symbols: Vec<String> = sqlx::query_scalar!(
            "SELECT symbol AS \"symbol!\" FROM watchlist WHERE channel_id = $1 ORDER BY symbol",
            channel_id,
        )
        .fetch_all(&self.pool)
        .await?;
        debug!(count = symbols.len(), "list done");
        Ok(symbols)
    }

    /// Distinct channels that have at least one watched symbol.
    #[instrument(name = "symbol_store_channels", skip(self))]
    pub async fn channels(&self) -> Result<Vec<i64>, Error> {
        let channels: Vec<i64> =
            sqlx::query_scalar!("SELECT DISTINCT channel_id AS \"channel_id!\" FROM watchlist")
                .fetch_all(&self.pool)
                .await?;
        debug!(count = channels.len(), "channels done");
        Ok(channels)
    }

    /// Set Pending Delete
    #[instrument(
        name = "symbol_store_set_pending_delete",
        skip(self, symbols),
        fields(req_id = %id, created_by, symbol_count = symbols.len())
    )]
    pub async fn set_pending_delete(
        &self,
        id: String,
        created_by: i64,
        symbols: Vec<String>,
    ) -> Result<i64, Error> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| Self::normalize(&s)).collect();

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "DELETE FROM pending_delete WHERE req_id = $1 OR expires_at < now()",
            id,
        )
        .execute(&mut *tx)
        .await?;

        if symbols.is_empty() {
            warn!("no symbols provided for pending delete");
            tx.commit().await?;
            return Ok(0);
        }

        let mut added = 0i64;
        for symbol in &symbols {
            let res = sqlx::query!(
                "INSERT INTO pending_delete (req_id, symbol, created_by, expires_at) \
                 VALUES ($1, $2, $3, now() + INTERVAL '5 minutes') \
                 ON CONFLICT (req_id, symbol) DO NOTHING",
                id,
                symbol,
                created_by,
            )
            .execute(&mut *tx)
            .await?;
            added += res.rows_affected() as i64;
        }

        tx.commit().await?;
        debug!(added, "pending delete set");

        Ok(added)
    }

    /// Mark a pending delete request as confirmed.
    #[instrument(name = "symbol_store_confirm_pending_delete", skip(self), fields(req_id = %id))]
    pub async fn confirm_pending_delete(&self, id: String) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE pending_delete SET status = 'confirmed' WHERE req_id = $1",
            id,
        )
        .execute(&self.pool)
        .await?;
        debug!("pending delete confirmed");
        Ok(())
    }

    /// Mark a pending delete request as cancelled.
    #[instrument(name = "symbol_store_cancel_pending_delete", skip(self), fields(req_id = %id))]
    pub async fn cancel_pending_delete(&self, id: String) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE pending_delete SET status = 'cancelled' WHERE req_id = $1",
            id,
        )
        .execute(&self.pool)
        .await?;
        debug!("pending delete cancelled");
        Ok(())
    }

    /// Get Pending Delete
    #[instrument(name = "symbol_store_get_pending_delete", skip(self), fields(req_id = %id))]
    pub async fn get_pending_delete(&self, id: String) -> Result<Option<Vec<String>>, Error> {
        let symbols: Vec<String> = sqlx::query_scalar!(
            "SELECT symbol AS \"symbol!\" FROM pending_delete WHERE req_id = $1 AND expires_at > now() ORDER BY symbol",
            id,
        )
        .fetch_all(&self.pool)
        .await?;

        if symbols.is_empty() {
            Ok(None)
        } else {
            debug!(count = symbols.len(), "pending delete loaded");
            Ok(Some(symbols))
        }
    }
}
