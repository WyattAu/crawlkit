use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Tenant configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Tenant ID.
    pub id: String,
    /// Tenant name.
    pub name: String,
    /// Tenant plan (free, pro, enterprise).
    pub plan: TenantPlan,
    /// Maximum crawls per month.
    pub max_crawls: u64,
    /// Maximum pages per crawl.
    pub max_pages: usize,
    /// Maximum concurrent crawls.
    pub max_concurrent: usize,
    /// API rate limit (requests per minute).
    pub rate_limit: u32,
    /// Features enabled for this tenant.
    pub features: Vec<String>,
}

/// Tenant plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantPlan {
    Free,
    Pro,
    Enterprise,
}

/// Tenant manager for multi-tenant support.
pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
}

impl TenantManager {
    /// Create new tenant manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a tenant.
    pub fn add_tenant(&self, tenant: Tenant) {
        let mut tenants = self.tenants.write();
        tenants.insert(tenant.id.clone(), tenant);
    }

    /// Get a tenant by ID.
    #[must_use]
    pub fn get_tenant(&self, id: &str) -> Option<Tenant> {
        self.tenants.read().get(id).cloned()
    }

    /// List all tenants.
    #[must_use]
    pub fn list_tenants(&self) -> Vec<Tenant> {
        self.tenants.read().values().cloned().collect()
    }

    /// Remove a tenant.
    pub fn remove_tenant(&self, id: &str) -> bool {
        self.tenants.write().remove(id).is_some()
    }

    /// Get tenant count.
    #[must_use]
    pub fn count(&self) -> usize {
        self.tenants.read().len()
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Role definitions for RBAC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role ID.
    pub id: String,
    /// Role name.
    pub name: String,
    /// Permissions granted by this role.
    pub permissions: Vec<String>,
}

/// Permission definitions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Create crawls.
    CrawlCreate,
    /// Read crawl results.
    CrawlRead,
    /// Delete crawls.
    CrawlDelete,
    /// Manage API keys.
    ApiKeyManage,
    /// View analytics.
    AnalyticsView,
    /// Manage users.
    UserManage,
    /// Manage tenants.
    TenantManage,
    /// View audit logs.
    AuditView,
    /// Manage billing.
    BillingManage,
}

impl Permission {
    /// Get permission string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::CrawlCreate => "crawl:create",
            Permission::CrawlRead => "crawl:read",
            Permission::CrawlDelete => "crawl:delete",
            Permission::ApiKeyManage => "apikey:manage",
            Permission::AnalyticsView => "analytics:view",
            Permission::UserManage => "user:manage",
            Permission::TenantManage => "tenant:manage",
            Permission::AuditView => "audit:view",
            Permission::BillingManage => "billing:manage",
        }
    }

    /// Parse a permission from its string representation.
    ///
    /// Returns `None` if the string does not correspond to a known permission.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "crawl:create" => Some(Permission::CrawlCreate),
            "crawl:read" => Some(Permission::CrawlRead),
            "crawl:delete" => Some(Permission::CrawlDelete),
            "apikey:manage" => Some(Permission::ApiKeyManage),
            "analytics:view" => Some(Permission::AnalyticsView),
            "user:manage" => Some(Permission::UserManage),
            "tenant:manage" => Some(Permission::TenantManage),
            "audit:view" => Some(Permission::AuditView),
            "billing:manage" => Some(Permission::BillingManage),
            _ => None,
        }
    }
}

/// User with RBAC support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID.
    pub id: String,
    /// User email.
    pub email: String,
    /// User display name.
    pub name: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// User roles.
    pub roles: Vec<String>,
    /// Whether user is active.
    pub active: bool,
}

