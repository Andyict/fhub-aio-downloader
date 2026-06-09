//! FHUB System API Routes
//!
//! FHUB runtime information, logs, and update-status endpoints.

use axum::{
    routing::get,
    Router,
    Json,
    extract::Query,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::fs;
use std::path::Path;
use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/version", get(get_version))
        .route("/update/check", get(check_update))
        .route("/logs", get(get_logs))
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
struct VersionResponse {
    version: String,
    rust_version: &'static str,
    build_date: Option<String>,
}

#[derive(Serialize)]
struct UpdateCheckResponse {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    release_url: Option<String>,
    changelog: Option<String>,
    checked_source: String,
}

#[derive(Serialize)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

#[derive(Serialize)]
struct LogsResponse {
    logs: Vec<LogEntry>,
    total: usize,
}

// ============================================================================
// Request Types
// ============================================================================

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default = "default_lines")]
    lines: usize,
    #[serde(default)]
    level: Option<String>,
}

fn default_lines() -> usize {
    100
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/system/version - Get FHUB runtime version info.
async fn get_version() -> Json<VersionResponse> {
    // Try to read the FHUB VERSION file first, then fall back to the crate version.
    let version = fs::read_to_string("VERSION")
        .or_else(|_| fs::read_to_string("../VERSION"))
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
        .trim()
        .to_string();
    
    Json(VersionResponse {
        version,
        rust_version: "1.75+",
        build_date: option_env!("BUILD_DATE").map(|s| s.to_string()),
    })
}

/// GET /api/system/logs - Get recent FHUB runtime log entries.
async fn get_logs(
    Query(params): Query<LogsQuery>,
) -> Json<LogsResponse> {
    let lines = params.lines.min(1000); // Cap at 1000 lines.
    
    // Try to read the FHUB runtime log file from known compatibility locations.
    let log_paths = [
        "data/fhub.log",
        "../data/fhub.log",
        "fhub.log",
    ];
    
    let mut log_content = String::new();
    for path in log_paths {
        if Path::new(path).exists() {
            if let Ok(content) = fs::read_to_string(path) {
                log_content = content;
                break;
            }
        }
    }
    
    // Parse FHUB log entries with simple line-based parsing.
    let log_lines: Vec<&str> = log_content.lines().rev().take(lines).collect();
    let mut logs: Vec<LogEntry> = Vec::new();
    
    for line in log_lines.into_iter().rev() {
        // Expected format: "TIMESTAMP - LEVEL - MESSAGE".
        let parts: Vec<&str> = line.splitn(3, " - ").collect();
        if parts.len() >= 3 {
            let level = parts[1].to_uppercase();
            
            // Filter by level if specified.
            if let Some(ref filter_level) = params.level {
                if !level.contains(&filter_level.to_uppercase()) {
                    continue;
                }
            }
            
            logs.push(LogEntry {
                timestamp: parts[0].to_string(),
                level,
                message: parts[2].to_string(),
            });
        } else {
            // Fallback: treat the entire line as a FHUB runtime message.
            logs.push(LogEntry {
                timestamp: String::new(),
                level: "INFO".to_string(),
                message: line.to_string(),
            });
        }
    }
    
    let total = logs.len();
    Json(LogsResponse { logs, total })
}


/// GET /api/system/update/check - Read-only FHUB update check via GitHub Releases.
async fn check_update() -> Result<Json<UpdateCheckResponse>, StatusCode> {
    let current_version = fs::read_to_string("VERSION")
        .or_else(|_| fs::read_to_string("../VERSION"))
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
        .trim()
        .to_string();

    let repo = std::env::var("FHUB_UPDATE_REPO")
        .unwrap_or_else(|_| "Andyict/FHUB".to_string());
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let client = reqwest::Client::builder()
        .user_agent("FHUB-Update-Checker")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let res = client.get(&url).send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !res.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    let data: serde_json::Value = res.json().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    let latest = data.get("tag_name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let release_url = data.get("html_url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let changelog = data.get("body").and_then(|v| v.as_str()).map(|s| s.chars().take(600).collect::<String>());

    let norm_current = current_version.trim_start_matches('v').to_string();
    let update_available = latest.as_ref()
        .map(|v| v.trim_start_matches('v') != norm_current)
        .unwrap_or(false);

    Ok(Json(UpdateCheckResponse {
        current_version,
        latest_version: latest,
        update_available,
        release_url,
        changelog,
        checked_source: repo,
    }))
}
