use std::sync::Arc;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Errors that can occur during authentication operations.
#[derive(Debug, thiserror::Error)]
pub enum Argon2Error {
    /// Password hashing failed.
    #[error("Password hashing failed: {0}")]
    HashFailed(String),
}

/// Errors from password strength validation.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("Password must be at least {min} characters (got {got})")]
    TooShort { min: usize, got: usize },

    #[error("Password must contain at least one uppercase letter")]
    MissingUppercase,

    #[error("Password must contain at least one lowercase letter")]
    MissingLowercase,

    #[error("Password must contain at least one digit")]
    MissingDigit,

    #[error("Password must contain at least one special character (!@#$%^&*...)")]
    MissingSpecialChar,

    #[error("Password is too common and easily guessed")]
    CommonPassword,
}

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
                "tenant:read".to_string(),
                "tenant:write".to_string(),
                "marketplace:read".to_string(),
                "marketplace:write".to_string(),
                "audit:read".to_string(),
            ],
        });
        manager.add_role(Role {
            name: "editor".to_string(),
            permissions: vec![
                "crawl:read".to_string(),
                "crawl:write".to_string(),
                "report:read".to_string(),
                "report:write".to_string(),
                "marketplace:read".to_string(),
            ],
        });
        manager.add_role(Role {
            name: "viewer".to_string(),
            permissions: vec![
                "crawl:read".to_string(),
                "report:read".to_string(),
                "marketplace:read".to_string(),
            ],
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

    /// Validate password complexity (delegates to `validate_password_strength`).
    /// Returns Ok(()) if valid, Err(String) with reason if invalid.
    pub fn validate_password(password: &str) -> Result<(), String> {
        validate_password_strength(password).map_err(|e| e.to_string())
    }

    /// Verify password.
    pub fn verify_password(&self, password: &str, hash: &str) -> bool {
        salting::verify_password(password, hash).unwrap_or(false)
    }

    /// Hash a password.
    pub fn hash_password(&self, password: &str) -> Result<String, Argon2Error> {
        salting::hash_password(password).map_err(|e| Argon2Error::HashFailed(e.to_string()))
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
    pub fn has_permission(&self, claims: &Claims, permission: &str) -> bool {
        claims.permissions.contains(&permission.to_string())
    }
}

/// Validate password strength against defense-sector standards.
///
/// Delegates to [`salting::check_password`] with the default
/// [`salting::Policy`] (12+ chars, uppercase, lowercase, digit, special):
///
/// - Policy failures map 1:1 onto [`PasswordError`] variants.
/// - The former hardcoded common-password list is replaced by zxcvbn's
///   entropy-based estimation: a password that passes the policy but
///   scores 0-1 on guessability is rejected as
///   [`PasswordError::CommonPassword`]. This subsumes (and strengthens)
///   the old 25-entry list, which only caught exact matches.
pub fn validate_password_strength(password: &str) -> Result<(), PasswordError> {
    if let Err(policy_err) = salting::check_password(password, &salting::Policy::default(), &[]) {
        return Err(match policy_err {
            salting::PolicyError::TooShort { min, got } => PasswordError::TooShort { min, got },
            salting::PolicyError::MissingUppercase => PasswordError::MissingUppercase,
            salting::PolicyError::MissingLowercase => PasswordError::MissingLowercase,
            salting::PolicyError::MissingDigit => PasswordError::MissingDigit,
            salting::PolicyError::MissingSpecialChar => PasswordError::MissingSpecialChar,
        });
    }

    // Entropy-based common detection: any password trivially guessable
    // (score <= 1) is treated as a common password.
    if salting::strength(password, &[]).score <= 1 {
        return Err(PasswordError::CommonPassword);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(id: &str, email: &str, roles: Vec<&str>) -> User {
        User {
            id: id.to_string(),
            email: email.to_string(),
            name: format!("User {id}"),
            password_hash: String::new(),
            tenant_id: "default".to_string(),
            roles: roles.into_iter().map(String::from).collect(),
            enabled: true,
        }
    }

    // ---------------------------------------------------------------
    // AuthManager::new – default roles
    // ---------------------------------------------------------------

    #[test]
    fn test_auth_manager_new_has_default_roles() {
        let am = AuthManager::new("secret".to_string());
        let roles = am.roles.read();
        let names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"admin"));
        assert!(names.contains(&"editor"));
        assert!(names.contains(&"viewer"));
    }

    #[test]
    fn test_auth_manager_new_empty_users() {
        let am = AuthManager::new("secret".to_string());
        assert!(am.users.read().is_empty());
    }

    // ---------------------------------------------------------------
    // add_role / add_user / find_user / find_user_by_id
    // ---------------------------------------------------------------

    #[test]
    fn test_add_and_find_user_by_email() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u1", "alice@example.com", vec!["admin"]);
        am.add_user(user);
        let found = am.find_user("alice@example.com");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "u1");
    }

    #[test]
    fn test_find_user_not_found() {
        let am = AuthManager::new("secret".to_string());
        assert!(am.find_user("nobody@example.com").is_none());
    }

    #[test]
    fn test_add_and_find_user_by_id() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u42", "bob@example.com", vec!["viewer"]);
        am.add_user(user);
        let found = am.find_user_by_id("u42");
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "bob@example.com");
    }

    #[test]
    fn test_find_user_by_id_not_found() {
        let am = AuthManager::new("secret".to_string());
        assert!(am.find_user_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_list_users() {
        let am = AuthManager::new("secret".to_string());
        am.add_user(make_user("u1", "a@b.com", vec!["admin"]));
        am.add_user(make_user("u2", "c@d.com", vec!["viewer"]));
        let users = am.list_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_list_users_empty() {
        let am = AuthManager::new("secret".to_string());
        assert!(am.list_users().is_empty());
    }

    // ---------------------------------------------------------------
    // delete_user
    // ---------------------------------------------------------------

    #[test]
    fn test_delete_user_existing() {
        let am = AuthManager::new("secret".to_string());
        am.add_user(make_user("u1", "a@b.com", vec!["admin"]));
        assert!(am.delete_user("u1"));
        assert!(am.find_user_by_id("u1").is_none());
    }

    #[test]
    fn test_delete_user_nonexistent() {
        let am = AuthManager::new("secret".to_string());
        assert!(!am.delete_user("nope"));
    }

    // ---------------------------------------------------------------
    // add_role
    // ---------------------------------------------------------------

    #[test]
    fn test_add_custom_role() {
        let am = AuthManager::new("secret".to_string());
        am.add_role(Role {
            name: "custom".to_string(),
            permissions: vec!["custom:read".to_string()],
        });
        let roles = am.roles.read();
        let names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"custom"));
    }

    // ---------------------------------------------------------------
    // Password hashing / verification
    // ---------------------------------------------------------------

    #[test]
    fn test_hash_and_verify_password_correct() {
        let am = AuthManager::new("secret".to_string());
        let hash = am.hash_password("my_secure_password").unwrap();
        assert!(am.verify_password("my_secure_password", &hash));
    }

    #[test]
    fn test_hash_and_verify_password_wrong() {
        let am = AuthManager::new("secret".to_string());
        let hash = am.hash_password("correct_password").unwrap();
        assert!(!am.verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let am = AuthManager::new("secret".to_string());
        assert!(!am.verify_password("anything", "not-a-hash"));
    }

    #[test]
    fn test_hash_password_different_each_time() {
        let am = AuthManager::new("secret".to_string());
        let h1 = am.hash_password("password").unwrap();
        let h2 = am.hash_password("password").unwrap();
        assert_ne!(h1, h2, "Argon2 hashes should use different salts");
    }

    // ---------------------------------------------------------------
    // JWT generate / validate
    // ---------------------------------------------------------------

    #[test]
    fn test_generate_and_validate_token() {
        let am = AuthManager::new("test-secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["admin"]);
        let token = am.generate_token(&user).unwrap();
        let claims = am.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "u1");
        assert_eq!(claims.tenant, "default");
        assert!(claims.roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_validate_token_wrong_secret() {
        let am = AuthManager::new("correct-secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["viewer"]);
        let token = am.generate_token(&user).unwrap();

        let wrong_am = AuthManager::new("wrong-secret".to_string());
        assert!(wrong_am.validate_token(&token).is_err());
    }

    #[test]
    fn test_validate_token_garbage() {
        let am = AuthManager::new("secret".to_string());
        assert!(am.validate_token("garbage.token.value").is_err());
    }

    #[test]
    fn test_generate_token_includes_permissions() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["editor"]);
        let token = am.generate_token(&user).unwrap();
        let claims = am.validate_token(&token).unwrap();
        // editor role has crawl:read, crawl:write, report:read, report:write
        assert!(claims.permissions.contains(&"crawl:read".to_string()));
        assert!(claims.permissions.contains(&"crawl:write".to_string()));
        assert!(claims.permissions.contains(&"report:read".to_string()));
        assert!(claims.permissions.contains(&"report:write".to_string()));
    }

    #[test]
    fn test_viewer_role_limited_permissions() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["viewer"]);
        let token = am.generate_token(&user).unwrap();
        let claims = am.validate_token(&token).unwrap();
        assert!(claims.permissions.contains(&"crawl:read".to_string()));
        assert!(!claims.permissions.contains(&"crawl:write".to_string()));
        assert!(!claims.permissions.contains(&"user:read".to_string()));
    }

    // ---------------------------------------------------------------
    // has_permission
    // ---------------------------------------------------------------

    #[test]
    fn test_has_permission_true() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["admin"]);
        let token = am.generate_token(&user).unwrap();
        let claims = am.validate_token(&token).unwrap();
        assert!(am.has_permission(&claims, "crawl:read"));
        assert!(am.has_permission(&claims, "user:write"));
    }

    #[test]
    fn test_has_permission_false() {
        let am = AuthManager::new("secret".to_string());
        let user = make_user("u1", "a@b.com", vec!["viewer"]);
        let token = am.generate_token(&user).unwrap();
        let claims = am.validate_token(&token).unwrap();
        assert!(!am.has_permission(&claims, "user:write"));
        assert!(!am.has_permission(&claims, "apikey:read"));
    }

    // ---------------------------------------------------------------
    // validate_password / validate_password_strength
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_password_valid() {
        assert!(AuthManager::validate_password("Str0ng!Pass#12").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        let err = AuthManager::validate_password("Sh0rt!xY").unwrap_err();
        assert!(err.contains("12 characters"));
    }

    #[test]
    fn test_validate_password_no_uppercase() {
        let err = AuthManager::validate_password("nouppercase12!x").unwrap_err();
        assert!(err.contains("uppercase"));
    }

    #[test]
    fn test_validate_password_no_lowercase() {
        let err = AuthManager::validate_password("NOLOWERCASE12!X").unwrap_err();
        assert!(err.contains("lowercase"));
    }

    #[test]
    fn test_validate_password_no_digit() {
        let err = AuthManager::validate_password("NoDigitHere!xY").unwrap_err();
        assert!(err.contains("digit"));
    }

    #[test]
    fn test_validate_password_no_special_char() {
        let err = AuthManager::validate_password("NoSpecialChar12X").unwrap_err();
        assert!(err.contains("special character"));
    }

    #[test]
    fn test_validate_password_common() {
        // Passes the composition policy but is trivially guessable
        // (zxcvbn score <= 1) → CommonPassword.
        let err = AuthManager::validate_password("Password123!").unwrap_err();
        assert!(err.contains("common"));
    }

    #[test]
    fn test_validate_password_common_variant() {
        // Entropy-based detection: trivially guessable variants of common
        // passwords are still caught (score <= 1). Unlike the old hardcoded
        // list, this generalizes beyond exact matches, though high-entropy
        // case variants (e.g. "PassWORD!123") now pass.
        let err = AuthManager::validate_password("Password123!!").unwrap_err();
        assert!(err.contains("common"));
    }

    #[test]
    fn test_validate_password_weak_still_rejected() {
        // Policy-first ordering: "password" fails on length before the
        // common check, but is still rejected.
        let err = AuthManager::validate_password("password").unwrap_err();
        assert!(err.contains("12 characters"));
    }

    // ---------------------------------------------------------------
    // validate_password_strength (standalone function)
    // ---------------------------------------------------------------

    #[test]
    fn test_password_strength_valid() {
        assert!(validate_password_strength("MyS3cure!Pass").is_ok());
    }

    #[test]
    fn test_password_strength_too_short() {
        let err = validate_password_strength("Ab1!cdefgh").unwrap_err();
        assert!(matches!(err, PasswordError::TooShort { min: 12, got: 10 }));
    }

    #[test]
    fn test_password_strength_missing_uppercase() {
        assert!(matches!(
            validate_password_strength("nospecial!1x"),
            Err(PasswordError::MissingUppercase)
        ));
    }

    #[test]
    fn test_password_strength_missing_lowercase() {
        assert!(matches!(
            validate_password_strength("NOSPECIAL!1X"),
            Err(PasswordError::MissingLowercase)
        ));
    }

    #[test]
    fn test_password_strength_missing_digit() {
        assert!(matches!(
            validate_password_strength("NoDigit!abcX"),
            Err(PasswordError::MissingDigit)
        ));
    }

    #[test]
    fn test_password_strength_missing_special() {
        assert!(matches!(
            validate_password_strength("NoSpecial123X"),
            Err(PasswordError::MissingSpecialChar)
        ));
    }

    #[test]
    fn test_password_strength_common_password() {
        // Passes the policy (12 chars, all classes) but scores <= 1 on
        // guessability → CommonPassword replaces the old hardcoded list.
        assert!(matches!(
            validate_password_strength("Password123!"),
            Err(PasswordError::CommonPassword)
        ));
    }

    #[test]
    fn test_password_strength_weak_short_fails_fast_on_policy() {
        // "password" is weak AND short; the policy runs first so the
        // error is TooShort, not CommonPassword.
        assert!(matches!(
            validate_password_strength("password"),
            Err(PasswordError::TooShort { min: 12, got: 8 })
        ));
    }

    #[test]
    fn test_password_strength_passphrase_passes_with_relaxed_policy() {
        // A long passphrase carries enough entropy without character-class
        // gymnastics; salting::Policy can express that, and zxcvbn agrees
        // it is strong.
        let policy = salting::Policy::default()
            .require_uppercase(false)
            .require_lowercase(false)
            .require_digit(false)
            .require_special(false);
        assert!(salting::check_password("correcthorsebatterystaple", &policy, &[]).is_ok());
        assert!(salting::strength("correcthorsebatterystaple", &[]).score >= 3);
        // The default crawlkit policy still requires classes, so this
        // passphrase needs the relaxed one:
        assert!(validate_password_strength("Correct-Horse42!Battery").is_ok());
    }

    #[test]
    fn test_password_strength_exactly_12_chars_valid() {
        assert!(validate_password_strength("Abcdef1!xyz0").is_ok());
    }

    #[test]
    fn test_password_strength_all_special_chars() {
        assert!(validate_password_strength("!@Abcdef1234").is_ok());
        assert!(validate_password_strength("#$Abcdef1234").is_ok());
        assert!(validate_password_strength("^&Abcdef1234").is_ok());
    }

    #[test]
    fn test_password_strength_11_chars_rejected() {
        let err = validate_password_strength("Abc1!efghij").unwrap_err();
        assert!(matches!(err, PasswordError::TooShort { min: 12, got: 11 }));
    }

    // ---------------------------------------------------------------
    // Claims serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_claims_serialization_roundtrip() {
        let claims = Claims {
            sub: "user-1".to_string(),
            tenant: "acme".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["crawl:read".to_string()],
            exp: 1700000000,
            iat: 1699996400,
            jti: "unique-id".to_string(),
        };
        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-1");
        assert_eq!(deserialized.tenant, "acme");
        assert_eq!(deserialized.jti, "unique-id");
    }
}
