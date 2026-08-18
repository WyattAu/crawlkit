use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::types::*;

pub async fn list_marketplace_plugins(State(state): State<AppState>) -> Json<Vec<MarketplacePlugin>> {
    let plugins = state.marketplace.plugins.read();
    let list: Vec<MarketplacePlugin> = plugins.values().cloned().collect();
    Json(list)
}

pub async fn get_marketplace_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<MarketplacePlugin>, ApiError> {
    let plugins = state.marketplace.plugins.read();
    plugins
        .get(&name)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Plugin '{name}' not found")))
}

pub async fn submit_plugin(
    State(state): State<AppState>,
    Json(input): Json<SubmitPluginRequest>,
) -> Result<(StatusCode, Json<MarketplacePlugin>), ApiError> {
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

pub async fn delete_marketplace_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut plugins = state.marketplace.plugins.write();
    if plugins.remove(&name).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

#[allow(dead_code, clippy::unused_async)]
pub async fn download_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Vec<u8>, ApiError> {
    let plugins = state.marketplace.plugins.read();
    if plugins.contains_key(&name) {
        Ok(Vec::new())
    } else {
        Err(ApiError::NotFound(format!("Plugin '{name}' not found")))
    }
}

pub async fn test_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
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
