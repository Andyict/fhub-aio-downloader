use axum::{extract::State, http::StatusCode, response::Json};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::hosts::base::HostHandler;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
});

#[derive(Serialize)]
pub struct SetupStatus {
    pub complete: bool,
}

#[derive(Serialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct IndexerKeyResponse {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct FshareCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct JellyfinConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Deserialize)]
pub struct CompleteSetupPayload {
    pub fshare: Option<FshareCredentials>,
    pub downloads: DownloadsConfig,
    pub jellyfin: Option<JellyfinConfig>,
}

#[derive(Deserialize)]
pub struct DownloadsConfig {
    pub directory: String,
    pub max_concurrent: u32,
}

pub async fn get_setup_status(State(state): State<Arc<AppState>>) -> Json<SetupStatus> {
    let complete = state.db.is_onboarding_complete().unwrap_or(false);
    Json(SetupStatus { complete })
}


async fn refresh_fshare_rank_after_setup(state: &Arc<AppState>) -> Result<String, String> {
    let Some(handler) = state.host_registry.get_handler_for_url("https://fshare.vn/file/test") else {
        let _ = state.db.save_setting("fshare_rank", "UNVERIFIED");
        let _ = state.db.save_setting("fshare_valid_until", "0");
        return Err("FShare handler not found".to_string());
    };

    if let Err(e) = handler.logout().await {
        tracing::warn!("[SETUP] Failed to clear FShare session before verify: {}", e);
    }

    match handler.check_account_status().await {
        Ok(status) => {
            let rank = if status.premium { "VIP" } else { "FREE" };
            let valid_until = status.valid_until.unwrap_or(0);
            let _ = state.db.save_setting("fshare_rank", rank);
            let _ = state.db.save_setting("fshare_valid_until", &valid_until.to_string());
            tracing::info!("[SETUP] FShare verified after setup: rank={} valid_until={}", rank, valid_until);
            Ok(rank.to_string())
        }
        Err(e) => {
            let _ = state.db.save_setting("fshare_rank", "UNVERIFIED");
            let _ = state.db.save_setting("fshare_valid_until", "0");
            tracing::warn!("[SETUP] FShare VIP verification failed after setup: {}", e);
            Err(e.to_string())
        }
    }
}

pub async fn setup_fshare(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FshareCredentials>,
) -> Result<Json<TestResult>, StatusCode> {
    let client = &*HTTP_CLIENT;

    let api_result = client
        .post("https://download.fsharegroup.site/api/user/login")
        .json(&serde_json::json!({
            "user_email": payload.email,
            "password": payload.password,
        }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match api_result {
        Ok(response) => {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                if data["code"] == 200 {
                    if let Err(e) = state.db.save_fshare_credentials(&payload.email, &payload.password) {
                        tracing::error!("Failed to save Fshare credentials: {}", e);
                        return Ok(Json(TestResult { success: false, message: "Failed to save credentials".to_string(), version: None }));
                    }
                    return Ok(Json(TestResult { success: true, message: "Connected via API successfully".to_string(), version: None }));
                }
                let msg = data["msg"].as_str().unwrap_or("Invalid credentials");
                return Ok(Json(TestResult { success: false, message: msg.to_string(), version: None }));
            }
            return Ok(Json(TestResult { success: false, message: "Failed to parse API response".to_string(), version: None }));
        }
        Err(e) => {
            let err_str = format!("{}", e);
            if !(err_str.contains("timed out") || err_str.contains("connect") || err_str.contains("dns")) {
                return Ok(Json(TestResult { success: false, message: format!("Connection failed: {}", e), version: None }));
            }
            tracing::warn!("[SETUP] FShare API unreachable ({}), trying web form login", err_str);
        }
    }

    let web_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let login_page = match web_client
        .get("https://www.fshare.vn/site/login")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return Ok(Json(TestResult { success: false, message: format!("Both API and web login unreachable: {}", e), version: None }));
        }
    };

    let initial_cookies: String = login_page.headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| s.split(';').next().map(|c| c.trim().to_string()))
        .collect::<Vec<_>>()
        .join("; ");

    let html = match login_page.text().await {
        Ok(h) => h,
        Err(_) => {
            return Ok(Json(TestResult { success: false, message: "Failed to read login page".to_string(), version: None }));
        }
    };

    let csrf_token = {
        let marker = "name=\"csrf-token\" content=\"";
        html.find(marker).and_then(|pos| {
            let start = pos + marker.len();
            html[start..].find('"').map(|end| html[start..start + end].to_string())
        })
    };

    let csrf_token = match csrf_token {
        Some(t) => t,
        None => {
            return Ok(Json(TestResult { success: false, message: "Could not extract CSRF token from login page".to_string(), version: None }));
        }
    };

    let form_body = format!(
        "_csrf-app={}&LoginForm%5Bemail%5D={}&LoginForm%5Bpassword%5D={}&LoginForm%5BrememberMe%5D=1",
        urlencoding::encode(&csrf_token),
        urlencoding::encode(&payload.email),
        urlencoding::encode(&payload.password),
    );

    match web_client
        .post("https://www.fshare.vn/site/login")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Referer", "https://www.fshare.vn/site/login")
        .header("Cookie", &initial_cookies)
        .body(form_body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status == reqwest::StatusCode::FOUND || status == reqwest::StatusCode::MOVED_PERMANENTLY {
                if let Err(e) = state.db.save_fshare_credentials(&payload.email, &payload.password) {
                    tracing::error!("Failed to save Fshare credentials: {}", e);
                    return Ok(Json(TestResult { success: false, message: "Failed to save credentials".to_string(), version: None }));
                }
                Ok(Json(TestResult { success: true, message: "Connected via web login (API unavailable)".to_string(), version: None }))
            } else if status == reqwest::StatusCode::OK {
                Ok(Json(TestResult { success: false, message: "Invalid credentials (verified via web login)".to_string(), version: None }))
            } else {
                Ok(Json(TestResult { success: false, message: format!("Unexpected web login response: {}", status), version: None }))
            }
        }
        Err(e) => Ok(Json(TestResult { success: false, message: format!("Both API and web login failed: {}", e), version: None })),
    }
}

