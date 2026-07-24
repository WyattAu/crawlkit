use std::sync::Arc;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

/// User in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub enabled: bool,
}

/// Role definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<String>,
}

/// Auth manager handling JWT and RBAC.
pub struct AuthManager {
    users: Arc<RwLock<Vec<User>>>,
    roles: Arc<RwLock<Vec<Role>>>,
    jwt_secret: String,
    token_expiry_hours: u64,
}

impl AuthManager {
    /// Create new auth manager.
    pub fn new(jwt_secret: String) -> Self {
        let manager = Self {
            users: Arc::new(RwLock::new(Vec::new())),
            roles: Arc::new(RwLock::new(Vec::new())),
            jwt_secret,
            token_expiry_hours: 1,
        };

        manager.add_role(Role {
            name: "admin".to_string(),
            permissions: vec![
                "crawl:read".to_string(),
                "crawl:write".to_string(),
                "crawl:delete".to_string(),
                "report:read".to_string(),
                "report:write".to_string(),
                "user:read".to_string(),
                "user:write".to_string(),
                "role:read".to_string(),
                "role:write".to_string(),
                "apikey:read".to_string(),
                "apikey:write".to_string(),
            ],
        });
        manager.add_role(Role {
            name: "editor".to_string(),
            permissions: vec![
                "crawl:read".to_string(),
                "crawl:write".to_string(),
                "report:read".to_string(),
                "report:write".to_string(),
            ],
        });
        manager.add_role(Role {
            name: "viewer".to_string(),
            permissions: vec!["crawl:read".to_string(), "report:read".to_string()],
        });

        manager
    }

    /// Add a role.
    pub fn add_role(&self, role: Role) {
        self.roles.write().push(role);
    }

    /// Add a user.
    pub fn add_user(&self, user: User) {
        self.users.write().push(user);
    }

    /// Find user by email.
    pub fn find_user(&self, email: &str) -> Option<User> {
        self.users.read().iter().find(|u| u.email == email).cloned()
    }

    /// Find user by ID.
    pub fn find_user_by_id(&self, id: &str) -> Option<User> {
        self.users.read().iter().find(|u| u.id == id).cloned()
    }

    /// List all users.
    pub fn list_users(&self) -> Vec<User> {
        self.users.read().clone()
    }

    /// Delete a user.
    pub fn delete_user(&self, id: &str) -> bool {
        let mut users = self.users.write();
        let len_before = users.len();
        users.retain(|u| u.id != id);
        users.len() < len_before
    }

    /// Verify password.
    pub fn verify_password(&self, password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Hash a password.
    #[allow(clippy::expect_used)]
    pub fn hash_password(&self, password: &str) -> String {
        use rand::rngs::OsRng;

        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("argon2 hashing should not fail");
        hash.to_string()
    }

    /// Generate JWT token.
    pub fn generate_token(&self, user: &User) -> Result<String, jsonwebtoken::errors::Error> {
        let role_permissions: Vec<String> = self
            .roles
            .read()
            .iter()
            .filter(|r| user.roles.contains(&r.name))
            .flat_map(|r| r.permissions.clone())
            .collect();

        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: user.id.clone(),
            tenant: user.tenant_id.clone(),
            roles: user.roles.clone(),
            permissions: role_permissions,
            exp: now + (self.token_expiry_hours as usize * 3600),
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
    }

    /// Validate JWT token.
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    /// Check if user has a specific permission.
    #[allow(dead_code)]
    pub fn has_permission(&self, claims: &Claims, permission: &str) -> bool {
        claims.permissions.contains(&permission.to_string())
    }
}
