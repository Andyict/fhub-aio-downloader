mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::get_setup_status))
        .route("/fshare", post(handlers::setup_fshare))
        .route("/jellyfin/test", post(handlers::test_jellyfin))
        .route("/indexer/key", get(handlers::get_indexer_key))
        .route("/complete", post(handlers::complete_setup))
}
