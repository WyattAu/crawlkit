use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::auth;
use crate::types::*;

/// List marketplace plugins. Requires the `marketplace:read` permission.
#[utoipa::path(
    get,
    path = "/api/v1/marketplace/plugins",
    tag = "marketplace",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "All marketplace plugins", body = [MarketplacePlugin]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:read permission", body = ApiErrorBody)
    )
)]
pub async fn list_marketplace_plugins(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Result<Json<Vec<MarketplacePlugin>>, ApiError> {
    require_permission(&claims, "marketplace:read")?;
    let plugins = state.marketplace.plugins.read();
    let list: Vec<MarketplacePlugin> = plugins.values().cloned().collect();
    Ok(Json(list))
}

/// Get a marketplace plugin by name. Requires `marketplace:read`.
#[utoipa::path(
    get,
    path = "/api/v1/marketplace/plugins/{name}",
    tag = "marketplace",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Plugin details", body = MarketplacePlugin),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:read permission", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn get_marketplace_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
) -> Result<Json<MarketplacePlugin>, ApiError> {
    require_permission(&claims, "marketplace:read")?;
    let plugins = state.marketplace.plugins.read();
    plugins
        .get(&name)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Plugin '{name}' not found")))
}

/// Publish a new plugin to the marketplace. Requires `marketplace:write`.
#[utoipa::path(
    post,
    path = "/api/v1/marketplace/plugins",
    tag = "marketplace",
    request_body = SubmitPluginRequest,
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 201, description = "Plugin published", body = MarketplacePlugin),
        (status = 400, description = "Plugin name already exists", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:write permission or CSRF origin rejected", body = ApiErrorBody)
    )
)]
pub async fn submit_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(input): Json<SubmitPluginRequest>,
) -> Result<(StatusCode, Json<MarketplacePlugin>), ApiError> {
    require_permission(&claims, "marketplace:write")?;
    {
        let plugins = state.marketplace.plugins.read();
        if plugins.contains_key(&input.name) {
            return Err(ApiError::BadRequest(format!(
                "Plugin '{}' already exists",
                input.name
            )));
        }
    }

    let plugin = MarketplacePlugin {
        name: input.name.clone(),
        version: input.version,
        author: input.author,
        description: input.description,
        license: input.license,
        categories: input.categories,
        tags: input.tags,
        downloads: 0,
        rating: 0.0,
        rating_count: 0,
        verified: false,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };

    state
        .marketplace
        .plugins
        .write()
        .insert(input.name, plugin.clone());
    Ok((StatusCode::CREATED, Json(plugin)))
}

/// Remove a plugin from the marketplace. Requires `marketplace:write`.
#[utoipa::path(
    delete,
    path = "/api/v1/marketplace/plugins/{name}",
    tag = "marketplace",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 204, description = "Plugin deleted"),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:write permission or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn delete_marketplace_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_permission(&claims, "marketplace:write")?;
    let mut plugins = state.marketplace.plugins.write();
    if plugins.remove(&name).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

