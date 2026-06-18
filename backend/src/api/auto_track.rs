use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, patch, post}, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{db::{AutoTrack, AutoTrackItem}, downloader::DownloadState, AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_tracks).post(create_track))
        .route("/:id", get(get_track).delete(delete_track).patch(update_track))
        .route("/:id/check", post(check_track))
}

#[derive(Debug, Deserialize)]
pub struct CreateAutoTrackRequest {
    pub title: String,
    pub folder_url: String,
    #[serde(default)] pub folder_code: Option<String>,
    #[serde(default)] pub media_type: Option<String>,
    #[serde(default)] pub category: Option<String>,
    #[serde(default)] pub download_folder: Option<String>,
    #[serde(default)] pub enabled: Option<bool>,
    #[serde(default)] pub check_interval_secs: Option<i64>,
    #[serde(default)] pub tmdb_id: Option<i64>,
    #[serde(default)] pub year: Option<i32>,
    #[serde(default)] pub season: Option<i32>,
    #[serde(default)] pub batch_id: Option<String>,
    #[serde(default)] pub batch_name: Option<String>,
    #[serde(default)] pub poster_url: Option<String>,
    #[serde(default)] pub check_now: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutoTrackRequest {
    #[serde(default)] pub enabled: Option<bool>,
    #[serde(default)] pub check_interval_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AutoTrackWithItems {
    #[serde(flatten)]
    pub track: AutoTrack,
    pub items: Vec<AutoTrackItem>,
}

async fn list_tracks(State(state): State<Arc<AppState>>) -> Result<Json<Vec<AutoTrack>>, (StatusCode, String)> {
    state.db.list_auto_tracks_async().await.map(Json).map_err(internal)
}

