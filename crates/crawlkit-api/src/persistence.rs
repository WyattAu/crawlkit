//! Persistence for API-plane state (users, tenants, API keys).
//!
//! The API server keeps its authoritative working set in memory for speed,
//! but writes every mutation through to SQLite so that a restart does not
//! lose accounts, tenants, or keys. Sessions are intentionally NOT
//! persisted: JWTs are stateless with short (1h) expiry, and revocation is
//! best-effort in-memory (documented trade-off).
//!
//! The store is additive and idempotent: tables are created on open, and
//! loading replaces the in-memory state wholesale at startup.

use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;
use sqlx::Row;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::auth::User;
use crate::types::{ApiKey, Tenant};

/// Persistence errors.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("postgres error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Storage backend for API-plane state (users, tenants, API keys).
///
/// SQLite is the default ([`SqliteStateStore`]); Postgres is available for
/// self-hosters running the API against shared infrastructure
/// ([`PgStateStore`]). The API server treats backends interchangeably
/// through this trait.
#[async_trait::async_trait]
pub trait ApiStateStore: Send + Sync {
    /// Load all persisted users.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn load_users(&self) -> Result<Vec<User>, PersistenceError>;
    /// Insert or replace a user (write-through).
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn save_user(&self, user: &User) -> Result<(), PersistenceError>;
    /// Delete a user by id.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn delete_user(&self, id: &str) -> Result<(), PersistenceError>;
    /// Load all persisted tenants.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn load_tenants(&self) -> Result<Vec<Tenant>, PersistenceError>;
    /// Insert a tenant.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn save_tenant(&self, tenant: &Tenant) -> Result<(), PersistenceError>;
    /// Delete a tenant by id.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn delete_tenant(&self, id: &str) -> Result<(), PersistenceError>;
    /// Load all persisted API keys.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn load_api_keys(&self) -> Result<Vec<ApiKey>, PersistenceError>;
    /// Insert an API key.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn save_api_key(&self, api_key: &ApiKey) -> Result<(), PersistenceError>;
    /// Delete an API key by value.
    ///
    /// # Errors
    ///
    /// Returns the backend error on failure.
    async fn delete_api_key(&self, key: &str) -> Result<(), PersistenceError>;
}

