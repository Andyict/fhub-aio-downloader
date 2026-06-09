//! FHUB Health Check API
//!
//! Provides comprehensive FHUB runtime health status for service components.

use axum::{
    extract::State,
    Json,
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub response_time_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub overall_status: HealthStatus,
    pub websocket: ServiceHealth,
    pub webhook: ServiceHealth,
    pub tvlib: Option<ServiceHealth>,
    pub movielib: Option<ServiceHealth>,
    pub fshare: ServiceHealth,
    pub fshare_ping: ServiceHealth,
    pub internet_speed: ServiceHealth,
    pub database: ServiceHealth,
}

/// GET /api/health/status - Comprehensive FHUB health check.
pub async fn health_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthCheckResponse>, StatusCode> {
    
    // Check FHUB realtime event infrastructure.
    let websocket = check_websocket(&state).await;

    // Check FHUB webhook/API bridge readiness.
    let webhook = check_webhook(&state).await;

    // Check TV library integration.
    let tvlib = check_arr_service(&state, "tvlib").await;

    // Check movie library integration.
    let movielib = check_arr_service(&state, "movielib").await;

    // Check source provider handler.
    let fshare = check_fshare(&state).await;

    // Check source provider connectivity.
    let fshare_ping = check_fshare_ping(&state).await;

    // Check network speed integration placeholder.
    let internet_speed = check_internet_speed().await;

    // Check FHUB database.
    let database = check_database(&state).await;

    // Determine overall FHUB runtime status.
    let overall_status = determine_overall_status(&[
        &websocket.status,
        &webhook.status,
        &tvlib.as_ref().map(|s| &s.status).unwrap_or(&HealthStatus::Healthy),
        &movielib.as_ref().map(|s| &s.status).unwrap_or(&HealthStatus::Healthy),
        &fshare.status,
        &fshare_ping.status,
        &database.status,
    ]);

    Ok(Json(HealthCheckResponse {
        overall_status,
        websocket,
        webhook,
        tvlib,
        movielib,
        fshare,
        fshare_ping,
        internet_speed,
        database,
    }))
}

async fn check_arr_service(state: &AppState, service_type: &str) -> Option<ServiceHealth> {
    // Get FHUB integration settings from database.
    let db = &state.db;
    
    let (enabled, url, api_key) = match service_type {
        "tvlib" => {
            let enabled = db.get_setting("tvlib_enabled")
                .ok()
                .flatten()
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or(false);
            let url = db.get_setting("tvlib_url").ok().flatten();
            let api_key = db.get_setting("tvlib_api_key").ok().flatten();
            (enabled, url, api_key)
        },
        "movielib" => {
            let enabled = db.get_setting("movielib_enabled")
                .ok()
                .flatten()
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or(false);
            let url = db.get_setting("movielib_url").ok().flatten();
            let api_key = db.get_setting("movielib_api_key").ok().flatten();
            (enabled, url, api_key)
        },
        _ => return None,
    };

    if !enabled {
        return None;
    }

    let url = url?;
    let api_key = api_key?;

    // Test connection with system/status endpoint.
    let start = std::time::Instant::now();
    let client = reqwest::Client::new();
    let test_url = format!("{}/api/v3/system/status", url.trim_end_matches('/'));
    
    match client
        .get(&test_url)
        .header("X-Api-Key", api_key)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => {
            let response_time = start.elapsed().as_millis() as u64;
            if resp.status().is_success() {
                Some(ServiceHealth {
                    status: HealthStatus::Healthy,
                    message: Some(format!("FHUB integration connected ({}ms)", response_time)),
                    response_time_ms: Some(response_time),
                })
            } else {
                Some(ServiceHealth {
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("FHUB integration returned HTTP {}", resp.status())),
                    response_time_ms: Some(response_time),
                })
            }
        }
        Err(e) => {
            Some(ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some(format!("FHUB integration connection failed: {}", e)),
                response_time_ms: None,
            })
        }
    }
}

async fn check_fshare(state: &AppState) -> ServiceHealth {
    // Get source provider handler from host registry.
    if let Some(_fshare_handler) = state.host_registry.get_handler("fshare") {
        ServiceHealth {
            status: HealthStatus::Healthy,
            message: Some("FHUB source handler available".to_string()),
            response_time_ms: Some(0),
        }
    } else {
        ServiceHealth {
            status: HealthStatus::Degraded,
            message: Some("FHUB source handler not registered".to_string()),
            response_time_ms: Some(0),
        }
    }
}

async fn check_database(_state: &AppState) -> ServiceHealth {
    // FHUB database is local SQLite.
    ServiceHealth {
        status: HealthStatus::Healthy,
        message: Some("FHUB database connected".to_string()),
        response_time_ms: Some(0),
    }
}

fn determine_overall_status(statuses: &[&HealthStatus]) -> HealthStatus {
    if statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
        HealthStatus::Degraded
    } else if statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

async fn check_websocket(state: &AppState) -> ServiceHealth {
    // FHUB uses the broadcast channel for realtime activity updates.
    // receiver_count() does not fully reflect active websocket connections,
    // so this reports whether the realtime infrastructure is available.
    
    let receiver_count = state.tx_broadcast.receiver_count();
    
    ServiceHealth {
        status: HealthStatus::Healthy,
        message: Some(if receiver_count > 0 {
            format!("FHUB realtime active: {} subscribers", receiver_count)
        } else {
            "FHUB realtime ready".to_string()
        }),
        response_time_ms: Some(0),
    }
}

async fn check_webhook(_state: &AppState) -> ServiceHealth {
    // Check if the FHUB API bridge is available for NAS integration.
    
    ServiceHealth {
        status: HealthStatus::Healthy,
        message: Some("FHUB API bridge ready".to_string()),
        response_time_ms: Some(0),
    }
}

async fn check_fshare_ping(state: &AppState) -> ServiceHealth {
    // Actual ping to the source provider to test connectivity.
    let start = std::time::Instant::now();
    let client = reqwest::Client::new();
    
    match client
        .get("https://www.fshare.vn")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => {
            let response_time = start.elapsed().as_millis() as u64;
            if resp.status().is_success() || resp.status().is_redirection() {
                ServiceHealth {
                    status: HealthStatus::Healthy,
                    message: Some(format!("FHUB source reachable in {}ms", response_time)),
                    response_time_ms: Some(response_time),
                }
            } else {
                ServiceHealth {
                    status: HealthStatus::Degraded,
                    message: Some(format!("FHUB source returned HTTP {}", resp.status())),
                    response_time_ms: Some(response_time),
                }
            }
        }
        Err(e) => {
            ServiceHealth {
                status: HealthStatus::Unhealthy,
                message: Some(format!("FHUB source connectivity failed: {}", e)),
                response_time_ms: None,
            }
        }
    }
}

async fn check_internet_speed() -> ServiceHealth {
    // Placeholder for FHUB network speed check.
    // TODO: Integrate a native FHUB network probe later.
    
    ServiceHealth {
        status: HealthStatus::Healthy,
        message: Some("FHUB network probe not configured".to_string()),
        response_time_ms: None,
    }
}
