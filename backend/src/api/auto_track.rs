use axum::{extract::{Path, State}, http::StatusCode, routing::{delete, get, patch, post}, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{db::{AutoTrack, AutoTrackItem}, AppState};

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
    #[serde(default)] pub enabled: Option<bool>,
    #[serde(default)] pub check_interval_secs: Option<i64>,
    #[serde(default)] pub tmdb_id: Option<i64>,
    #[serde(default)] pub year: Option<i32>,
    #[serde(default)] pub season: Option<i32>,
    #[serde(default)] pub batch_id: Option<String>,
    #[serde(default)] pub batch_name: Option<String>,
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
        enabled: payload.enabled.unwrap_or(true),
        check_interval_secs: payload.check_interval_secs.unwrap_or(default_interval).clamp(300, 86400),
        tmdb_id: payload.tmdb_id,
        year: payload.year,
        season: payload.season,
        batch_id: payload.batch_id,
        batch_name: payload.batch_name,
        last_checked_at: None,
        last_error: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let track = state.auto_track_service.create_track(track).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if payload.check_now.unwrap_or(false) {
        let svc = Arc::clone(&state.auto_track_service);
        let track_id = track.id.clone();
        tokio::spawn(async move { let _ = svc.check_track(&track_id).await; });
    }
    Ok(Json(track))
}

async fn update_track(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(payload): Json<UpdateAutoTrackRequest>) -> Result<Json<AutoTrack>, (StatusCode, String)> {
    if let Some(enabled) = payload.enabled { state.db.set_auto_track_enabled_async(id.clone(), enabled).await.map_err(internal)?; }
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

fn internal(e: rusqlite::Error) -> (StatusCode, String) { (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()) }
