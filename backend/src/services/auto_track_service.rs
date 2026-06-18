use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{AutoTrack, AutoTrackItem, Db};
use crate::downloader::{DownloadOrchestrator, TmdbDownloadMetadata};
use crate::utils::parser::FilenameParser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FshareTrackFile {
    pub name: String,
    pub url: String,
    pub fshare_code: String,
    pub size: u64,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub quality: String,
}

#[derive(Debug, Serialize)]
pub struct AutoTrackCheckReport {
    pub track_id: String,
    pub seen: usize,
    pub new_items: usize,
    pub queued: usize,
    pub skipped: usize,
    pub message: String,
}

#[derive(Clone)]
pub struct AutoTrackService {
    db: Arc<Db>,
    orchestrator: Arc<DownloadOrchestrator>,
    client: reqwest::Client,
}

impl AutoTrackService {
    pub fn new(db: Arc<Db>, orchestrator: Arc<DownloadOrchestrator>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .expect("auto-track reqwest client");
        Self { db, orchestrator, client }
    }

    pub async fn create_track(&self, mut track: AutoTrack) -> Result<AutoTrack> {
        if track.id.trim().is_empty() { track.id = Uuid::new_v4().to_string(); }
        if track.folder_code.trim().is_empty() { track.folder_code = extract_folder_code(&track.folder_url).ok_or_else(|| anyhow!("URL folder FShare không hợp lệ"))?; }
        if track.check_interval_secs <= 0 { track.check_interval_secs = 3600; }
        if track.media_type.trim().is_empty() { track.media_type = "tv".to_string(); }
        if track.category.trim().is_empty() { track.category = "tv".to_string(); }
        let now = Utc::now().to_rfc3339();
        if track.created_at.is_empty() { track.created_at = now.clone(); }
        track.updated_at = now;
        self.db.upsert_auto_track_async(track.clone()).await?;
        Ok(self.db
            .get_auto_track_by_folder_async(track.media_type.clone(), track.folder_code.clone())
            .await?
            .unwrap_or(track))
    }

    pub async fn baseline_track(&self, track_id: &str) -> Result<AutoTrackCheckReport> {
        let mut track = self.db.get_auto_track_async(track_id.to_string()).await?.ok_or_else(|| anyhow!("Không tìm thấy auto track"))?;
        let files = dedupe_episode_files(collect_fshare_files(&self.client, &track.folder_url, true, 0).await?);
        let mut new_items = 0usize;
        let mut skipped = 0usize;

        for file in files.iter().filter(|f| !f.fshare_code.is_empty()) {
            if self.db.get_auto_track_item_by_code_async(track.id.clone(), file.fshare_code.clone()).await?.is_some() {
                skipped += 1;
                continue;
            }
            new_items += 1;
            let parsed = FilenameParser::parse(&file.name);
            let season = file.season.or(parsed.season).map(|v| v as i32).or(track.season);
            let episode = file.episode.or(parsed.episode).map(|v| v as i32);
            let item = AutoTrackItem {
                id: Uuid::new_v4().to_string(),
                track_id: track.id.clone(),
                fshare_code: file.fshare_code.clone(),
                file_name: file.name.clone(),
                file_url: file.url.clone(),
                file_size: file.size as i64,
                season,
                episode,
                status: "seen".to_string(),
                download_id: None,
                first_seen_at: Utc::now().to_rfc3339(),
                queued_at: None,
                completed_at: None,
                error_message: None,
                auto_queued: false,
            };
            self.db.upsert_auto_track_item_async(item).await?;
        }

        track.last_checked_at = Some(Utc::now().to_rfc3339());
        track.last_error = None;
        track.updated_at = Utc::now().to_rfc3339();
        self.db.upsert_auto_track_async(track.clone()).await?;
        Ok(AutoTrackCheckReport { track_id: track.id, seen: files.len(), new_items, queued: 0, skipped, message: format!("Đã ghi nhận baseline {} file hiện có, không queue tải.", files.len()) })
    }

