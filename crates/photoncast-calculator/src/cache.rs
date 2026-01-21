//! SQLite cache for currency rates.
//!
//! Provides persistent storage for currency exchange rates with
//! timestamps for tracking freshness.

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::Connection;
use rust_decimal::Decimal;
use tokio::task;
use tracing::{debug, info};

use crate::currency::ExchangeRate;
use crate::error::{CalculatorError, Result};

/// Current schema version.
const SCHEMA_VERSION: i32 = 1;

/// SQLite cache for currency rates.
#[derive(Debug)]
pub struct RateCache {
    conn: Arc<Mutex<Connection>>,
}

impl RateCache {
    /// Opens or creates a rate cache at the specified path.
    pub async fn new(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        task::spawn_blocking(move || Self::open_sync(&path)).await?
    }

    /// Opens the cache synchronously.
    fn open_sync(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)
            .map_err(|e| CalculatorError::DatabaseError(format!("failed to open cache: {}", e)))?;

        // Enable WAL mode
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let cache = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        cache.run_migrations()?;

        info!("Rate cache initialized at {:?}", path);
        Ok(cache)
    }

    /// Creates an in-memory cache (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let cache = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        cache.run_migrations()?;
        Ok(cache)
    }

    /// Runs database migrations.
    fn run_migrations(&self) -> Result<()> {
        // Check if schema exists
        let has_schema: bool = self
            .conn
            .lock()
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_schema {
            Self::migrate_v1(&self.conn.lock())?;
        }

        Ok(())
    }

    /// Migration v1: Initial schema.
    fn migrate_v1(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r"
            -- Schema version
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );

            -- Currency rates
            CREATE TABLE IF NOT EXISTS currency_rates (
                code TEXT PRIMARY KEY,
                rate_to_usd TEXT NOT NULL,
                currency_type TEXT NOT NULL,
                source TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_rates_updated 
            ON currency_rates(updated_at DESC);

            -- Calculator history
            CREATE TABLE IF NOT EXISTS calculation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                expression TEXT NOT NULL,
                result_type TEXT NOT NULL,
                raw_value REAL NOT NULL,
                formatted_value TEXT NOT NULL,
                details TEXT,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_history_created 
            ON calculation_history(created_at DESC);

            INSERT INTO schema_version (version, applied_at) VALUES (1, strftime('%s', 'now'));
            ",
        )?;

        debug!("Applied migration v1");
        Ok(())
    }

    /// Stores a currency rate.
    pub fn store_rate(
        &self,
        code: &str,
        rate_to_usd: Decimal,
        currency_type: &str,
        source: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        self.conn.lock().execute(
            r"
            INSERT OR REPLACE INTO currency_rates 
            (code, rate_to_usd, currency_type, source, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            rusqlite::params![
                code.to_uppercase(),
                rate_to_usd.to_string(),
                currency_type,
                source,
                now,
            ],
        )?;

        debug!("Stored rate for {}: {}", code, rate_to_usd);
        Ok(())
    }

    /// Loads all stored rates.
    #[allow(clippy::significant_drop_tightening)]
    pub fn load_all_rates(&self) -> Result<Vec<ExchangeRate>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT code, rate_to_usd, source, updated_at
            FROM currency_rates
            ORDER BY code
            ",
        )?;

        let rates = stmt
            .query_map([], |row| {
                let code: String = row.get(0)?;
                let rate_str: String = row.get(1)?;
                let source: String = row.get(2)?;
                let updated_at: i64 = row.get(3)?;

                let rate_to_usd = rate_str.parse::<Decimal>().unwrap_or(Decimal::ONE);

                Ok(ExchangeRate {
                    code,
                    rate_to_usd,
                    source,
                    updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or_else(Utc::now),
                })
            })?
            .filter_map(std::result::Result::ok)
            .collect();

        Ok(rates)
    }

    /// Gets a specific rate.
    pub fn get_rate(&self, code: &str) -> Result<Option<ExchangeRate>> {
        let result = self.conn.lock().query_row(
            r"
            SELECT code, rate_to_usd, source, updated_at
            FROM currency_rates
            WHERE code = ?1
            ",
            [code.to_uppercase()],
            |row| {
                let code: String = row.get(0)?;
                let rate_str: String = row.get(1)?;
                let source: String = row.get(2)?;
                let updated_at: i64 = row.get(3)?;

                let rate_to_usd = rate_str.parse::<Decimal>().unwrap_or(Decimal::ONE);

                Ok(ExchangeRate {
                    code,
                    rate_to_usd,
                    source,
                    updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or_else(Utc::now),
                })
            },
        );

        match result {
            Ok(rate) => Ok(Some(rate)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Gets the last update time for rates.
    pub fn last_update_time(&self) -> Result<Option<DateTime<Utc>>> {
        let result: rusqlite::Result<i64> =
            self.conn
                .lock()
                .query_row("SELECT MAX(updated_at) FROM currency_rates", [], |row| {
                    row.get(0)
                });

        result.map_or(Ok(None), |timestamp| {
            Ok(DateTime::from_timestamp(timestamp, 0))
        })
    }

    /// Returns the number of cached rates.
    pub fn rate_count(&self) -> Result<usize> {
        let count: i64 =
            self.conn
                .lock()
                .query_row("SELECT COUNT(*) FROM currency_rates", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Clears all cached rates.
    pub fn clear_rates(&self) -> Result<()> {
        self.conn.lock().execute("DELETE FROM currency_rates", [])?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // History Operations
    // -------------------------------------------------------------------------

    /// Stores a calculation in history.
    pub fn store_history(
        &self,
        expression: &str,
        result_type: &str,
        raw_value: f64,
        formatted_value: &str,
        details: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        self.conn.lock().execute(
            r"
            INSERT INTO calculation_history 
            (expression, result_type, raw_value, formatted_value, details, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            rusqlite::params![
                expression,
                result_type,
                raw_value,
                formatted_value,
                details,
                now,
            ],
        )?;

        Ok(())
    }

    /// Loads recent history.
    #[allow(clippy::significant_drop_tightening)]
    pub fn load_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r"
            SELECT id, expression, result_type, raw_value, formatted_value, details, created_at
            FROM calculation_history
            ORDER BY created_at DESC
            LIMIT ?1
            ",
        )?;

        let limit = i64::try_from(limit).map_err(|_| CalculatorError::Overflow)?;
        let entries = stmt
            .query_map([limit], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    expression: row.get(1)?,
                    result_type: row.get(2)?,
                    raw_value: row.get(3)?,
                    formatted_value: row.get(4)?,
                    details: row.get(5)?,
                    created_at: DateTime::from_timestamp(row.get::<_, i64>(6)?, 0)
                        .unwrap_or_else(Utc::now),
                })
            })?
            .filter_map(std::result::Result::ok)
            .collect();

        Ok(entries)
    }

    /// Clears calculation history.
    pub fn clear_history(&self) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM calculation_history", [])?;
        Ok(())
    }
}

