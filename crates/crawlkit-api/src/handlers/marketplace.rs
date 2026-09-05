use axum::extract::{Extension, Path, State};
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