    pub async fn check_track(&self, track_id: &str) -> Result<AutoTrackCheckReport> {
        let mut track = self.db.get_auto_track_async(track_id.to_string()).await?.ok_or_else(|| anyhow!("Không tìm thấy auto track"))?;
        // Mark the attempt at the beginning so the UI shows the real background
        // watcher activity even if FShare scan/download matching later fails or is slow.
        track.last_checked_at = Some(Utc::now().to_rfc3339());
        track.last_error = None;
        track.updated_at = Utc::now().to_rfc3339();
        self.db.upsert_auto_track_async(track.clone()).await?;

        let files = dedupe_episode_files(collect_fshare_files(&self.client, &track.folder_url, true, 0).await?);
        let mut new_items = 0usize;
        let mut queued = 0usize;
        let mut skipped = 0usize;
        let batch_id = track.batch_id.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| format!("autotrack-{}", track.id));
        let batch_name = track.batch_name.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| track.title.clone());

        for file in files.iter().filter(|f| !f.fshare_code.is_empty()) {
            if self.db.get_auto_track_item_by_code_async(track.id.clone(), file.fshare_code.clone()).await?.is_some() {
                skipped += 1;
                continue;
            }
            new_items += 1;
            let parsed = FilenameParser::parse(&file.name);
            let season = file.season.or(parsed.season).map(|v| v as i32).or(track.season);
            let episode = file.episode.or(parsed.episode).map(|v| v as i32);
            let mut item = AutoTrackItem {
                id: Uuid::new_v4().to_string(),
                track_id: track.id.clone(),
                fshare_code: file.fshare_code.clone(),
                file_name: file.name.clone(),
                file_url: file.url.clone(),
                file_size: file.size as i64,
                season,
                episode,
                status: "seen".to_string(),
                download_id: None,
                first_seen_at: Utc::now().to_rfc3339(),
                queued_at: None,
                completed_at: None,
                error_message: None,
                auto_queued: false,
            };

            let meta = TmdbDownloadMetadata {
                tmdb_id: track.tmdb_id,
                media_type: Some("tv".to_string()),
                title: Some(track.title.clone()),
                year: track.year,
                collection_name: None,
                season,
                episode,
            };
            match self.orchestrator.add_download_with_metadata(
                file.url.clone(),
                Some(file.name.clone()),
                "fshare".to_string(),
                track.category.clone(),
                Some(meta),
                Some(batch_id.clone()),
                Some(batch_name.clone()),
                Some(track.title.clone()),
                track.download_folder.clone(),
            ).await {
                Ok(task) => {
                    item.status = "queued".to_string();
                    item.download_id = Some(task.id.to_string());
                    item.queued_at = Some(Utc::now().to_rfc3339());
                    item.auto_queued = true;
                    queued += 1;
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") { item.status = "skipped".to_string(); } else { item.status = "failed".to_string(); item.error_message = Some(msg); }
                }
            }
            self.db.upsert_auto_track_item_async(item).await?;
        }

        track.last_checked_at = Some(Utc::now().to_rfc3339());
        track.last_error = None;
        track.updated_at = Utc::now().to_rfc3339();
        self.db.upsert_auto_track_async(track.clone()).await?;
        Ok(AutoTrackCheckReport { track_id: track.id, seen: files.len(), new_items, queued, skipped, message: format!("Đã quét {} file, thêm {} file mới, queue {} file.", files.len(), new_items, queued) })
    }

    pub async fn check_due_tracks(&self) {
        let tracks = match self.db.list_due_auto_tracks_async().await {
            Ok(tracks) => tracks,
            Err(e) => {
                tracing::warn!("Auto-track due list failed: {}", e);
                return;
            }
        };
        tracing::info!(count = tracks.len(), "Auto-track due list loaded");
        if !tracks.is_empty() {
            tracing::info!(count = tracks.len(), "Auto-track due check starting");
        }
        for track in tracks {
            tracing::info!(track_id=%track.id, title=%track.title, "Auto-track checking due track");
            if let Err(e) = self.check_track(&track.id).await {
                tracing::warn!(track_id=%track.id, title=%track.title, "Auto-track check failed: {}", e);
                let _ = self.db.update_auto_track_error_async(track.id, e.to_string()).await;
            }
        }
    }
}

fn normalize_external_url(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(u_part) = trimmed.split("u=").nth(1) {
        let encoded = u_part.split('&').next().unwrap_or("");
        if let Ok(decoded) = urlencoding::decode(encoded) { return decoded.to_string(); }
    }
    trimmed.to_string()
}

pub fn extract_folder_code(folder_url: &str) -> Option<String> {
    let normalized = normalize_external_url(folder_url);
    if !normalized.contains("/folder/") { return None; }
    let after = normalized.split("/folder/").last()?;
    let code = after.split(|c| c == '?' || c == '&' || c == '/').next().unwrap_or("");
    if code.is_empty() { None } else { Some(code.to_string()) }
}

fn extract_folder_token(folder_url: &str) -> Option<String> {
    let query = folder_url.split('?').nth(1)?;
    query.split('&').find_map(|part| part.strip_prefix("token=").map(|s| s.to_string()))
}

