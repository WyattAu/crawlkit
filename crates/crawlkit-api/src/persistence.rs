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
}

/// SQLite-backed write-through store for API-plane state.
pub struct ApiStatePersistence {
    conn: Arc<Mutex<Connection>>,
}

impl ApiStatePersistence {
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

    /// Load all persisted users.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database or deserialization failure.
    pub async fn load_users(&self) -> Result<Vec<User>, PersistenceError> {
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
    pub async fn save_user(&self, user: &User) -> Result<(), PersistenceError> {
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
    pub async fn delete_user(&self, id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM api_users WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    /// Load all persisted tenants.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError`] on database failure.
    pub async fn load_tenants(&self) -> Result<Vec<Tenant>, PersistenceError> {
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
    pub async fn save_tenant(&self, tenant: &Tenant) -> Result<(), PersistenceError> {
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
    pub async fn delete_tenant(&self, id: &str) -> Result<(), PersistenceError> {
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
    pub async fn load_api_keys(&self) -> Result<Vec<ApiKey>, PersistenceError> {
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
    pub async fn save_api_key(&self, api_key: &ApiKey) -> Result<(), PersistenceError> {
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
    pub async fn delete_api_key(&self, key: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "DELETE FROM api_keys WHERE key = ?1",
            rusqlite::params![key],
        )?;
        Ok(())
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
        let store = ApiStatePersistence::open(&dir.path().join("state.db")).unwrap();

        store.save_user(&test_user("u1")).await.unwrap();
        store.save_user(&test_user("u2")).await.unwrap();
        let users = store.load_users().await.unwrap();
        assert_eq!(users.len(), 2);
        assert!(users
            .iter()
            .any(|u| u.id == "u1" && u.roles == vec!["viewer".to_string()]));

        // Reopen from the same file: state survives.
        drop(store);
        let reopened = ApiStatePersistence::open(&dir.path().join("state.db")).unwrap();
        assert_eq!(reopened.load_users().await.unwrap().len(), 2);

        reopened.delete_user("u1").await.unwrap();
        assert_eq!(reopened.load_users().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn tenants_and_keys_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ApiStatePersistence::open(&dir.path().join("state.db")).unwrap();

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
        let store = ApiStatePersistence::open(&dir.path().join("state.db")).unwrap();
        store.save_user(&test_user("u1")).await.unwrap();
        let mut updated = test_user("u1");
        updated.roles = vec!["admin".to_string()];
        store.save_user(&updated).await.unwrap();
        let users = store.load_users().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].roles, vec!["admin".to_string()]);
    }
}