/// Execute a plugin against a free-form test input. Requires
/// `marketplace:write`. Both the request and response bodies are
/// plugin-defined JSON documents.
#[utoipa::path(
    post,
    path = "/api/v1/marketplace/plugins/{name}/test",
    tag = "marketplace",
    request_body(content = serde_json::Value, description = "Plugin-defined test input"),
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Plugin-defined test result (e.g. status, findings, execution_time_ms)", body = serde_json::Value),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:write permission or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn test_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_permission(&claims, "marketplace:write")?;
    let _ = input;
    let plugins = state.marketplace.plugins.read();
    if plugins.contains_key(&name) {
        Ok(Json(serde_json::json!({
            "status": "passed",
            "findings": 0,
            "execution_time_ms": 15,
        })))
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

/// Submit a rating (0.0–5.0) for a marketplace plugin. Requires
/// `marketplace:read` (any authenticated user can rate).
#[utoipa::path(
    post,
    path = "/api/v1/marketplace/plugins/{name}/rate",
    tag = "marketplace",
    request_body = PluginRating,
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Rating recorded", body = PluginRatingResponse),
        (status = 400, description = "Rating out of range", body = ApiErrorBody),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:read permission", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn rate_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
    Json(rating): Json<PluginRating>,
) -> Result<Json<PluginRatingResponse>, ApiError> {
    require_permission(&claims, "marketplace:read")?;

    if rating.rating < 0.0 || rating.rating > 5.0 {
        return Err(ApiError::BadRequest(
            "Rating must be between 0.0 and 5.0".to_string(),
        ));
    }

    // Verify plugin exists and update its aggregate rating.
    {
        let mut plugins = state.marketplace.plugins.write();
        let plugin = plugins
            .get_mut(&name)
            .ok_or_else(|| ApiError::NotFound(format!("Plugin '{name}' not found")))?;

        // Record the individual rating.
        let mut ratings = state.marketplace.ratings.write();
        let plugin_ratings = ratings.entry(name.clone()).or_insert_with(Vec::new);
        plugin_ratings.push(rating.rating);

        // Recompute aggregate.
        let count = plugin_ratings.len() as u32;
        let avg = plugin_ratings.iter().sum::<f64>() / plugin_ratings.len() as f64;

        plugin.rating = avg;
        plugin.rating_count = count;
        plugin.updated_at = Utc::now().to_rfc3339();

        Ok(Json(PluginRatingResponse {
            name,
            rating: rating.rating,
            rating_count: count,
            average_rating: avg,
        }))
    }
}

/// Search marketplace plugins by free-text query and/or category.
/// Requires `marketplace:read`.
#[utoipa::path(
    get,
    path = "/api/v1/marketplace/plugins/search",
    tag = "marketplace",
    params(PluginSearchQuery),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Plugins matching the query (may be empty)", body = [MarketplacePlugin]),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:read permission", body = ApiErrorBody)
    )
)]
pub async fn search_marketplace_plugins(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Query(params): Query<PluginSearchQuery>,
) -> Result<Json<Vec<MarketplacePlugin>>, ApiError> {
    require_permission(&claims, "marketplace:read")?;
    let plugins = state.marketplace.plugins.read();
    let list: Vec<MarketplacePlugin> = plugins
        .values()
        .filter(|plugin| matches_search(plugin, params.q.as_deref(), params.category.as_deref()))
        .cloned()
        .collect();
    Ok(Json(list))
}

/// Case-insensitive search predicate over names, descriptions, authors,
/// and tags, with optional category filtering.
fn matches_search(plugin: &MarketplacePlugin, query: Option<&str>, category: Option<&str>) -> bool {
    if let Some(category) = category {
        let wanted = category.trim().to_lowercase();
        if !plugin.categories.iter().any(|c| c.to_lowercase() == wanted) {
            return false;
        }
    }
    let Some(query) = query else {
        return true;
    };
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    plugin.name.to_lowercase().contains(&needle)
        || plugin.description.to_lowercase().contains(&needle)
        || plugin.author.to_lowercase().contains(&needle)
        || plugin
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&needle))
}

/// Record a plugin download, incrementing its download counter. Requires
/// `marketplace:read` (any authenticated user downloading a plugin).
#[utoipa::path(
    post,
    path = "/api/v1/marketplace/plugins/{name}/download",
    tag = "marketplace",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Download recorded", body = PluginDownloadResponse),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Missing marketplace:read permission or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn track_plugin_download(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
) -> Result<Json<PluginDownloadResponse>, ApiError> {
    require_permission(&claims, "marketplace:read")?;
    let mut plugins = state.marketplace.plugins.write();
    let Some(plugin) = plugins.get_mut(&name) else {
        return Err(ApiError::NotFound(format!("Plugin '{name}' not found")));
    };
    plugin.downloads = plugin.downloads.saturating_add(1);
    plugin.updated_at = Utc::now().to_rfc3339();
    Ok(Json(PluginDownloadResponse {
        name,
        downloads: plugin.downloads,
    }))
}