async fn fetch_page(client: &reqwest::Client, folder_code: &str, token: Option<&str>, page: u32) -> Result<Vec<serde_json::Value>> {
    let mut query = vec![
        ("linkcode", folder_code.to_string()),
        ("sort", "type".to_string()),
        ("page", page.to_string()),
        // FShare uses `per-page` for folder page size. `limit` alone falls
        // back to the default page size (10), so keep both for compatibility.
        ("per-page", "100".to_string()),
        ("limit", "100".to_string()),
    ];
    if let Some(t) = token.filter(|s| !s.is_empty()) { query.push(("token", t.to_string())); }
    let body: serde_json::Value = client.get("https://www.fshare.vn/api/v3/files/folder")
        .query(&query)
        .header("User-Agent", "Mozilla/5.0")
        .send().await?
        .error_for_status()?
        .json().await?;
    Ok(body.get("items").and_then(|v| v.as_array()).or_else(|| body.get("data").and_then(|v| v.as_array())).cloned().unwrap_or_default())
}

fn str_field(v: &serde_json::Value, key: &str) -> Option<String> { v.get(key).and_then(|x| x.as_str().map(str::to_string).or_else(|| x.as_i64().map(|n| n.to_string()))) }
fn u64_field(v: &serde_json::Value, key: &str) -> u64 { v.get(key).and_then(|x| x.as_u64().or_else(|| x.as_str()?.parse().ok())).unwrap_or(0) }

fn episode_quality_score(file: &FshareTrackFile) -> u64 {
    let name = file.name.to_lowercase();
    let resolution_score = if name.contains("2160p") || name.contains("4k") { 4_000_000_000 }
        else if name.contains("1080p") { 2_000_000_000 }
        else if name.contains("720p") { 1_000_000_000 }
        else { 0 };
    let codec_score = if name.contains("h.265") || name.contains("x265") || name.contains("hevc") { 200_000_000 } else { 0 };
    let hdr_score = if name.contains("dv") || name.contains("hdr") { 100_000_000 } else { 0 };
    resolution_score + codec_score + hdr_score + file.size
}

fn dedupe_episode_files(files: Vec<FshareTrackFile>) -> Vec<FshareTrackFile> {
    let mut episodic: HashMap<(u32, u32), FshareTrackFile> = HashMap::new();
    let mut loose = Vec::new();
    for file in files {
        match (file.season, file.episode) {
            (Some(season), Some(episode)) => {
                let key = (season, episode);
                let replace = episodic.get(&key)
                    .map(|existing| episode_quality_score(&file) > episode_quality_score(existing))
                    .unwrap_or(true);
                if replace { episodic.insert(key, file); }
            }
            _ => loose.push(file),
        }
    }
    let mut out: Vec<FshareTrackFile> = episodic.into_values().collect();
    out.sort_by_key(|f| (f.season.unwrap_or(0), f.episode.unwrap_or(0), f.name.clone()));
    out.extend(loose);
    out
}

fn item_is_dir(v: &serde_json::Value) -> bool {
    let typ = str_field(v, "type").unwrap_or_default();
    let mime = str_field(v, "mimetype").or_else(|| str_field(v, "mimeType")).unwrap_or_default();
    typ == "0" || mime.is_empty()
}

async fn collect_fshare_files(client: &reqwest::Client, folder_url: &str, recursive: bool, depth: u32) -> Result<Vec<FshareTrackFile>> {
    if depth > 8 { return Ok(vec![]); }
    let normalized = normalize_external_url(folder_url);
    let folder_code = extract_folder_code(&normalized).ok_or_else(|| anyhow!("URL folder FShare không hợp lệ"))?;
    let token = extract_folder_token(&normalized);
    let mut out = Vec::new();
    let mut page = 1u32;
    loop {
        let items = fetch_page(client, &folder_code, token.as_deref(), page).await?;
        if items.is_empty() { break; }
        let page_len = items.len();
        for v in items {
            let Some(code) = str_field(&v, "linkcode").or_else(|| str_field(&v, "linkCode")) else { continue; };
            let Some(name) = str_field(&v, "name") else { continue; };
            if item_is_dir(&v) {
                if recursive {
                    let child = match &token { Some(t) if !t.is_empty() => format!("https://www.fshare.vn/folder/{}?token={}", code, t), _ => format!("https://www.fshare.vn/folder/{}", code) };
                    let mut nested = Box::pin(collect_fshare_files(client, &child, true, depth + 1)).await?;
                    out.append(&mut nested);
                }
            } else {
                let parsed = FilenameParser::parse(&name);
                out.push(FshareTrackFile { name, url: format!("https://www.fshare.vn/file/{}", code), fshare_code: code, size: u64_field(&v, "size"), season: parsed.season, episode: parsed.episode, quality: parsed.quality_attrs.quality_name() });
            }
        }
        if page_len < 50 { break; }
        page += 1;
        if page > 100 { break; }
    }
    Ok(out)
}