async fn get_track(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<AutoTrackWithItems>, (StatusCode, String)> {
    let track = state.db.get_auto_track_async(id.clone()).await.map_err(internal)?.ok_or((StatusCode::NOT_FOUND, "Không tìm thấy auto track".to_string()))?;
    let items = state.db.list_auto_track_items_async(id).await.map_err(internal)?;
    let items = sync_items_with_downloads(&state, &track, items).await;
    Ok(Json(AutoTrackWithItems { track, items }))
}

async fn create_track(State(state): State<Arc<AppState>>, Json(payload): Json<CreateAutoTrackRequest>) -> Result<Json<AutoTrack>, (StatusCode, String)> {
    if payload.title.trim().is_empty() || payload.folder_url.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Thiếu title hoặc folder_url".to_string()));
    }
    let now = Utc::now().to_rfc3339();
    let folder_code = payload.folder_code.unwrap_or_default();
    let id = Uuid::new_v4().to_string();
    let default_interval = state.db.get_setting("auto_track_check_interval_secs")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(3600)
        .clamp(300, 86400);
    let track = AutoTrack {
        id,
        media_type: payload.media_type.unwrap_or_else(|| "tv".to_string()),
        title: payload.title.trim().to_string(),
        folder_url: payload.folder_url.trim().to_string(),
        folder_code,
        category: payload.category.unwrap_or_else(|| "tv".to_string()),
        download_folder: payload.download_folder.filter(|value| !value.trim().is_empty()),
        enabled: payload.enabled.unwrap_or(true),
        check_interval_secs: payload.check_interval_secs.unwrap_or(default_interval).clamp(300, 86400),
        tmdb_id: payload.tmdb_id,
        year: payload.year,
        season: payload.season,
        batch_id: payload.batch_id,
        batch_name: payload.batch_name,
        poster_url: payload.poster_url,
        last_checked_at: None,
        last_error: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let track = state.auto_track_service.create_track(track).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let svc = Arc::clone(&state.auto_track_service);
    let track_id = track.id.clone();
    if payload.check_now.unwrap_or(false) {
        tokio::spawn(async move { let _ = svc.check_track(&track_id).await; });
    } else {
        // First enable should only establish a baseline of existing files.
        // Otherwise Auto Track treats the whole current folder as "new" and
        // downloads the complete season without the user pressing Download.
        tokio::spawn(async move { let _ = svc.baseline_track(&track_id).await; });
    }
    Ok(Json(track))
}

async fn update_track(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(payload): Json<UpdateAutoTrackRequest>) -> Result<Json<AutoTrack>, (StatusCode, String)> {
    if let Some(enabled) = payload.enabled {
        let updated = state.db.set_auto_track_enabled_async(id.clone(), enabled).await.map_err(internal)?;
        if !updated { return Err((StatusCode::NOT_FOUND, "Không tìm thấy auto track".to_string())); }
    }
    if payload.check_interval_secs.is_some() {
        let mut track = state.db.get_auto_track_async(id.clone()).await.map_err(internal)?.ok_or((StatusCode::NOT_FOUND, "Không tìm thấy auto track".to_string()))?;
        track.check_interval_secs = payload.check_interval_secs.unwrap_or(track.check_interval_secs).max(300);
        track.updated_at = Utc::now().to_rfc3339();
        state.db.upsert_auto_track_async(track).await.map_err(internal)?;
    }
    let track = state.db.get_auto_track_async(id).await.map_err(internal)?.ok_or((StatusCode::NOT_FOUND, "Không tìm thấy auto track".to_string()))?;
    Ok(Json(track))
}

async fn delete_track(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.db.delete_auto_track_async(id).await.map_err(internal)?;
    Ok(Json(serde_json::json!({"success": true})))
}

async fn check_track(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Json<crate::services::auto_track_service::AutoTrackCheckReport>, (StatusCode, String)> {
    state.auto_track_service.check_track(&id).await.map(Json).map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

async fn sync_items_with_downloads(state: &Arc<AppState>, track: &AutoTrack, items: Vec<AutoTrackItem>) -> Vec<AutoTrackItem> {
    let mut synced = Vec::with_capacity(items.len());

    for mut item in items {
        let mut task = None;

        if let Some(download_id) = item.download_id.as_deref().and_then(|id| Uuid::parse_str(id).ok()) {
            if let Ok(found) = state.db.get_task_by_id(download_id) {
                task = found;
            }
        }

        // Baseline items created by Auto Track intentionally have no download_id.
        // If the user later downloads the same FShare file manually/from Discovery,
        // link the item back to the real download task by fshare_code so the UI shows
        // the actual state: waiting/downloading/paused/completed.
        if task.is_none() && !item.fshare_code.trim().is_empty() {
            if let Ok(found) = state.db.get_task_by_fshare_code_async(item.fshare_code.clone()).await {
                task = found;
            }
        }

        // Some FShare folders change/repost a file: the Auto Track item can keep one
        // fshare_code while the actual completed download was created from another
        // code for the same title + SxxExx. Fall back to episode identity so already
        // downloaded episodes do not remain stuck as queued/seen.
        if task.is_none() {
            if let (Some(season), Some(episode)) = (item.season, item.episode) {
                if let Ok(found) = state.db.get_task_by_title_episode_async(track.title.clone(), season, episode).await {
                    task = found;
                }
            }
        }

        if let Some(task) = task {
            let completed_file_exists = matches!(task.state, DownloadState::Completed)
                && !task.destination.trim().is_empty()
                && std::path::Path::new(&task.destination).is_file();

            let synced_status = if completed_file_exists {
                "completed".to_string()
            } else {
                match task.state {
                    DownloadState::Completed => "failed".to_string(),
                    DownloadState::Downloading | DownloadState::Starting | DownloadState::Extracting => "downloading".to_string(),
                    DownloadState::Paused => "paused".to_string(),
                    DownloadState::Queued | DownloadState::Waiting => "queued".to_string(),
                    DownloadState::Skipped => "skipped".to_string(),
                    DownloadState::Failed | DownloadState::Cancelled => "failed".to_string(),
                }
            };

            let task_id = task.id.to_string();
            let mut changed = item.status != synced_status;
            item.status = synced_status;

            if item.download_id.as_deref() != Some(task_id.as_str()) {
                item.download_id = Some(task_id);
                changed = true;
            }

            if item.queued_at.is_none() {
                item.queued_at = Some(task.created_at.to_rfc3339());
                changed = true;
            }

            if completed_file_exists {
                let completed_at = task.completed_at.map(|t| t.to_rfc3339()).or_else(|| Some(Utc::now().to_rfc3339()));
                if item.completed_at != completed_at {
                    item.completed_at = completed_at;
                    changed = true;
                }
            } else if matches!(task.state, DownloadState::Completed) {
                if item.completed_at.take().is_some() { changed = true; }
                let msg = "Task báo hoàn thành nhưng file không còn tồn tại trên ổ".to_string();
                if item.error_message.as_deref() != Some(msg.as_str()) {
                    item.error_message = Some(msg);
                    changed = true;
                }
            }

            if item.error_message.is_none() && task.error_message.is_some() {
                item.error_message = task.error_message;
                changed = true;
            }

            if changed {
                let _ = state.db.update_auto_track_item_sync_state_async(
                    item.id.clone(),
                    item.status.clone(),
                    item.download_id.clone(),
                    item.queued_at.clone(),
                    item.completed_at.clone(),
                    item.error_message.clone(),
                ).await;
            }
        }
        synced.push(item);
    }

    synced
}

fn internal(e: rusqlite::Error) -> (StatusCode, String) { (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()) }
