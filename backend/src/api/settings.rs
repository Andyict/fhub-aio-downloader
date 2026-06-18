//! Settings API Routes
//!
//! Application configuration management endpoints.

use axum::{extract::State, routing::{get, put}, Json, Router};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
        .route("/auto-track", get(get_auto_track_settings))
        .route("/auto-track", put(update_auto_track_settings))
        .route("/download-categories", get(get_download_categories))
        .route("/download-categories", put(update_download_categories))
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    host_directory: Option<String>,
    max_concurrent: usize,
    segments_per_download: u32,
}

#[derive(Serialize, Deserialize)]
struct IndexerSettings {
    enabled: bool,
    api_key: String,
    indexer_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct DownloadCategory {
    id: String,
    label: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct DownloadCategoriesSettings {
    categories: Vec<DownloadCategory>,
}

#[derive(Serialize, Deserialize)]
struct AutoTrackSettings {
    check_interval_secs: i64,
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

fn host_downloads_dir() -> Option<String> {
    std::env::var("FHUB_HOST_DOWNLOADS_DIR").ok().filter(|value| !value.trim().is_empty())
}

async fn get_settings(State(state): State<Arc<AppState>>) -> Json<SettingsResponse> {
    let config = &state.config;
    let host_directory = host_downloads_dir();
    Json(SettingsResponse {
        server: ServerSettings {
            host: config.server.host.clone(),
            port: config.server.port,
        },
        downloads: DownloadsSettings {
            directory: config.downloads.directory.to_string_lossy().to_string(),
            host_directory,
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
        host_directory: host_downloads_dir(),
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

fn sanitize_category_id(label: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !out.ends_with('-') { out.push('-'); }
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { fallback.to_string() } else { out }
}

fn normalize_category_path(root: &Path, value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() { return root.join("Movies").to_string_lossy().to_string(); }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path.to_string_lossy().to_string()
    } else {
        root.join(path).to_string_lossy().to_string()
    }
}

fn default_download_categories(root: &Path) -> Vec<DownloadCategory> {
    vec![
        DownloadCategory { id: "movies".to_string(), label: "Phim lẻ".to_string(), path: root.join("Movies").to_string_lossy().to_string() },
        DownloadCategory { id: "shows".to_string(), label: "Phim bộ".to_string(), path: root.join("Shows").to_string_lossy().to_string() },
        DownloadCategory { id: "animation".to_string(), label: "Phim hoạt hình".to_string(), path: root.join("Animation").to_string_lossy().to_string() },
        DownloadCategory { id: "others".to_string(), label: "Khác".to_string(), path: root.join("Others").to_string_lossy().to_string() },
    ]
}

fn load_download_categories(state: &AppState) -> Vec<DownloadCategory> {
    let root = state.config.downloads.directory.clone();
    state.db.get_setting("download_categories")
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<DownloadCategoriesSettings>(&raw).ok())
        .map(|payload| payload.categories.into_iter().filter(|c| !c.label.trim().is_empty() && !c.path.trim().is_empty()).collect::<Vec<_>>())
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| default_download_categories(&root))
}

async fn get_download_categories(State(state): State<Arc<AppState>>) -> Json<DownloadCategoriesSettings> {
    Json(DownloadCategoriesSettings { categories: load_download_categories(&state) })
}

async fn update_download_categories(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DownloadCategoriesSettings>,
) -> Json<ActionResponse> {
    let root = state.config.downloads.directory.clone();
    let mut categories = Vec::new();
    for (index, item) in payload.categories.into_iter().enumerate() {
        let label = item.label.trim().to_string();
        if label.is_empty() { continue; }
        let id_source = if item.id.trim().is_empty() { &label } else { item.id.trim() };
        categories.push(DownloadCategory {
            id: sanitize_category_id(id_source, &format!("category-{}", index + 1)),
            label,
            path: normalize_category_path(&root, &item.path),
        });
    }
    if categories.is_empty() { categories = default_download_categories(&root); }
    let raw = match serde_json::to_string(&DownloadCategoriesSettings { categories: categories.clone() }) {
        Ok(raw) => raw,
        Err(e) => return Json(ActionResponse { success: false, message: Some(format!("Không lưu được thư mục phân loại: {e}")) }),
    };
    if let Err(e) = state.db.save_setting("download_categories", &raw) {
        return Json(ActionResponse { success: false, message: Some(format!("Không lưu được thư mục phân loại: {e}")) });
    }
    Json(ActionResponse { success: true, message: Some("Đã lưu thư mục phân loại tải xuống".to_string()) })
}

async fn get_auto_track_settings(State(state): State<Arc<AppState>>) -> Json<AutoTrackSettings> {
    let check_interval_secs = state.db.get_setting("auto_track_check_interval_secs")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(3600)
        .clamp(300, 86400);
    Json(AutoTrackSettings { check_interval_secs })
}

async fn update_auto_track_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AutoTrackSettings>,
) -> Json<ActionResponse> {
    let value = payload.check_interval_secs.clamp(300, 86400);
    if let Err(e) = state.db.save_setting("auto_track_check_interval_secs", &value.to_string()) {
        return Json(ActionResponse { success: false, message: Some(format!("Failed to save Auto Track settings: {}", e)) });
    }
    if let Err(e) = state.db.update_all_auto_track_intervals_async(value).await {
        return Json(ActionResponse { success: false, message: Some(format!("Failed to update existing Auto Tracks: {}", e)) });
    }
    Json(ActionResponse { success: true, message: Some("Đã lưu Auto Track và cập nhật các track hiện có".to_string()) })
}

async fn generate_api_key() -> Json<GenerateKeyResponse> {
    let api_key = format!("fhub_{}", uuid::Uuid::new_v4().simple());
    Json(GenerateKeyResponse { api_key })
}