/// A history entry.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Entry ID.
    pub id: i64,
    /// The original expression.
    pub expression: String,
    /// Type of result (math, currency, unit, datetime).
    pub result_type: String,
    /// Raw numeric value.
    pub raw_value: f64,
    /// Formatted display value.
    pub formatted_value: String,
    /// Additional details.
    pub details: Option<String>,
    /// When the calculation was performed.
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_cache() {
        let cache = RateCache::in_memory().expect("should create in-memory cache");

        // Store some rates
        cache
            .store_rate("USD", Decimal::ONE, "fiat", "test")
            .expect("should store USD");
        cache
            .store_rate("EUR", Decimal::new(109, 2), "fiat", "test")
            .expect("should store EUR");

        // Load all rates
        let rates = cache.load_all_rates().expect("should load rates");
        assert_eq!(rates.len(), 2);

        // Get specific rate
        let eur = cache.get_rate("EUR").expect("should query EUR");
        assert!(eur.is_some());
        assert_eq!(eur.unwrap().rate_to_usd, Decimal::new(109, 2));

        // Count rates
        let count = cache.rate_count().expect("should count rates");
        assert_eq!(count, 2);

        // Clear rates
        cache.clear_rates().expect("should clear rates");
        let count = cache.rate_count().expect("should count rates");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_history() {
        let cache = RateCache::in_memory().expect("should create cache");

        // Store some history
        cache
            .store_history("2 + 3", "math", 5.0, "5", None)
            .expect("should store history");
        cache
            .store_history(
                "100 usd to eur",
                "currency",
                92.0,
                "€92.00",
                Some("Rate: 0.92"),
            )
            .expect("should store history");

        // Load history
        let history = cache.load_history(10).expect("should load history");
        assert_eq!(history.len(), 2);

        // Check that both expressions are present (order may vary for same-timestamp entries)
        let expressions: Vec<&str> = history.iter().map(|h| h.expression.as_str()).collect();
        assert!(expressions.contains(&"2 + 3"));
        assert!(expressions.contains(&"100 usd to eur"));

        // Clear history
        cache.clear_history().expect("should clear history");
        let history = cache.load_history(10).expect("should load history");
        assert!(history.is_empty());
    }

    #[test]
    fn test_rate_update() {
        let cache = RateCache::in_memory().expect("should create cache");

        // Store initial rate
        cache
            .store_rate("EUR", Decimal::new(108, 2), "fiat", "test")
            .expect("should store");

        // Update rate
        cache
            .store_rate("EUR", Decimal::new(110, 2), "fiat", "test")
            .expect("should update");

        // Should have one rate
        let count = cache.rate_count().expect("should count");
        assert_eq!(count, 1);

        // Should have updated value
        let eur = cache.get_rate("EUR").expect("should query");
        assert_eq!(eur.unwrap().rate_to_usd, Decimal::new(110, 2));
    }
}