/// RBAC manager for role-based access control.
pub struct RbacManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl RbacManager {
    /// Create new RBAC manager.
    #[must_use]
    pub fn new() -> Self {
        let mut roles = HashMap::new();

        // Default roles
        roles.insert(
            "admin".to_string(),
            Role {
                id: "admin".to_string(),
                name: "Administrator".to_string(),
                permissions: Permission::iter().map(|p| p.as_str().to_string()).collect(),
            },
        );

        roles.insert(
            "user".to_string(),
            Role {
                id: "user".to_string(),
                name: "User".to_string(),
                permissions: vec![
                    Permission::CrawlCreate.as_str().to_string(),
                    Permission::CrawlRead.as_str().to_string(),
                    Permission::AnalyticsView.as_str().to_string(),
                ],
            },
        );

        roles.insert(
            "viewer".to_string(),
            Role {
                id: "viewer".to_string(),
                name: "Viewer".to_string(),
                permissions: vec![
                    Permission::CrawlRead.as_str().to_string(),
                    Permission::AnalyticsView.as_str().to_string(),
                ],
            },
        );

        Self {
            roles: Arc::new(RwLock::new(roles)),
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a role.
    pub fn add_role(&self, role: Role) {
        let mut roles = self.roles.write();
        roles.insert(role.id.clone(), role);
    }

    /// Get a role by ID.
    #[must_use]
    pub fn get_role(&self, id: &str) -> Option<Role> {
        self.roles.read().get(id).cloned()
    }

    /// Add a user.
    pub fn add_user(&self, user: User) {
        let mut users = self.users.write();
        users.insert(user.id.clone(), user);
    }

    /// Get a user by ID.
    #[must_use]
    pub fn get_user(&self, id: &str) -> Option<User> {
        self.users.read().get(id).cloned()
    }

    /// Check if user has permission.
    #[must_use]
    pub fn has_permission(&self, user_id: &str, permission: &Permission) -> bool {
        let users = self.users.read();
        let roles = self.roles.read();

        if let Some(user) = users.get(user_id) {
            if !user.active {
                return false;
            }

            for role_id in &user.roles {
                if let Some(role) = roles.get(role_id) {
                    if role.permissions.contains(&permission.as_str().to_string()) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all permissions for a user.
    #[must_use]
    pub fn get_permissions(&self, user_id: &str) -> Vec<String> {
        let users = self.users.read();
        let roles = self.roles.read();
        let mut permissions = Vec::new();

        if let Some(user) = users.get(user_id) {
            if !user.active {
                return permissions;
            }

            for role_id in &user.roles {
                if let Some(role) = roles.get(role_id) {
                    permissions.extend(role.permissions.clone());
                }
            }
        }

        permissions.sort();
        permissions.dedup();
        permissions
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SSO configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// SSO provider (saml, oidc).
    pub provider: SsoProvider,
    /// Provider URL.
    pub provider_url: String,
    /// Client ID.
    pub client_id: String,
    /// Client secret.
    pub client_secret: String,
    /// Callback URL.
    pub callback_url: String,
    /// Enabled SSO domains.
    pub enabled_domains: Vec<String>,
}

/// SSO provider type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SsoProvider {
    Saml,
    Oidc,
}

/// SSO manager for enterprise authentication.
pub struct SsoManager {
    configs: Arc<RwLock<Vec<SsoConfig>>>,
}

impl SsoManager {
    /// Create new SSO manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add SSO configuration.
    pub fn add_config(&self, config: SsoConfig) {
        let mut configs = self.configs.write();
        configs.push(config);
    }

    /// Get SSO config for a domain.
    #[must_use]
    pub fn get_config_for_domain(&self, domain: &str) -> Option<SsoConfig> {
        self.configs
            .read()
            .iter()
            .find(|c| c.enabled_domains.contains(&domain.to_string()))
            .cloned()
    }

    /// List all SSO configs.
    #[must_use]
    pub fn list_configs(&self) -> Vec<SsoConfig> {
        self.configs.read().clone()
    }
}

impl Default for SsoManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SLA monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaConfig {
    /// Uptime target (percentage).
    pub uptime_target: f64,
    /// Response time target (ms).
    pub response_time_target: u64,
    /// Error rate target (percentage).
    pub error_rate_target: f64,
    /// Alert thresholds.
    pub alert_thresholds: SlaThresholds,
}

/// SLA alert thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaThresholds {
    /// Warning threshold (percentage of target).
    pub warning: f64,
    /// Critical threshold (percentage of target).
    pub critical: f64,
}

impl Default for SlaConfig {
    fn default() -> Self {
        Self {
            uptime_target: 99.9,
            response_time_target: 500,
            error_rate_target: 1.0,
            alert_thresholds: SlaThresholds {
                warning: 0.95,
                critical: 0.90,
            },
        }
    }
}

/// SLA monitor for tracking compliance.
pub struct SlaMonitor {
    config: SlaConfig,
    metrics: Arc<RwLock<SlaMetrics>>,
}

/// Current SLA metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlaMetrics {
    /// Total requests.
    pub total_requests: u64,
    /// Successful requests.
    pub successful_requests: u64,
    /// Failed requests.
    pub failed_requests: u64,
    /// Total response time (ms).
    pub total_response_time: u64,
    /// Uptime percentage.
    pub uptime: f64,
    /// Average response time (ms).
    pub avg_response_time: f64,
    /// Error rate (percentage).
    pub error_rate: f64,
}

impl SlaMonitor {
    /// Create new SLA monitor.
    #[must_use]
    pub fn new(config: SlaConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(RwLock::new(SlaMetrics::default())),
        }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(SlaConfig::default())
    }

    /// Record a successful request.
    pub fn record_success(&self, response_time_ms: u64) {
        let mut metrics = self.metrics.write();
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_response_time += response_time_ms;
        self.update_metrics(&mut metrics);
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        let mut metrics = self.metrics.write();
        metrics.total_requests += 1;
        metrics.failed_requests += 1;
        self.update_metrics(&mut metrics);
    }

    /// Update calculated metrics.
    fn update_metrics(&self, metrics: &mut SlaMetrics) {
        if metrics.total_requests > 0 {
            metrics.error_rate =
                (metrics.failed_requests as f64 / metrics.total_requests as f64) * 100.0;
            metrics.avg_response_time =
                metrics.total_response_time as f64 / metrics.total_requests as f64;
        }
    }

    /// Get current metrics.
    #[must_use]
    pub fn metrics(&self) -> SlaMetrics {
        self.metrics.read().clone()
    }

    /// Check if SLA is met.
    #[must_use]
    pub fn is_sla_met(&self) -> bool {
        let metrics = self.metrics.read();
        metrics.error_rate <= self.config.error_rate_target
            && metrics.avg_response_time <= self.config.response_time_target as f64
    }

    /// Get SLA status.
    #[must_use]
    pub fn status(&self) -> SlaStatus {
        let metrics = self.metrics.read();
        let error_rate_ratio = metrics.error_rate / self.config.error_rate_target;
        let response_time_ratio =
            metrics.avg_response_time / self.config.response_time_target as f64;

        let worst_ratio = error_rate_ratio.max(response_time_ratio);

        if worst_ratio <= self.config.alert_thresholds.critical {
            SlaStatus::Critical
        } else if worst_ratio <= self.config.alert_thresholds.warning {
            SlaStatus::Warning
        } else {
            SlaStatus::Healthy
        }
    }

    /// Get configuration.
    #[must_use]
    pub fn config(&self) -> &SlaConfig {
        &self.config
    }
}

impl Default for SlaMonitor {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// SLA status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlaStatus {
    Healthy,
    Warning,
    Critical,
}

/// Helper trait for iterating permissions.
trait PermissionIter {
    fn iter() -> Box<dyn Iterator<Item = Permission>>;
}

impl PermissionIter for Permission {
    fn iter() -> Box<dyn Iterator<Item = Permission>> {
        Box::new(
            vec![
                Permission::CrawlCreate,
                Permission::CrawlRead,
                Permission::CrawlDelete,
                Permission::ApiKeyManage,
                Permission::AnalyticsView,
                Permission::UserManage,
                Permission::TenantManage,
                Permission::AuditView,
                Permission::BillingManage,
            ]
            .into_iter(),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_manager() {
        let manager = TenantManager::new();

        let tenant = Tenant {
            id: "tenant1".to_string(),
            name: "Acme Corp".to_string(),
            plan: TenantPlan::Enterprise,
            max_crawls: 1000,
            max_pages: 10000,
            max_concurrent: 10,
            rate_limit: 60,
            features: vec!["backlinks".to_string(), "rum".to_string()],
        };

        manager.add_tenant(tenant);
        assert_eq!(manager.count(), 1);
        assert!(manager.get_tenant("tenant1").is_some());
    }

    #[test]
    fn test_rbac_manager() {
        let manager = RbacManager::new();

        let user = User {
            id: "user1".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            tenant_id: "tenant1".to_string(),
            roles: vec!["admin".to_string()],
            active: true,
        };

        manager.add_user(user);

        // Admin has all permissions
        assert!(manager.has_permission("user1", &Permission::CrawlCreate));
        assert!(manager.has_permission("user1", &Permission::TenantManage));
        assert!(manager.has_permission("user1", &Permission::BillingManage));

        // Test non-existent user
        assert!(!manager.has_permission("nonexistent", &Permission::CrawlCreate));
    }

    #[test]
    fn test_sla_monitor() {
        let monitor = SlaMonitor::with_default_config();

        // Record some successful requests
        for _ in 0..90 {
            monitor.record_success(100);
        }

        // Record some failures
        for _ in 0..10 {
            monitor.record_failure();
        }

        let metrics = monitor.metrics();
        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.successful_requests, 90);
        assert_eq!(metrics.failed_requests, 10);
        assert!((metrics.error_rate - 10.0).abs() < 0.01);
    }
}
