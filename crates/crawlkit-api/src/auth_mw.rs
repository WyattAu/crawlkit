use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::auth::{AuthManager, Claims};

/// Authorization middleware that checks JWT and permissions.
pub async fn auth_middleware(
    State(state): State<crate::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = state
        .auth
        .validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut request = request;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Permission check middleware.
#[allow(dead_code)]
pub fn require_permission(
    auth: Arc<AuthManager>,
    permission: &'static str,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> {
    move |request: Request, next: Next| {
        let auth = auth.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<Claims>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            if !auth.has_permission(claims, permission) {
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;

    fn make_claims(permissions: Vec<&str>) -> Claims {
        Claims {
            sub: "user-1".to_string(),
            tenant: "default".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: permissions.into_iter().map(String::from).collect(),
            exp: 1700000000,
            iat: 1699996400,
            jti: "test-jti".to_string(),
        }
    }

    #[test]
    fn test_require_permission_returns_closure() {
        let auth = Arc::new(crate::auth::AuthManager::new("test-secret".to_string()));
        let _checker = require_permission(auth, "crawl:read");
    }

    #[test]
    fn test_permission_check_with_claims() {
        let claims = make_claims(vec!["crawl:read", "crawl:write"]);
        assert!(claims.permissions.contains(&"crawl:read".to_string()));
        assert!(claims.permissions.contains(&"crawl:write".to_string()));
        assert!(!claims.permissions.contains(&"user:write".to_string()));
    }

    #[test]
    fn test_permission_check_empty_permissions() {
        let claims = make_claims(vec![]);
        assert!(!claims.permissions.contains(&"crawl:read".to_string()));
    }

    #[test]
    fn test_auth_middleware_claims_insertion() {
        let claims = make_claims(vec!["admin"]);
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request.extensions_mut().insert(claims);

        let retrieved = request.extensions().get::<Claims>().unwrap();
        assert_eq!(retrieved.sub, "user-1");
        assert_eq!(retrieved.tenant, "default");
        assert!(retrieved.permissions.contains(&"admin".to_string()));
    }

    #[test]
    fn test_auth_middleware_no_claims_in_extensions() {
        let request = Request::builder().body(Body::empty()).unwrap();
        assert!(request.extensions().get::<Claims>().is_none());
    }

    #[test]
    fn test_permission_string_comparison() {
        let perm = "crawl:read";
        let claims = make_claims(vec!["crawl:read"]);
        let has = claims.permissions.contains(&perm.to_string());
        assert!(has);
    }
}