/// SQLite-backed [`ApiStateStore`] (default backend).
pub struct SqliteStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStateStore {
    /// Open (or create) the persistence database at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] when the database cannot be opened or
    /// the schema created.
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS api_users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                roles TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS api_tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS api_keys (
                key TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                requests_per_minute INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait::async_trait]
impl ApiStateStore for SqliteStateStore {
    /// Load all persisted users.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database or deserialization failure.
    async fn load_users(&self) -> Result<Vec<User>, PersistenceError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, email, name, password_hash, tenant_id, roles, enabled FROM api_users",
        )?;
        let rows = stmt.query_map([], |row| {
            let roles_json: String = row.get(5)?;
            let roles: Vec<String> = serde_json::from_str(&roles_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(User {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                password_hash: row.get(3)?,
                tenant_id: row.get(4)?,
                roles,
                enabled: row.get::<_, i64>(6)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Insert or replace a user (write-through).
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn save_user(&self, user: &User) -> Result<(), PersistenceError> {
        let roles_json = serde_json::to_string(&user.roles)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO api_users (id, email, name, password_hash, tenant_id, roles, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                user.id,
                user.email,
                user.name,
                user.password_hash,
                user.tenant_id,
                roles_json,
                user.enabled as i64
            ],
        )?;
        Ok(())
    }

    /// Delete a user by id.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn delete_user(&self, id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM api_users WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    /// Load all persisted tenants.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn load_tenants(&self) -> Result<Vec<Tenant>, PersistenceError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, created_at FROM api_tenants")?;
        let rows = stmt.query_map([], |row| {
            let created_at: String = row.get(2)?;
            Ok(Tenant {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Insert a tenant.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn save_tenant(&self, tenant: &Tenant) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO api_tenants (id, name, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![tenant.id, tenant.name, tenant.created_at.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Delete a tenant by id.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn delete_tenant(&self, id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "DELETE FROM api_tenants WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    /// Load all persisted API keys.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn load_api_keys(&self) -> Result<Vec<ApiKey>, PersistenceError> {
        let conn = self.conn.lock().await;
        let mut stmt =
            conn.prepare("SELECT key, name, created_at, requests_per_minute FROM api_keys")?;
        let rows = stmt.query_map([], |row| {
            let created_at: String = row.get(2)?;
            Ok(ApiKey {
                key: row.get(0)?,
                name: row.get(1)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                requests_per_minute: row.get::<_, i64>(3)? as u32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Insert an API key.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn save_api_key(&self, api_key: &ApiKey) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO api_keys (key, name, created_at, requests_per_minute)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                api_key.key,
                api_key.name,
                api_key.created_at.to_rfc3339(),
                api_key.requests_per_minute as i64
            ],
        )?;
        Ok(())
    }

    /// Delete an API key by value.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    async fn delete_api_key(&self, key: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "DELETE FROM api_keys WHERE key = ?1",
            rusqlite::params![key],
        )?;
        Ok(())
    }
}

/// PostgreSQL-backed [`ApiStateStore`] for self-hosted deployments.
///
/// Schema is created idempotently on [`PgStateStore::open`]. Selected via
/// `API_STATE_PG_URL` (takes precedence over `API_STATE_DB_PATH`).
pub struct PgStateStore {
    pool: sqlx::PgPool,
}

impl PgStateStore {
    /// Open (or create) the store, ensuring the schema exists.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on connection or DDL failure.
    pub async fn open(database_url: &str) -> Result<Self, PersistenceError> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS api_users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                roles TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE
            );
            CREATE TABLE IF NOT EXISTS api_tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS api_keys (
                key TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                requests_per_minute INTEGER NOT NULL
            );",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl ApiStateStore for PgStateStore {
    async fn load_users(&self) -> Result<Vec<User>, PersistenceError> {
        let rows = sqlx::query(
            "SELECT id, email, name, password_hash, tenant_id, roles, enabled FROM api_users",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let roles_json: String = row.try_get("roles").map_err(PersistenceError::Sqlx)?;
                Ok(User {
                    id: row.try_get("id").map_err(PersistenceError::Sqlx)?,
                    email: row.try_get("email").map_err(PersistenceError::Sqlx)?,
                    name: row.try_get("name").map_err(PersistenceError::Sqlx)?,
                    password_hash: row
                        .try_get("password_hash")
                        .map_err(PersistenceError::Sqlx)?,
                    tenant_id: row.try_get("tenant_id").map_err(PersistenceError::Sqlx)?,
                    roles: serde_json::from_str(&roles_json)
                        .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
                    enabled: row
                        .try_get::<bool, _>("enabled")
                        .map_err(PersistenceError::Sqlx)?,
                })
            })
            .collect()
    }

    async fn save_user(&self, user: &User) -> Result<(), PersistenceError> {
        let roles_json = serde_json::to_string(&user.roles)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        sqlx::query(
            "INSERT INTO api_users (id, email, name, password_hash, tenant_id, roles, enabled)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE SET
                email = EXCLUDED.email,
                name = EXCLUDED.name,
                password_hash = EXCLUDED.password_hash,
                tenant_id = EXCLUDED.tenant_id,
                roles = EXCLUDED.roles,
                enabled = EXCLUDED.enabled",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.tenant_id)
        .bind(&roles_json)
        .bind(user.enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_user(&self, id: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM api_users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_tenants(&self) -> Result<Vec<Tenant>, PersistenceError> {
        let rows = sqlx::query("SELECT id, name, created_at FROM api_tenants")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Tenant {
                    id: row.try_get("id").map_err(PersistenceError::Sqlx)?,
                    name: row.try_get("name").map_err(PersistenceError::Sqlx)?,
                    created_at: parse_rfc3339(
                        &row.try_get::<String, _>("created_at")
                            .map_err(PersistenceError::Sqlx)?,
                    ),
                })
            })
            .collect()
    }

    async fn save_tenant(&self, tenant: &Tenant) -> Result<(), PersistenceError> {
        sqlx::query(
            "INSERT INTO api_tenants (id, name, created_at) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name",
        )
        .bind(&tenant.id)
        .bind(&tenant.name)
        .bind(tenant.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_tenant(&self, id: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM api_tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_api_keys(&self) -> Result<Vec<ApiKey>, PersistenceError> {
        let rows = sqlx::query("SELECT key, name, created_at, requests_per_minute FROM api_keys")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(ApiKey {
                    key: row.try_get("key").map_err(PersistenceError::Sqlx)?,
                    name: row.try_get("name").map_err(PersistenceError::Sqlx)?,
                    created_at: parse_rfc3339(
                        &row.try_get::<String, _>("created_at")
                            .map_err(PersistenceError::Sqlx)?,
                    ),
                    requests_per_minute: row
                        .try_get::<i64, _>("requests_per_minute")
                        .map_err(PersistenceError::Sqlx)?
                        as u32,
                })
            })
            .collect()
    }

    async fn save_api_key(&self, api_key: &ApiKey) -> Result<(), PersistenceError> {
        sqlx::query(
            "INSERT INTO api_keys (key, name, created_at, requests_per_minute)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (key) DO UPDATE SET
                name = EXCLUDED.name,
                requests_per_minute = EXCLUDED.requests_per_minute",
        )
        .bind(&api_key.key)
        .bind(&api_key.name)
        .bind(api_key.created_at.to_rfc3339())
        .bind(api_key.requests_per_minute as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_api_key(&self, key: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM api_keys WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Parse an RFC 3339 timestamp with a defensive fallback to now.
fn parse_rfc3339(value: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod pg_tests {
    use super::*;
    use crate::auth::User;

    fn test_user(id: &str) -> User {
        User {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            name: id.to_string(),
            password_hash: "hash".to_string(),
            tenant_id: "t1".to_string(),
            roles: vec!["viewer".to_string()],
            enabled: true,
        }
    }

    #[tokio::test]
    #[ignore] // Requires a running PostgreSQL instance (DATABASE_URL or default)
    async fn pg_state_store_roundtrip() {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/crawlkit_test".to_string());
        let store = PgStateStore::open(&url).await.unwrap();

        store.save_user(&test_user("pg-u1")).await.unwrap();
        let users = store.load_users().await.unwrap();
        assert!(users.iter().any(|u| u.id == "pg-u1"));

        let tenant = Tenant {
            id: "pg-acme".to_string(),
            name: "ACME".to_string(),
            created_at: chrono::Utc::now(),
        };
        store.save_tenant(&tenant).await.unwrap();
        assert!(store
            .load_tenants()
            .await
            .unwrap()
            .iter()
            .any(|t| t.id == "pg-acme"));

        let key = ApiKey {
            key: "ck_pg_test".to_string(),
            name: "ci".to_string(),
            created_at: chrono::Utc::now(),
            requests_per_minute: 60,
        };
        store.save_api_key(&key).await.unwrap();
        assert!(store
            .load_api_keys()
            .await
            .unwrap()
            .iter()
            .any(|k| k.key == "ck_pg_test"));

        store.delete_user("pg-u1").await.unwrap();
        store.delete_tenant("pg-acme").await.unwrap();
        store.delete_api_key("ck_pg_test").await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user(id: &str) -> User {
        User {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            name: id.to_string(),
            password_hash: "hash".to_string(),
            tenant_id: "t1".to_string(),
            roles: vec!["viewer".to_string()],
            enabled: true,
        }
    }

    #[tokio::test]
    async fn users_roundtrip_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteStateStore::open(&dir.path().join("state.db")).unwrap();

        store.save_user(&test_user("u1")).await.unwrap();
        store.save_user(&test_user("u2")).await.unwrap();
        let users = store.load_users().await.unwrap();
        assert_eq!(users.len(), 2);
        assert!(users
            .iter()
            .any(|u| u.id == "u1" && u.roles == vec!["viewer".to_string()]));

        // Reopen from the same file: state survives.
        drop(store);
        let reopened = SqliteStateStore::open(&dir.path().join("state.db")).unwrap();
        assert_eq!(reopened.load_users().await.unwrap().len(), 2);

        reopened.delete_user("u1").await.unwrap();
        assert_eq!(reopened.load_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tenants_and_keys_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteStateStore::open(&dir.path().join("state.db")).unwrap();

        let tenant = Tenant {
            id: "acme".to_string(),
            name: "ACME".to_string(),
            created_at: chrono::Utc::now(),
        };
        store.save_tenant(&tenant).await.unwrap();

        let key = ApiKey {
            key: "ck_test".to_string(),
            name: "ci".to_string(),
            created_at: chrono::Utc::now(),
            requests_per_minute: 120,
        };
        store.save_api_key(&key).await.unwrap();

        let tenants = store.load_tenants().await.unwrap();
        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, "acme");

        let keys = store.load_api_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].requests_per_minute, 120);

        store.delete_tenant("acme").await.unwrap();
        store.delete_api_key("ck_test").await.unwrap();
        assert!(store.load_tenants().await.unwrap().is_empty());
        assert!(store.load_api_keys().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn save_user_replaces_on_same_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteStateStore::open(&dir.path().join("state.db")).unwrap();
        store.save_user(&test_user("u1")).await.unwrap();
        let mut updated = test_user("u1");
        updated.roles = vec!["admin".to_string()];
        store.save_user(&updated).await.unwrap();
        let users = store.load_users().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].roles, vec!["admin".to_string()]);
    }
}
