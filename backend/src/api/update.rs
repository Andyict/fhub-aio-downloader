use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::AppState;

const DEFAULT_REPO: &str = "Andyict/fhub-aio-downloader";
const DEFAULT_IMAGE: &str = "ghcr.io/andyict/fhub-aio-downloader:latest";
const DEFAULT_CONTAINER: &str = "fhub";
const DOCKER_SOCKET: &str = "/var/run/docker.sock";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(update_status))
        .route("/run", post(run_update))
}

#[derive(Serialize)]
struct UpdateStatusResponse {
    current_version: String,
    current_commit: Option<String>,
    latest_commit: Option<String>,
    latest_commit_url: Option<String>,
    update_available: bool,
    updater_available: bool,
    image: String,
    container: String,
    message: String,
}

#[derive(Serialize)]
struct UpdateRunResponse {
    success: bool,
    message: String,
    logs: Vec<String>,
}

#[derive(Deserialize)]
struct GithubCommitResponse {
    sha: String,
    html_url: Option<String>,
}

fn current_version() -> String {
    std::fs::read_to_string("/app/VERSION")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn current_commit() -> Option<String> {
    std::fs::read_to_string("/app/BUILD_INFO")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("Git Commit:")
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty() && value != "dev")
            })
        })
        .or_else(|| option_env!("FHUB_VCS_REF").map(|v| v.to_string()))
        .filter(|value| !value.is_empty() && value != "dev")
}

fn github_repo() -> String {
    std::env::var("FHUB_UPDATE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string())
}

fn github_branch() -> String {
    std::env::var("FHUB_UPDATE_BRANCH").unwrap_or_else(|_| "main".to_string())
}

fn update_image() -> String {
    std::env::var("FHUB_UPDATE_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string())
}

fn container_name() -> String {
    std::env::var("FHUB_CONTAINER_NAME").unwrap_or_else(|_| DEFAULT_CONTAINER.to_string())
}

async fn latest_commit(state: &AppState) -> Result<(String, Option<String>), String> {
    let repo = github_repo();
    let branch = github_branch();
    let url = format!("https://api.github.com/repos/{repo}/commits/{branch}");
    let response = state.http_client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "FHUB-Updater")
        .send()
        .await
        .map_err(|e| format!("Không kết nối được GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub trả HTTP {}", response.status()));
    }
    let payload: GithubCommitResponse = response.json().await.map_err(|e| format!("Không đọc được phản hồi GitHub: {e}"))?;
    Ok((payload.sha, payload.html_url))
}

fn updater_available() -> bool {
    Path::new(DOCKER_SOCKET).exists()
}

fn short_sha(value: &str) -> String {
    value.chars().take(12).collect()
}

async fn update_status(State(state): State<Arc<AppState>>) -> Result<Json<UpdateStatusResponse>, StatusCode> {
    let current = current_commit();
    let (latest, latest_url, message) = match latest_commit(&state).await {
        Ok((sha, url)) => (Some(sha), url, "Đã kiểm tra update từ GitHub.".to_string()),
        Err(err) => (None, None, err),
    };
    let update_available = match (&current, &latest) {
        (Some(current), Some(latest)) => !latest.starts_with(current) && !current.starts_with(latest),
        (None, Some(_)) => true,
        _ => false,
    };

    Ok(Json(UpdateStatusResponse {
        current_version: current_version(),
        current_commit: current.map(|v| short_sha(&v)),
        latest_commit: latest.map(|v| short_sha(&v)),
        latest_commit_url: latest_url,
        update_available,
        updater_available: updater_available(),
        image: update_image(),
        container: container_name(),
        message,
    }))
}

async fn docker_request(method: &str, path: &str, body: Option<String>) -> Result<(u16, String), String> {
    let mut stream = UnixStream::connect(DOCKER_SOCKET)
        .await
        .map_err(|e| format!("Docker socket chưa bật hoặc không truy cập được: {e}"))?;
    let body = body.unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: docker\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    stream.write_all(request.as_bytes()).await.map_err(|e| format!("Không gửi được lệnh Docker: {e}"))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.map_err(|e| format!("Không đọc được Docker response: {e}"))?;
    let raw = String::from_utf8_lossy(&buf).to_string();
    let mut parts = raw.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or_default();
    let response_body = parts.next().unwrap_or_default().to_string();
    let status = head.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(500);
    Ok((status, response_body))
}

