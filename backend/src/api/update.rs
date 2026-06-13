use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::AppState;

const DEFAULT_REPO: &str = "Andyict/fhub-aio-downloader";
const DEFAULT_IMAGE: &str = "ghcr.io/andyict/fhub-aio:latest";
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

#[derive(Serialize, Deserialize)]
pub struct UpdateRunResponse {
    pub success: bool,
    pub message: String,
    pub logs: Vec<String>,
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

fn updater_url() -> Option<String> {
    std::env::var("FHUB_UPDATER_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
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

async fn updater_available() -> bool {
    if let Some(updater) = updater_url() {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
            Ok(client) => client,
            Err(_) => return false,
        };
        return client
            .get(format!("{updater}/health"))
            .send()
            .await
            .map(|resp| resp.status().is_success())
            .unwrap_or(false);
    }

    if !Path::new(DOCKER_SOCKET).exists() {
        return false;
    }
    UnixStream::connect(DOCKER_SOCKET).await.is_ok()
}


async fn local_image_commit(image: &str) -> Option<String> {
    let inspect_path = format!("/images/{}/json", urlencoding::encode(image));
    let Ok((status, body)) = docker_request("GET", &inspect_path, None).await else { return None; };
    if status != 200 { return None; }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) else { return None; };
    value
        .pointer("/Config/Labels/org.opencontainers.image.revision")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v != "dev")
}

async fn pull_image_for_status(image: &str) {
    if !Path::new(DOCKER_SOCKET).exists() || UnixStream::connect(DOCKER_SOCKET).await.is_err() {
        return;
    }
    if !image.contains('/') {
        return;
    }
    let pull_path = format!("/images/create?fromImage={}", urlencoding::encode(image));
    let _ = tokio::time::timeout(Duration::from_secs(45), docker_request("POST", &pull_path, None)).await;
}

fn short_sha(value: &str) -> String {
    value.chars().take(12).collect()
}

async fn update_status(State(state): State<Arc<AppState>>) -> Result<Json<UpdateStatusResponse>, StatusCode> {
    let current = current_commit();
    let image = update_image();
    let (mut latest, latest_url, mut message) = match latest_commit(&state).await {
        Ok((sha, url)) => (Some(sha), url, "Đã kiểm tra update từ GitHub.".to_string()),
        Err(err) => (None, None, err),
    };
    if latest.is_none() {
        pull_image_for_status(&image).await;
        if let Some(image_commit) = local_image_commit(&image).await {
            latest = Some(image_commit);
            message = "Đã kiểm tra update từ Docker image metadata.".to_string();
        }
    }
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
        updater_available: updater_available().await,
        image,
        container: container_name(),
        message,
    }))
}

fn decode_chunked_body(body: &str) -> String {
    let bytes = body.as_bytes();
    let mut pos = 0usize;
    let mut out = Vec::new();
    loop {
        let Some(line_end_rel) = bytes[pos..].windows(2).position(|w| w == b"\r\n") else { break; };
        let line_end = pos + line_end_rel;
        let size_line = String::from_utf8_lossy(&bytes[pos..line_end]);
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size_hex, 16) else { return body.to_string(); };
        pos = line_end + 2;
        if size == 0 { break; }
        if pos + size > bytes.len() { return body.to_string(); }
        out.extend_from_slice(&bytes[pos..pos + size]);
        pos += size;
        if pos + 2 <= bytes.len() && &bytes[pos..pos + 2] == b"\r\n" { pos += 2; }
    }
    String::from_utf8_lossy(&out).to_string()
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
    let response_body_raw = parts.next().unwrap_or_default().to_string();
    let is_chunked = head.lines().any(|line| line.to_ascii_lowercase().starts_with("transfer-encoding:") && line.to_ascii_lowercase().contains("chunked"));
    let response_body = if is_chunked { decode_chunked_body(&response_body_raw) } else { response_body_raw };
    let status = head.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(500);
    Ok((status, response_body))
}

async fn run_update() -> Result<Json<UpdateRunResponse>, StatusCode> {
    if let Some(updater) = updater_url() {
        let payload = json!({
            "image": update_image(),
            "container": container_name(),
        });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(900))
            .build()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match client.post(format!("{updater}/update"))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<UpdateRunResponse>().await {
                Ok(result) => return Ok(Json(result)),
                Err(e) => return Ok(Json(UpdateRunResponse { success: false, message: format!("Updater trả phản hồi không đọc được: {e}"), logs: vec![] })),
            },
            Err(e) => return Ok(Json(UpdateRunResponse { success: false, message: format!("Không gọi được updater helper: {e}"), logs: vec![] })),
        }
    }

    perform_docker_update(update_image(), container_name()).await
}

