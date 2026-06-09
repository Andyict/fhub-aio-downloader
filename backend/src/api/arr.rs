//! ARR compatibility API routes
//!
//! FHub can run standalone without Radarr/Sonarr. The frontend still exposes a
//! few external-library actions, so these endpoints return safe, explicit
//! standalone-mode responses instead of 404s.

use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/series", get(list_series))
        .route("/episodes", get(list_episodes))
        .route("/movies", get(list_movies))
        .route("/movies/add", post(add_movie))
        .route("/series/add", post(add_series))
}

#[derive(Debug, Deserialize)]
struct EpisodesQuery {
    #[allow(dead_code)]
    series_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmptyListResponse<T> {
    items: Vec<T>,
    total: usize,
    standalone: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    success: bool,
    standalone: bool,
    message: String,
}

fn standalone_message() -> String {
    "ARR integration is not configured in this standalone FHub build.".to_string()
}

async fn list_series() -> Json<EmptyListResponse<Value>> {
    Json(EmptyListResponse {
        items: vec![],
        total: 0,
        standalone: true,
        message: standalone_message(),
    })
}

async fn list_episodes(Query(_query): Query<EpisodesQuery>) -> Json<EmptyListResponse<Value>> {
    Json(EmptyListResponse {
        items: vec![],
        total: 0,
        standalone: true,
        message: standalone_message(),
    })
}

async fn list_movies() -> Json<EmptyListResponse<Value>> {
    Json(EmptyListResponse {
        items: vec![],
        total: 0,
        standalone: true,
        message: standalone_message(),
    })
}

async fn add_movie(Json(_payload): Json<Value>) -> Json<ActionResponse> {
    Json(ActionResponse {
        success: false,
        standalone: true,
        message: "Radarr add is unavailable because ARR integration is not configured.".to_string(),
    })
}

async fn add_series(Json(_payload): Json<Value>) -> Json<ActionResponse> {
    Json(ActionResponse {
        success: false,
        standalone: true,
        message: "Sonarr add is unavailable because ARR integration is not configured.".to_string(),
    })
}