pub async fn test_jellyfin(Json(payload): Json<JellyfinConfig>) -> Json<TestResult> {
    let client = &*HTTP_CLIENT;
    let url = format!("{}/System/Info", payload.url.trim_end_matches('/'));

    match client
        .get(&url)
        .header("X-Emby-Token", &payload.api_key)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    let version = data["Version"].as_str().map(|s| s.to_string());
                    Json(TestResult { success: true, message: "Connected to Jellyfin".to_string(), version })
                } else {
                    Json(TestResult { success: true, message: "Connected to Jellyfin".to_string(), version: None })
                }
            } else {
                Json(TestResult { success: false, message: format!("HTTP {}: Check API key", response.status()), version: None })
            }
        }
        Err(e) => Json(TestResult { success: false, message: format!("Connection failed: {}", e), version: None }),
    }
}

pub async fn get_indexer_key(State(state): State<Arc<AppState>>) -> Json<IndexerKeyResponse> {
    let api_key = state.db.get_indexer_api_key().unwrap_or_else(|_| {
        let key = uuid::Uuid::new_v4().to_string().replace("-", "");
        let _ = state.db.save_indexer_api_key(&key);
        key
    });
    Json(IndexerKeyResponse { api_key })
}

pub async fn complete_setup(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CompleteSetupPayload>,
) -> Json<SuccessResponse> {
    let mut fshare_message: Option<String> = None;
    if let Some(fshare) = payload.fshare {
        if let Err(e) = state.db.save_fshare_credentials(&fshare.email, &fshare.password) {
            tracing::error!("Failed to save Fshare credentials: {}", e);
            return Json(SuccessResponse { success: false, message: "Failed to save Fshare credentials".to_string() });
        }
        match refresh_fshare_rank_after_setup(&state).await {
            Ok(rank) => fshare_message = Some(format!("FShare {} verified", rank)),
            Err(e) => fshare_message = Some(format!("FShare saved, VIP verification pending: {}", e)),
        }
    }

    if let Err(e) = state.db.save_download_settings(&payload.downloads.directory, payload.downloads.max_concurrent) {
        tracing::error!("Failed to save download settings: {}", e);
        return Json(SuccessResponse { success: false, message: "Failed to save download settings".to_string() });
    }

    if let Some(jellyfin) = payload.jellyfin {
        if let Err(e) = state.db.save_jellyfin_config(&jellyfin.url, &jellyfin.api_key) {
            tracing::error!("Failed to save Jellyfin config: {}", e);
        }
    }

    if let Err(e) = state.db.mark_onboarding_complete() {
        tracing::error!("Failed to mark onboarding complete: {}", e);
        return Json(SuccessResponse { success: false, message: "Failed to complete setup".to_string() });
    }

    tracing::info!("Setup wizard completed successfully");
    Json(SuccessResponse { success: true, message: fshare_message.unwrap_or_else(|| "Setup completed successfully".to_string()) })
}