async fn rollback_update(container: &str, backup: &str, logs: &mut Vec<String>) {
    logs.push("Rolling back to previous container".to_string());
    let remove_new = format!("/containers/{}?v=false&force=true", urlencoding::encode(container));
    let _ = docker_request("DELETE", &remove_new, None).await;
    let rollback_path = format!("/containers/{}/rename?name={}", urlencoding::encode(backup), urlencoding::encode(container));
    let _ = docker_request("POST", &rollback_path, None).await;
    let old_start = format!("/containers/{}/start", urlencoding::encode(container));
    let _ = docker_request("POST", &old_start, None).await;
}

async fn wait_for_container_healthy(container: &str, timeout_secs: u64, logs: &mut Vec<String>) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let inspect_path = format!("/containers/{}/json", urlencoding::encode(container));
    let mut last_status = String::new();

    loop {
        let (status, body) = docker_request("GET", &inspect_path, None)
            .await
            .map_err(|e| format!("Không kiểm tra được container mới: {e}"))?;
        if status != 200 {
            return Err(format!("Không tìm thấy container mới sau khi start HTTP {status}"));
        }

        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Không đọc được trạng thái container mới: {e}"))?;
        let running = value.pointer("/State/Running").and_then(|v| v.as_bool()).unwrap_or(false);
        let exit_code = value.pointer("/State/ExitCode").and_then(|v| v.as_i64()).unwrap_or_default();
        let health = value.pointer("/State/Health/Status").and_then(|v| v.as_str());

        match health {
            Some("healthy") => {
                logs.push("Updated container is healthy".to_string());
                return Ok(());
            }
            Some(current) if current != last_status => {
                logs.push(format!("Waiting for updated container health: {current}"));
                last_status = current.to_string();
            }
            None if running => {
                logs.push("Updated container is running; no Docker healthcheck configured".to_string());
                return Ok(());
            }
            _ if !running => return Err(format!("Container mới đã dừng sớm với exit code {exit_code}")),
            _ => {}
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Container mới chưa healthy sau {timeout_secs}s"));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

pub async fn perform_docker_update(image: String, container: String) -> Result<Json<UpdateRunResponse>, StatusCode> {
    let mut logs = Vec::new();

    if !updater_available().await {
        return Ok(Json(UpdateRunResponse {
            success: false,
            message: "Chưa bật web updater. Cần mount /var/run/docker.sock vào container FHub rồi restart một lần.".to_string(),
            logs,
        }));
    }

    if image.contains('/') {
        logs.push(format!("Pulling image {image}"));
        let pull_path = format!("/images/create?fromImage={}", urlencoding::encode(&image));
        match tokio::time::timeout(Duration::from_secs(600), docker_request("POST", &pull_path, None)).await {
            Ok(Ok((status, body))) if (200..300).contains(&status) => logs.push(format!("Pull complete: {}", body.lines().last().unwrap_or("ok"))),
            Ok(Ok((status, body))) => return Ok(Json(UpdateRunResponse { success: false, message: format!("Pull image thất bại HTTP {status}"), logs: [logs, vec![body]].concat() })),
            Ok(Err(e)) => return Ok(Json(UpdateRunResponse { success: false, message: e, logs })),
            Err(_) => return Ok(Json(UpdateRunResponse { success: false, message: "Pull image quá thời gian.".to_string(), logs })),
        }
    } else {
        logs.push(format!("Using local image {image}"));
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
    // Do not reuse inspected NetworkSettings as create-time NetworkingConfig.
    // Docker's inspect payload contains runtime endpoint details that can be stale
    // after renaming/stopping the old container. HostConfig.NetworkMode is enough
    // for compose/default networks and avoids create failures during self-update.
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
            logs.push("Updated container started; verifying health before cleanup".to_string());
            match wait_for_container_healthy(&container, 120, &mut logs).await {
                Ok(()) => {
                    let remove_path = format!("/containers/{}?v=false&force=true", urlencoding::encode(&backup));
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(20)).await;
                        let _ = docker_request("DELETE", &remove_path, None).await;
                    });
                    Ok(Json(UpdateRunResponse { success: true, message: "Cập nhật thành công. FHub đã khởi động lại và healthy.".to_string(), logs }))
                }
                Err(e) => {
                    rollback_update(&container, &backup, &mut logs).await;
                    Ok(Json(UpdateRunResponse { success: false, message: format!("{e}; đã rollback về bản cũ."), logs }))
                }
            }
        }
        Ok((status, body)) => {
            rollback_update(&container, &backup, &mut logs).await;
            Ok(Json(UpdateRunResponse { success: false, message: format!("Start container mới thất bại HTTP {status}; đã rollback."), logs: [logs, vec![body]].concat() }))
        }
        Err(e) => {
            rollback_update(&container, &backup, &mut logs).await;
            Ok(Json(UpdateRunResponse { success: false, message: format!("{e}; đã rollback."), logs }))
        }
    }
}
