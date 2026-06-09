//! Settings API Routes
//!
//! Application configuration management endpoints.

use axum::{extract::State, routing::{get, put}, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_settings))
        .route("/", put(update_settings))
        .route("/downloads", get(get_downloads_settings))
        .route("/downloads", put(update_downloads_settings))
        .route("/indexer", get(get_indexer_settings))
        .route("/indexer", put(update_indexer_settings))
        .route("/indexer/generate-key", get(generate_api_key))
}

#[derive(Serialize)]
struct SettingsResponse {
    server: ServerSettings,
    downloads: DownloadsSettings,
}

#[derive(Serialize)]
struct ServerSettings {
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize)]
struct DownloadsSettings {
    directory: String,
    max_concurrent: usize,
    segments_per_download: u32,
}

#[derive(Serialize, Deserialize)]
struct IndexerSettings {
    enabled: bool,
    api_key: String,
    indexer_url: String,
}

#[derive(Serialize)]
struct GenerateKeyResponse {
    api_key: String,
}

#[derive(Serialize)]
struct ActionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Deserialize)]
struct UpdateSettingsRequest {
    downloads: Option<DownloadsSettings>,
}

async fn get_settings(State(state): State<Arc<AppState>>) -> Json<SettingsResponse> {
    let config = &state.config;
    Json(SettingsResponse {
        server: ServerSettings {
            host: config.server.host.clone(),
            port: config.server.port,
        },
        downloads: DownloadsSettings {
            directory: config.downloads.directory.to_string_lossy().to_string(),
            max_concurrent: config.downloads.max_concurrent,
            segments_per_download: config.downloads.segments_per_download,
        },
    })
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Json<ActionResponse> {
    if let Some(downloads) = payload.downloads {
        let mut config = state.download_orchestrator.get_config().await;
        config.max_concurrent = downloads.max_concurrent;
        config.segments_per_download = downloads.segments_per_download as usize;
        config.download_dir = std::path::PathBuf::from(downloads.directory);
        state.download_orchestrator.update_config(config).await;
    }

    Json(ActionResponse {
        success: true,
        message: Some("Settings updated successfully".to_string()),
    })
}

async fn get_downloads_settings(State(state): State<Arc<AppState>>) -> Json<DownloadsSettings> {
    let config = state.download_orchestrator.get_config().await;
    Json(DownloadsSettings {
        directory: config.download_dir.to_string_lossy().to_string(),
        max_concurrent: config.max_concurrent,
        segments_per_download: config.segments_per_download as u32,
    })
}

async fn update_downloads_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DownloadsSettings>,
) -> Json<ActionResponse> {
    let mut config = state.download_orchestrator.get_config().await;
    config.max_concurrent = payload.max_concurrent;
    config.segments_per_download = payload.segments_per_download as usize;
    config.download_dir = std::path::PathBuf::from(payload.directory);
    state.download_orchestrator.update_config(config).await;
    Json(ActionResponse {
        success: true,
        message: Some("Download settings updated successfully".to_string()),
    })
}

async fn get_indexer_settings(State(state): State<Arc<AppState>>) -> Json<IndexerSettings> {
    let host = &state.config.server.host;
    let port = state.config.server.port;
    let indexer_url = format!("http://{}:{}/newznab", host, port);
    let api_key = state.db.get_setting("indexer_api_key")
        .ok()
        .flatten()
        .unwrap_or_else(|| "fhub-default-key".to_string());
    Json(IndexerSettings { enabled: true, api_key, indexer_url })
}

async fn update_indexer_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IndexerSettings>,
) -> Json<ActionResponse> {
    if let Err(e) = state.db.save_setting("indexer_api_key", &payload.api_key) {
        return Json(ActionResponse {
            success: false,
            message: Some(format!("Failed to save API settings: {}", e)),
        });
    }
    tracing::info!("FHub API settings updated");
    Json(ActionResponse {
        success: true,
        message: Some("API settings updated successfully".to_string()),
    })
}

async fn generate_api_key() -> Json<GenerateKeyResponse> {
    let api_key = format!("fhub_{}", uuid::Uuid::new_v4().simple());
    Json(GenerateKeyResponse { api_key })
}