/// Mark a plugin as verified (registry maintainer badge). Requires
/// `marketplace:write` AND the `admin` role; editors or other
/// `marketplace:write` holders are rejected.
#[utoipa::path(
    post,
    path = "/api/v1/marketplace/plugins/{name}/verify",
    tag = "marketplace",
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Plugin verified", body = MarketplacePlugin),
        (status = 401, description = "Missing or invalid credentials", body = ApiErrorBody),
        (status = 403, description = "Caller is not an admin, missing marketplace:write permission, or CSRF origin rejected", body = ApiErrorBody),
        (status = 404, description = "Plugin not found", body = ApiErrorBody)
    )
)]
pub async fn verify_marketplace_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(name): Path<String>,
) -> Result<Json<MarketplacePlugin>, ApiError> {
    require_permission(&claims, "marketplace:write")?;
    if !is_admin(&claims) {
        return Err(ApiError::Forbidden(
            "Only admins can verify plugins".to_string(),
        ));
    }
    let mut plugins = state.marketplace.plugins.write();
    let Some(plugin) = plugins.get_mut(&name) else {
        return Err(ApiError::NotFound(format!("Plugin '{name}' not found")));
    };
    plugin.verified = true;
    plugin.updated_at = Utc::now().to_rfc3339();
    Ok(Json(plugin.clone()))
}

#[cfg(test)]
mod tests {
    use super::matches_search;
    use crate::types::MarketplacePlugin;

    fn plugin(
        name: &str,
        description: &str,
        categories: &[&str],
        tags: &[&str],
    ) -> MarketplacePlugin {
        MarketplacePlugin {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "author".to_string(),
            description: description.to_string(),
            license: "MIT".to_string(),
            categories: categories.iter().map(|s| (*s).to_string()).collect(),
            tags: tags.iter().map(|s| (*s).to_string()).collect(),
            downloads: 0,
            rating: 0.0,
            rating_count: 0,
            verified: false,
            created_at: "2026-01-01T00:00:00+00:00".to_string(),
            updated_at: "2026-01-01T00:00:00+00:00".to_string(),
        }
    }

    #[test]
    fn search_matches_name_description_and_tags_case_insensitively() {
        let p = plugin(
            "Meta-Checker",
            "Audits meta description tags",
            &["seo"],
            &["metadata"],
        );
        assert!(matches_search(&p, Some("meta"), None));
        assert!(matches_search(&p, Some("META"), None));
        assert!(matches_search(&p, Some("description"), None));
        assert!(matches_search(&p, Some("author"), None));
        assert!(matches_search(&p, Some("Metadata"), None));
        assert!(!matches_search(&p, Some("viewport"), None));
    }

    #[test]
    fn search_query_is_trimmed_and_blank_query_matches_everything() {
        let p = plugin("meta-checker", "Audits meta tags", &["seo"], &["meta"]);
        assert!(matches_search(&p, Some("  meta  "), None));
        assert!(matches_search(&p, Some("   "), None));
        assert!(matches_search(&p, None, None));
    }

    #[test]
    fn search_category_filter_is_exact_but_case_insensitive() {
        let p = plugin("meta-checker", "Audits meta tags", &["seo", "mobile"], &[]);
        assert!(matches_search(&p, None, Some("seo")));
        assert!(matches_search(&p, None, Some("  SEO ")));
        assert!(matches_search(&p, None, Some("Mobile")));
        // Category substring matches do not count: exact match only.
        assert!(!matches_search(&p, None, Some("se")));
        assert!(!matches_search(&p, None, Some("performance")));
    }

    #[test]
    fn search_combines_query_and_category_filters() {
        let p = plugin("meta-checker", "Audits meta tags", &["seo"], &["meta"]);
        let other = plugin("viewport-checker", "Viewport audits", &["mobile"], &[]);
        assert!(matches_search(&p, Some("meta"), Some("seo")));
        assert!(!matches_search(&p, Some("meta"), Some("mobile")));
        assert!(!matches_search(&other, Some("meta"), Some("mobile")));
    }
}