async fn run_update() -> Result<Json<UpdateRunResponse>, StatusCode> {
    let image = update_image();
    let container = container_name();
    let mut logs = Vec::new();

    if !updater_available() {
        return Ok(Json(UpdateRunResponse {
            success: false,
            message: "Chưa bật web updater. Cần mount /var/run/docker.sock vào container FHub rồi restart một lần.".to_string(),
            logs,
        }));
    }

    logs.push(format!("Pulling image {image}"));
    let pull_path = format!("/images/create?fromImage={}", urlencoding::encode(&image));
    match tokio::time::timeout(Duration::from_secs(600), docker_request("POST", &pull_path, None)).await {
        Ok(Ok((status, body))) if (200..300).contains(&status) => logs.push(format!("Pull complete: {}", body.lines().last().unwrap_or("ok"))),
        Ok(Ok((status, body))) => return Ok(Json(UpdateRunResponse { success: false, message: format!("Pull image thất bại HTTP {status}"), logs: [logs, vec![body]].concat() })),
        Ok(Err(e)) => return Ok(Json(UpdateRunResponse { success: false, message: e, logs })),
        Err(_) => return Ok(Json(UpdateRunResponse { success: false, message: "Pull image quá thời gian.".to_string(), logs })),
    }

    logs.push(format!("Inspecting container {container}"));
    let inspect_path = format!("/containers/{}/json", urlencoding::encode(&container));
    let (inspect_status, inspect_body) = match docker_request("GET", &inspect_path, None).await {
        Ok(result) => result,
        Err(e) => return Ok(Json(UpdateRunResponse { success: false, message: e, logs })),
    };
    if inspect_status != 200 {
        return Ok(Json(UpdateRunResponse { success: false, message: format!("Không tìm thấy container {container}."), logs }));
    }
    let inspect_json: serde_json::Value = match serde_json::from_str(&inspect_body) {
        Ok(value) => value,
        Err(e) => return Ok(Json(UpdateRunResponse { success: false, message: format!("Không đọc được cấu hình container hiện tại: {e}"), logs })),
    };

    let host_config = inspect_json.get("HostConfig").cloned().unwrap_or_else(|| json!({}));
    let mut config = inspect_json.get("Config").cloned().unwrap_or_else(|| json!({}));
    config["Image"] = json!(image);
    let networking_config = json!({
        "EndpointsConfig": inspect_json
            .get("NetworkSettings")
            .and_then(|n| n.get("Networks"))
            .cloned()
            .unwrap_or_else(|| json!({}))
    });
    let create_body = json!({
        "Hostname": config.get("Hostname").cloned().unwrap_or_else(|| json!("")),
        "Domainname": config.get("Domainname").cloned().unwrap_or_else(|| json!("")),
        "User": config.get("User").cloned().unwrap_or_else(|| json!("")),
        "Env": config.get("Env").cloned().unwrap_or_else(|| json!([])),
        "Cmd": config.get("Cmd").cloned().unwrap_or_else(|| json!(null)),
        "Entrypoint": config.get("Entrypoint").cloned().unwrap_or_else(|| json!(null)),
        "Image": image,
        "Labels": config.get("Labels").cloned().unwrap_or_else(|| json!({})),
        "ExposedPorts": config.get("ExposedPorts").cloned().unwrap_or_else(|| json!({})),
        "WorkingDir": config.get("WorkingDir").cloned().unwrap_or_else(|| json!("")),
        "HostConfig": host_config,
        "NetworkingConfig": networking_config,
    }).to_string();

    let backup = format!("{}-old-{}", container, chrono::Utc::now().format("%Y%m%d%H%M%S"));
    logs.push(format!("Renaming old container to {backup}"));
    let rename_path = format!("/containers/{}/rename?name={}", urlencoding::encode(&container), urlencoding::encode(&backup));
    let _ = docker_request("POST", &rename_path, None).await;

    logs.push("Stopping old container".to_string());
    let stop_path = format!("/containers/{}/stop?t=10", urlencoding::encode(&backup));
    let _ = docker_request("POST", &stop_path, None).await;

    logs.push("Creating updated container".to_string());
    let create_path = format!("/containers/create?name={}", urlencoding::encode(&container));
    let (create_status, create_response) = match docker_request("POST", &create_path, Some(create_body)).await {
        Ok(result) => result,
        Err(e) => return Ok(Json(UpdateRunResponse { success: false, message: e, logs })),
    };
    if !(200..300).contains(&create_status) {
        let rollback_path = format!("/containers/{}/rename?name={}", urlencoding::encode(&backup), urlencoding::encode(&container));
        let _ = docker_request("POST", &rollback_path, None).await;
        let start_path = format!("/containers/{}/start", urlencoding::encode(&container));
        let _ = docker_request("POST", &start_path, None).await;
        return Ok(Json(UpdateRunResponse { success: false, message: format!("Tạo container mới thất bại HTTP {create_status}"), logs: [logs, vec![create_response]].concat() }));
    }

    logs.push("Starting updated container".to_string());
    let start_path = format!("/containers/{}/start", urlencoding::encode(&container));
    match docker_request("POST", &start_path, None).await {
        Ok((status, body)) if (200..300).contains(&status) || status == 304 => {
            logs.push("Updated container started. Current request may disconnect while FHub restarts.".to_string());
            let remove_path = format!("/containers/{}?v=false&force=true", urlencoding::encode(&backup));
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(20)).await;
                let _ = docker_request("DELETE", &remove_path, None).await;
            });
            Ok(Json(UpdateRunResponse { success: true, message: "Đã bắt đầu cập nhật. FHub sẽ khởi động lại trong vài giây.".to_string(), logs }))
        }
        Ok((status, body)) => {
            let remove_new = format!("/containers/{}?v=false&force=true", urlencoding::encode(&container));
            let _ = docker_request("DELETE", &remove_new, None).await;
            let rollback_path = format!("/containers/{}/rename?name={}", urlencoding::encode(&backup), urlencoding::encode(&container));
            let _ = docker_request("POST", &rollback_path, None).await;
            let old_start = format!("/containers/{}/start", urlencoding::encode(&container));
            let _ = docker_request("POST", &old_start, None).await;
            Ok(Json(UpdateRunResponse { success: false, message: format!("Start container mới thất bại HTTP {status}; đã rollback."), logs: [logs, vec![body]].concat() }))
        }
        Err(e) => Ok(Json(UpdateRunResponse { success: false, message: e, logs })),
    }
}
