//! Download Orchestrator
//!
//! Coordinates between TaskManager and DownloadEngine.
//! Handles URL resolution, retry logic, and progress broadcasting.

use std::sync::Arc;
use std::time::Duration;
use std::path::PathBuf;
use tokio::sync::{broadcast, Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use chrono::Utc;

use super::config::DownloadConfig;
use super::stats::EngineStats;
use super::engine_simple::SimpleDownloadEngine;
use super::manager::DownloadTaskManager;
use super::task::{DownloadTask, DownloadState, UrlMetadata};
use super::progress::{ProgressUpdate, TaskEvent};
use super::path_builder::{TmdbDownloadMetadata, PathBuilder};
use super::duplicate_detector::DuplicateDetector;
use crate::hosts::registry::HostRegistry;
use crate::db::Db;
use crate::utils::parser::FilenameParser;

/// Download orchestrator - coordinates all download operations
pub struct DownloadOrchestrator {
    /// Task manager (Single Source of Truth)
    task_manager: Arc<DownloadTaskManager>,
    
    /// HTTP download engine (wrapped for dynamic updates)
    download_engine: Arc<RwLock<Arc<SimpleDownloadEngine>>>,
    
    /// Host registry for URL resolution
    host_registry: Arc<HostRegistry>,
    
    /// Database connection
    db: Option<Arc<Db>>,
    
    /// Progress broadcast channel (legacy, will be deprecated)
    progress_tx: broadcast::Sender<ProgressUpdate>,
    
    /// Event bus for event-driven architecture
    event_bus: Arc<super::events::EventBus>,
    
    /// Worker handles
    workers: Mutex<Vec<JoinHandle<()>>>,
    
    /// Configuration (wrapped for dynamic updates)
    config: Arc<RwLock<DownloadConfig>>,
    
    /// Running state
    running: Arc<std::sync::atomic::AtomicBool>,
    
    /// Task notification for workers (wake when new task added)
    task_notify: Arc<Notify>,
    
    /// NAS API client for bi-directional sync (wrapped for dynamic updates)
    arr_client: Arc<RwLock<Option<Arc<crate::arr::FHubClient>>>>,
    
    /// NAS artifact manager for Series/Movie metadata (Phase 1: Artifact Management)
    artifact_manager: Arc<RwLock<Option<Arc<crate::arr::FHubArtifactManager>>>>,
    

    
    /// Track announced batches to prevent duplicate announcements
    announced_batches: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl DownloadOrchestrator {
    /// Create new orchestrator
    pub fn new(
        config: DownloadConfig,
        host_registry: Arc<HostRegistry>,
        db: Option<Arc<Db>>,
        tv_config: Option<crate::config::FHubConfig>,
        movie_config: Option<crate::config::FHubConfig>,
    ) -> Self {
        let task_manager = Arc::new(DownloadTaskManager::new());
        let download_engine = Arc::new(SimpleDownloadEngine::with_config(config.clone()));
        let (progress_tx, _) = broadcast::channel(1000);
        
        // Create event bus for event-driven architecture
        let event_bus = Arc::new(super::events::EventBus::new(1000));
        
        // Phase 4: Start TaskManager event listener for auto-updating cache
        let event_rx = event_bus.subscribe();
        Arc::clone(&task_manager).start_event_listener(event_rx);
        Arc::clone(&task_manager).start_cleanup_loop();
        tracing::info!("Phase 4 activated: TaskManager event-driven cache with auto-eviction + background cleanup");
        
        // External library integration disabled in standalone FHub mode
        tracing::info!("FHub standalone mode: external library integration disabled (tv={}, movie={})",
            tv_config.is_some(), movie_config.is_some());
        let arr_client = None;
        
        // Create artifact manager for Series/Movie metadata management
        let artifact_manager = if let (Some(ref client), Some(ref database)) = (&arr_client, &db) {
            tracing::info!("Creating FHubArtifactManager for artifact lifecycle management");
            Some(Arc::new(crate::arr::FHubArtifactManager::new(
                Arc::clone(client),
                Arc::clone(database),
            )))
        } else {
            None
        };
        

        
        Self {
            task_manager,
            download_engine: Arc::new(RwLock::new(download_engine)),
            host_registry,
            db,
            progress_tx,
            event_bus,
            workers: Mutex::new(Vec::new()),
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            task_notify: Arc::new(Notify::new()),
            arr_client: Arc::new(RwLock::new(arr_client)),
            artifact_manager: Arc::new(RwLock::new(artifact_manager)),

            announced_batches: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }
    
    /// Start orchestrator with workers
    pub async fn start(&self) {
        use std::sync::atomic::Ordering;
        
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        {
            let config_guard = self.config.read().await;
            tracing::info!("Starting download orchestrator with {} workers", config_guard.max_concurrent);
        }
        
        // Restore tasks from database
        if let Err(e) = self.restore_from_db().await {
            tracing::error!("Failed to restore tasks from database: {}", e);
        }
        
        // Spawn worker tasks
        let mut workers = self.workers.lock().await;
        // Acquire config read lock to get max_concurrent
        let max_concurrent = self.config.read().await.max_concurrent;
        for worker_id in 0..max_concurrent {
            let handle = self.spawn_worker(worker_id);
            workers.push(handle);
        }
    }
    
    /// Stop orchestrator
    pub async fn stop(&self) {
        use std::sync::atomic::Ordering;
        
        self.running.store(false, Ordering::SeqCst);
        
        // Wait for workers to finish
        let mut workers = self.workers.lock().await;
        for worker in workers.drain(..) {
            let _ = worker.await;
        }
        
        tracing::info!("Download orchestrator stopped");
    }
    
    
    /// Add a new download with duplicate detection
    pub async fn add_download(
        &self,
        url: String,
        filename: String,
        host: String,
        category: String,
    ) -> Result<DownloadTask, anyhow::Error> {
        self.add_download_with_metadata(url, Some(filename), host, category, None, None, None, None, None).await
    }
    
    /// Add a new download with TMDB metadata for organized folder structure
    pub async fn add_download_with_metadata(
        &self,
        url: String,
        filename_override: Option<String>,
        host: String,
        category: String,
        mut tmdb_metadata: Option<TmdbDownloadMetadata>,
        batch_id: Option<String>,
        batch_name: Option<String>,
        folder_name: Option<String>,
        download_folder: Option<String>,
    ) -> Result<DownloadTask, anyhow::Error> {
        // === DEBUG: Log orchestrator input ===
        tracing::info!("=== [ORCHESTRATOR] add_download_with_metadata called ===");
        tracing::info!("[ORCHESTRATOR] Input URL: {}", url);
        tracing::info!("[ORCHESTRATOR] Input filename_override: {:?}", filename_override);
        tracing::info!("[ORCHESTRATOR] Input host: {}", host);
        tracing::info!("[ORCHESTRATOR] Input category: {}", category);
        if let Some(ref meta) = tmdb_metadata {
            tracing::info!("[ORCHESTRATOR] TMDB: tmdb_id={:?}, media_type={:?}, title={:?}", 
                meta.tmdb_id, meta.media_type, meta.title);
        }
        
        // Extract Fshare code from URL for duplicate detection
        let fshare_code = Self::extract_fshare_code(&url);
        tracing::info!("[ORCHESTRATOR] Extracted fshare_code: {:?}", fshare_code);
        
        // Check for duplicates in memory first (fast path)
        if let Some(code) = &fshare_code {
            if let Some(existing) = self.find_task_by_fshare_code(code) {
                let filename = filename_override.unwrap_or_else(|| "unknown".to_string());
                return self.handle_duplicate(existing, url, filename, host, category, code.clone()).await;
            }
        }
        
        // Check for duplicates in database (slow path - catches evicted tasks)
        if let (Some(code), Some(db)) = (&fshare_code, &self.db) {
            match db.find_task_by_fshare_code_async(code).await {
                Ok(Some((existing_id, existing_state))) => {
                    // Found existing task in database with same fshare code
                    match existing_state.as_str() {
                        "QUEUED" | "STARTING" | "DOWNLOADING" | "PAUSED" | "COMPLETED" => {
                            tracing::info!(
                                "Duplicate detected in DB [fshare_code: {}]: Task {} already exists in state {}, skipping",
                                code, existing_id, existing_state
                            );
                            return Err(anyhow::anyhow!("Download already exists ({})", existing_state));
                        }
                        "FAILED" | "CANCELLED" => {
                            tracing::info!(
                                "Duplicate detected in DB [fshare_code: {}]: Task {} is {}, deleting old and creating new",
                                code, existing_id, existing_state
                            );
                            // Delete old task from database
                            if let Ok(old_id) = uuid::Uuid::parse_str(&existing_id) {
                                let _ = db.delete_task(old_id);
                            }
                        }
                        _ => {
                            tracing::info!(
                                "Duplicate detected in DB [fshare_code: {}]: Task {} in state {}, skipping",
                                code, existing_id, existing_state
                            );
                            return Err(anyhow::anyhow!("Download already exists ({})", existing_state));
                        }
                    }
                }
                Ok(None) => {
                    // No duplicate found in DB, proceed
                }
                Err(e) => {
                    tracing::warn!("Failed to check DB for duplicate fshare code: {}", e);
                    // Continue anyway - better to have a potential duplicate than fail
                }
            }
        }
        
        // Fetch real filename and size from Fshare API.
        // Even when the frontend supplies a display filename, still try to fetch file size
        // so progress/speed and segmented downloads are initialized correctly.
        let (filename, file_size) = if let Some(handler) = self.host_registry.get_handler(&host) {
            match tokio::time::timeout(std::time::Duration::from_secs(12), handler.get_file_info(&url)).await {
                Ok(Ok(file_info)) => {
                    tracing::info!("Fetched file info from API: name='{}', size={}", file_info.filename, file_info.size);
                    (filename_override.clone().unwrap_or(file_info.filename), file_info.size)
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to get file info from API, using fallback: {}", e);
                    let final_name = filename_override.clone().unwrap_or_else(|| url.split('/').last().unwrap_or("unknown").to_string());
                    (final_name, 0u64)
                }
                Err(_) => {
                    tracing::warn!("Timed out fetching file info from API, using fallback filename");
                    let final_name = filename_override.clone().unwrap_or_else(|| url.split('/').last().unwrap_or("unknown").to_string());
                    (final_name, 0u64)
                }
            }
        } else {
            let final_name = filename_override.clone().unwrap_or_else(|| url.split('/').last().unwrap_or("unknown").to_string());
            (final_name, 0u64)
        };
        
        // Build destination path based on TMDB metadata
        let download_dir = {
            self.config.read().await.download_dir.clone()
        };

        // Safety net for TV-series downloads: if metadata arrived without season/episode,
        // infer it from the real filename so episodes never collapse into one movie-style file.
        if category == "tv" {
            if let Some(ref mut meta) = tmdb_metadata {
                if meta.season.is_none() || meta.episode.is_none() {
                    let parsed = FilenameParser::parse(&filename);
                    if meta.season.is_none() { meta.season = parsed.season.map(|v| v as i32); }
                    if meta.episode.is_none() { meta.episode = parsed.episode.map(|v| v as i32); }
                }
                meta.media_type = Some("tv".to_string());
            }
        }
        
        // Preserve the original upstream filename on disk.
        // TMDB metadata is still used for routing/grouping/display, but the physical
        // file keeps codec/source/release-group details from FShare.
        let mut final_filename = filename.clone();
        if tmdb_metadata.is_some() {
            tracing::info!("Preserving original filename: {}", final_filename);
        }
        
        let inferred_series_folder = if tmdb_metadata.is_none() && folder_name.is_none() && category == "tv" {
            let parsed = FilenameParser::parse(&filename);
            if parsed.is_series && !parsed.title.trim().is_empty() {
                Some(parsed.title)
            } else {
                None
            }
        } else {
            None
        };
        let manual_folder_name = folder_name.or(inferred_series_folder);
        let selected_download_dir = download_folder
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        let routed_download_dir = if let Some(dir) = selected_download_dir.clone() {
            dir
        } else { match category.as_str() {
            "tv" | "show" | "shows" | "series" => std::env::var("FHUB_SHOWS_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| download_dir.join("Shows")),
            "movie" | "movies" => std::env::var("FHUB_MOVIES_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| download_dir.join("Movies")),
            _ => std::env::var("FHUB_MOVIES_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| download_dir.join("Movies")),
        }};

        // Build destination with FHub-compatible filename.
        // Manual series mode uses folder_name when TMDB metadata is not available,
        // so every episode added with the same show title lands in one folder.
        let mut destination = if tmdb_metadata.is_none() {
            if let Some(ref folder) = manual_folder_name {
                let clean_folder = PathBuilder::sanitize_filename(folder);
                if !clean_folder.is_empty() {
                    routed_download_dir.join(clean_folder).join(&final_filename).to_string_lossy().to_string()
                } else {
                    if selected_download_dir.is_some() {
                        routed_download_dir.join(&final_filename).to_string_lossy().to_string()
                    } else {
                        if selected_download_dir.is_some() {
                    routed_download_dir.join(&final_filename).to_string_lossy().to_string()
                } else {
                    if selected_download_dir.is_some() {
                routed_download_dir.join(&final_filename).to_string_lossy().to_string()
            } else {
                self.build_destination_path(&final_filename, &category, &tmdb_metadata, &download_dir)
            }
                }
                    }
                }
            } else {
                if selected_download_dir.is_some() {
                    routed_download_dir.join(&final_filename).to_string_lossy().to_string()
                } else {
                    if selected_download_dir.is_some() {
                routed_download_dir.join(&final_filename).to_string_lossy().to_string()
            } else {
                self.build_destination_path(&final_filename, &category, &tmdb_metadata, &download_dir)
            }
                }
            }
        } else {
            if selected_download_dir.is_some() {
                routed_download_dir.join(&final_filename).to_string_lossy().to_string()
            } else {
                self.build_destination_path(&final_filename, &category, &tmdb_metadata, &download_dir)
            }
        };

        // Duplicate guard by final destination. FShare folders may contain multiple encodes
        // or links for the same episode with different linkcodes; fshare_code-only duplicate
        // detection is not enough and can create 2x/3x queue entries for one episode.
        if let Some(existing) = self.task_manager.get_tasks().into_iter().find(|t| t.destination == destination) {
            if matches!(existing.state, DownloadState::Queued | DownloadState::Starting | DownloadState::Downloading | DownloadState::Waiting | DownloadState::Paused | DownloadState::Completed) {
                tracing::info!("Duplicate detected by destination in memory: {} already exists as {} in state {:?}, skipping", destination, existing.id, existing.state);
                return Err(anyhow::anyhow!("Download destination already exists ({:?})", existing.state));
            }
        }
        if let Some(db) = &self.db {
            match db.find_task_by_destination_async(&destination).await {
                Ok(Some((existing_id, existing_state))) => match existing_state.as_str() {
                    "QUEUED" | "STARTING" | "DOWNLOADING" | "WAITING" | "PAUSED" | "COMPLETED" => {
                        tracing::info!("Duplicate detected by destination in DB: {} already exists as {} in state {}, skipping", destination, existing_id, existing_state);
                        return Err(anyhow::anyhow!("Download destination already exists ({})", existing_state));
                    }
                    "FAILED" | "CANCELLED" => {
                        if let Ok(old_id) = uuid::Uuid::parse_str(&existing_id) {
                            let _ = db.delete_task(old_id);
                        }
                    }
                    _ => {}
                },
                Ok(None) => {}
                Err(e) => tracing::warn!("Failed to check duplicate destination: {}", e),
            }
        }

        // Final overwrite guard: never let a new download replace an existing file.
        // This protects TV episodes even if upstream metadata/filename parsing regresses.
        if std::path::Path::new(&destination).exists() {
            let original_path = std::path::Path::new(&destination);
            let parent = original_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| download_dir.clone());
            let stem = original_path.file_stem().and_then(|s| s.to_str()).unwrap_or("download");
            let ext = original_path.extension().and_then(|e| e.to_str()).unwrap_or("mkv");
            for idx in 2..=999 {
                let candidate_name = format!("{} ({idx}).{}", stem, ext);
                let candidate_path = parent.join(&candidate_name);
                if !candidate_path.exists() {
                    tracing::warn!("Destination already exists, avoiding overwrite: {} -> {}", destination, candidate_path.to_string_lossy());
                    final_filename = candidate_name;
                    destination = candidate_path.to_string_lossy().to_string();
                    break;
                }
            }
        }

        if let Some(ref folder) = manual_folder_name {
            if let Some(code) = &fshare_code {
                tracing::info!("[SERIES_MAP] fshare_code={} url={} folder={} filename={}", code, url, folder, final_filename);
            } else {
                tracing::info!("[SERIES_MAP] url={} folder={} filename={}", url, folder, final_filename);
            }
        }
        
        // Create new task with file size and current runtime segment configuration
        let current_config = self.config.read().await.clone();
        let mut task = DownloadTask::new(url, final_filename, host, category);
        task.segments = current_config.segments_per_download.clamp(1, 32) as u32;
        task.fshare_code = fshare_code;
        task.destination = destination;
        task.size = file_size;
        task.batch_id = batch_id;
        task.batch_name = batch_name;
        
        // ── Quality metadata ───────────────────────────────────────────────
        // IMPORTANT: parse from the ORIGINAL API filename (which contains tokens
        // like "1080p", "WEB-DL", "x265", "AAC") BEFORE it gets replaced by the
        // clean FHub-compatible name ("Series - S01E01.mkv" has no quality tags).
        let quality_attrs = FilenameParser::extract_quality_attributes(&filename);
        task.quality = Some(quality_attrs.quality_name());
        task.resolution = quality_attrs.resolution.clone();
        tracing::debug!(
            "[QUALITY] original='{}' final='{}' → quality={:?} resolution={:?}",
            filename, task.filename,
            task.quality, task.resolution
        );
        
        // Store TMDB metadata for FHub matching
        if let Some(ref meta) = tmdb_metadata {
            task.tmdb_id = meta.tmdb_id;
            task.tmdb_title = meta.title.clone();
            task.tmdb_season = meta.season.map(|s| s as u32);
            task.tmdb_episode = meta.episode.map(|e| e as u32);
            
            // ── Persist to media_items (TMDB-centric data model) ──────────
            if let (Some(tmdb_id), Some(db)) = (meta.tmdb_id, &self.db) {
                let media_type = meta.media_type.clone().unwrap_or_else(|| {
                    if meta.season.is_some() || meta.episode.is_some() { "tv".to_string() } else { "movie".to_string() }
                });
                let title = meta.title.clone().unwrap_or_else(|| "Unknown".to_string());
                let mut media_item = crate::db::media::MediaItem::new(tmdb_id, &media_type, &title);
                media_item.year = meta.year;
                
                // Check if item already exists to preserve existing arr state
                if let Ok(Some(existing)) = db.get_media_item(tmdb_id) {
                    media_item.arr_id = existing.arr_id;
                    media_item.arr_type = existing.arr_type;
                    media_item.arr_path = existing.arr_path;
                    media_item.arr_monitored = existing.arr_monitored;
                    media_item.arr_status = existing.arr_status;
                    media_item.arr_quality_profile_id = existing.arr_quality_profile_id;
                    media_item.arr_has_file = existing.arr_has_file;
                    media_item.arr_size_on_disk = existing.arr_size_on_disk;
                    media_item.tvdb_id = existing.tvdb_id;
                    media_item.imdb_id = existing.imdb_id;
                    media_item.arr_synced_at = existing.arr_synced_at;
                    media_item.created_at = existing.created_at;
                }
                
                if let Err(e) = db.upsert_media_item(&media_item) {
                    tracing::warn!("[ORCHESTRATOR] Failed to upsert media_item for tmdb_id={}: {}", tmdb_id, e);
                } else {
                    tracing::info!("[ORCHESTRATOR] Upserted media_item: tmdb_id={}, type={}, title={}", tmdb_id, media_type, title);
                }
                
                // For TV episodes, also upsert media_episodes row
                if media_type == "tv" {
                    if let (Some(season), Some(episode)) = (meta.season, meta.episode) {
                        let ep = crate::db::media::MediaEpisode::new(tmdb_id, season, episode);
                        if let Err(e) = db.upsert_media_episode(&ep) {
                            tracing::warn!("[ORCHESTRATOR] Failed to upsert media_episode for tmdb_id={} S{:02}E{:02}: {}", tmdb_id, season, episode, e);
                        }
                    }
                }
            }
        }
        
        // Add to manager (SSOT)
        self.task_manager.add_task(task.clone());
        
        // Wake idle workers
        self.task_notify.notify_waiters();
        
        // Persist to database
        if let Some(db) = &self.db {
            db.save_task(&task)?;
        }
        
        // Broadcast task added event (legacy)
        let _ = self.progress_tx.send(ProgressUpdate {
            event: TaskEvent::Added,
            task_id: task.id.to_string(),
            downloaded_bytes: 0,
            total_bytes: task.size,
            speed_bytes_per_sec: 0.0,
            eta_seconds: 0.0,
            percentage: 0.0,
            state: "QUEUED".to_string(),
        });
        
        // Emit TaskEvent::Created (event-driven architecture)
        self.event_bus.publish(super::events::TaskEvent::Created {
            task: task.clone(),
        });
        
        // Phase 1: Manage FHub Artifact (Series/Movie metadata)
        // This must never block add-to-queue. Take the read lock briefly, clone the
        // manager, then release the guard before any await-heavy/background logic.
        let artifact_manager_opt = {
            let guard = self.artifact_manager.read().await;
            guard.as_ref().map(Arc::clone)
        };
        tracing::debug!("Artifact management check: tmdb_id={:?}, artifact_manager_exists={}",
            task.tmdb_id, artifact_manager_opt.is_some());

        if task.tmdb_id.is_some() {
            if let Some(artifact_manager) = artifact_manager_opt {
                // Check if we should manage artifact based on batch_id
                let should_manage = if let Some(ref batch_id) = task.batch_id {
                    let mut announced = self.announced_batches.write().await;
                    if announced.contains(batch_id) {
                        tracing::debug!("Batch {} already managed - skipping artifact creation", batch_id);
                        false
                    } else {
                        announced.insert(batch_id.clone());
                        tracing::info!("First task in batch {} - will manage artifact in FHub", batch_id);
                        true
                    }
                } else {
                    tracing::debug!("No batch_id - managing artifact immediately");
                    true
                };

                tracing::info!("Should manage artifact: {} for task: {}", should_manage, task.filename);

                if should_manage {
                    tracing::info!("Managing artifact for: {} (TMDB ID: {:?})", task.filename, task.tmdb_id);

                    // Spawn artifact management in background to avoid blocking
                    let artifact_manager_clone = Arc::clone(&artifact_manager);
                    let task_clone = task.clone();
                    let task_manager_clone = Arc::clone(&self.task_manager);
                    let db_clone = self.db.clone();
                    tokio::spawn(async move {
                        match artifact_manager_clone.manage_artifact(&task_clone).await {
                            Ok(status) => {
                                use crate::arr::ArtifactStatus;
                                match status {
                                    ArtifactStatus::Created { arr_id } | 
                                    ArtifactStatus::AlreadyMonitored { arr_id } => {
                                        let status_name = match status {
                                            ArtifactStatus::Created { .. } => "created",
                                            ArtifactStatus::AlreadyMonitored { .. } => "found",
                                            _ => unreachable!(),
                                        };
                                        tracing::info!(
                                            "Successfully {} artifact for {} (FHub ID: {})",
                                            status_name,
                                            task_clone.filename,
                                            arr_id
                                        );
                                        
                                        // Update in-memory tasks with arr_series_id/arr_movie_id
                                        if let Some(tmdb_id) = task_clone.tmdb_id {
                                            let all_tasks = task_manager_clone.get_tasks();
                                            let mut updated_count = 0;
                                            
                                            for mut task in all_tasks {
                                                if task.tmdb_id == Some(tmdb_id) {
                                                    // Update appropriate ID based on media type
                                                    match task.detect_media_type() {
                                                        crate::downloader::MediaType::TvSeries | 
                                                        crate::downloader::MediaType::TvEpisode => {
                                                            task.arr_series_id = Some(arr_id as i64);
                                                        }
                                                        crate::downloader::MediaType::Movie => {
                                                            task.arr_movie_id = Some(arr_id as i64);
                                                        }
                                                    }
                                                    task_manager_clone.update_task(task);
                                                    updated_count += 1;
                                                }
                                            }
                                            
                                            tracing::info!("Updated {} in-memory tasks with arr_id={}", updated_count, arr_id);
                                            
                                            // Persist arr state to media_items table
                                            if let Some(ref db) = db_clone {
                                                let arr_type = match task_clone.detect_media_type() {
                                                    crate::downloader::MediaType::TvSeries |
                                                    crate::downloader::MediaType::TvEpisode => "fhub",
                                                    crate::downloader::MediaType::Movie => "fhub",
                                                };
                                                if let Err(e) = db.update_media_arr_state(
                                                    tmdb_id, arr_id, arr_type,
                                                    None, true, None, None,
                                                ) {
                                                    tracing::warn!("Failed to update media_items arr state for tmdb_id={}: {}", tmdb_id, e);
                                                } else {
                                                    tracing::info!("Updated media_items arr state: tmdb_id={}, arr_id={}, type={}", tmdb_id, arr_id, arr_type);
                                                }
                                            }
                                        }
                                    }
                                    ArtifactStatus::Skipped { reason } => {
                                        tracing::debug!(
                                            "Skipped artifact management for {}: {}",
                                            task_clone.filename,
                                            reason
                                        );
                                    }
                                    ArtifactStatus::Failed { error } => {
                                        tracing::warn!(
                                            "Failed to manage artifact for {}: {}",
                                            task_clone.filename,
                                            error
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Error managing artifact for {}: {}",
                                    task_clone.filename,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::debug!("Skipping artifact management for {} - already managed for this batch", task.filename);
                }
            }
        }
        
        tracing::info!("Added download: {} ({}) [code: {:?}] -> {}", task.filename, task.id, task.fshare_code, task.destination);
        Ok(task)
    }
    
    /// Build organized destination path based on TMDB metadata
    /// Delegates to PathBuilder module
    fn build_destination_path(&self, filename: &str, category: &str, tmdb: &Option<TmdbDownloadMetadata>, root_dir: &std::path::Path) -> String {
        PathBuilder::build_destination_path(filename, category, tmdb, root_dir)
    }
    
    /// Sanitize a string for use as a folder/file name
    /// Delegates to PathBuilder module
    #[allow(dead_code)] // May be used by future code
    fn sanitize_filename(name: &str) -> String {
        PathBuilder::sanitize_filename(name)
    }
    
    /// Extract Fshare file code from URL
    /// Delegates to DuplicateDetector module
    fn extract_fshare_code(url: &str) -> Option<String> {
        DuplicateDetector::extract_fshare_code(url)
    }
    
    /// Find task by Fshare code
    /// Delegates to DuplicateDetector module
    fn find_task_by_fshare_code(&self, code: &str) -> Option<DownloadTask> {
        DuplicateDetector::find_task_by_fshare_code(&self.task_manager, code)
    }
    
    /// Handle duplicate download based on existing task state
    async fn handle_duplicate(
        &self,
        existing: DownloadTask,
        url: String,
        filename: String,
        host: String,
        category: String,
        code: String,
    ) -> Result<DownloadTask, anyhow::Error> {
        use crate::downloader::DownloadState;
        
        match existing.state {
            // Active states - skip and return existing
            DownloadState::Queued | 
            DownloadState::Starting | 
            DownloadState::Downloading | 
            DownloadState::Paused | 
            DownloadState::Completed => {
                tracing::info!(
                    "Duplicate detected [code: {}]: Task {} already exists in state {:?}, skipping",
                    code, existing.id, existing.state
                );
                Ok(existing)
            }
            
            // Failed/Cancelled states - delete old and create new
            DownloadState::Failed | 
            DownloadState::Cancelled => {
                tracing::info!(
                    "Duplicate detected [code: {}]: Task {} is {:?}, deleting and creating new",
                    code, existing.id, existing.state
                );
                
                // Delete old task (this will also delete the file)
                self.task_manager.delete_task(existing.id);
                
                // Delete from database
                if let Some(db) = &self.db {
                    let _ = db.delete_task(existing.id);
                }
                
                // Create new task
                let mut task = DownloadTask::new(url, filename, host, category);
                task.fshare_code = Some(code);
                let download_dir = self.config.read().await.download_dir.clone();
                task.destination = download_dir.join(&task.filename).to_string_lossy().to_string();
                
                // Parse quality metadata from filename
                let quality_attrs = FilenameParser::extract_quality_attributes(&task.filename);
                task.quality = Some(quality_attrs.quality_name());
                task.resolution = quality_attrs.resolution.clone();
                
                // Add to manager
                self.task_manager.add_task(task.clone());
                
                // Persist to database
                if let Some(db) = &self.db {
                    db.save_task(&task)?;
                }
                
                // Broadcast task added event
                let _ = self.progress_tx.send(ProgressUpdate {
                    event: TaskEvent::Added,
                    task_id: task.id.to_string(),
                    downloaded_bytes: 0,
                    total_bytes: task.size,
                    speed_bytes_per_sec: 0.0,
                    eta_seconds: 0.0,
                    percentage: 0.0,
                    state: "QUEUED".to_string(),
                });
                
                tracing::info!("Created new download: {} ({})", task.filename, task.id);
                Ok(task)
            }
            
            // Other states - skip and return existing
            _ => {
                tracing::info!(
                    "Duplicate detected [code: {}]: Task {} in state {:?}, skipping",
                    code, existing.id, existing.state
                );
                Ok(existing)
            }
        }
    }
    
    /// Get task manager reference
    pub fn task_manager(&self) -> &Arc<DownloadTaskManager> {
        &self.task_manager
    }

    /// Get aggregate engine statistics (with database counts for filter dropdown)
    pub async fn get_stats(&self) -> EngineStats {
        let mut stats = self.task_manager.get_stats();
        
        // Query database for accurate status counts (for filter dropdown)
        if let Some(db) = &self.db {
            match db.get_status_counts() {
                Ok(counts) => {
                    use crate::downloader::stats::DbStatusCounts;
                    
                    // Aggregate counts by UI category
                    let downloading = *counts.get("DOWNLOADING").unwrap_or(&0) 
                        + *counts.get("STARTING").unwrap_or(&0);
                    let queued = *counts.get("QUEUED").unwrap_or(&0) 
                        + *counts.get("WAITING").unwrap_or(&0);
                    let paused = *counts.get("PAUSED").unwrap_or(&0);
                    let completed = *counts.get("COMPLETED").unwrap_or(&0);
                    let failed = *counts.get("FAILED").unwrap_or(&0);
                    let cancelled = *counts.get("CANCELLED").unwrap_or(&0);
                    let all: usize = counts.values().sum();
                    
                    stats.db_counts = Some(DbStatusCounts {
                        all,
                        downloading,
                        queued,
                        paused,
                        completed,
                        failed,
                        cancelled,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to get status counts from DB: {}", e);
                }
            }
        }
        
        stats
    }
    
    /// Subscribe to progress updates (legacy)
    pub fn subscribe_progress(&self) -> broadcast::Receiver<ProgressUpdate> {
        self.progress_tx.subscribe()
    }
    
    /// Subscribe to task events (event-driven architecture)
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<super::events::TaskEvent> {
        self.event_bus.subscribe()
    }
    
    /// Broadcast a task update to all WebSocket clients
    /// Used when task state changes (pause, resume, etc.)
    pub fn broadcast_task_update(&self, task: &DownloadTask) {
        let state_str = format!("{:?}", task.state).to_uppercase();
        
        // Legacy progress broadcast
        let _ = self.progress_tx.send(ProgressUpdate {
            event: TaskEvent::Updated,
            task_id: task.id.to_string(),
            downloaded_bytes: task.downloaded,
            total_bytes: task.size,
            speed_bytes_per_sec: task.speed,
            eta_seconds: task.eta,
            percentage: task.progress as f64,
            state: state_str,
        });
        
        // Emit StateChanged event (event-driven architecture)
        // Note: We don't have old_state here, so we'll use the current state for both
        // This is acceptable as the task object contains the new state
        self.event_bus.publish(super::events::TaskEvent::StateChanged {
            task: task.clone(),
            old_state: task.state.clone(),
            new_state: task.state.clone(),
            timestamp: chrono::Utc::now(),
        });
        
        tracing::debug!("Broadcast task update: {} -> {:?}", task.id, task.state);
    }
    
    /// Broadcast a task removal to all WebSocket clients
    /// Used when a task is deleted
    pub fn broadcast_task_removed(&self, task_id: &str) {
        let _ = self.progress_tx.send(ProgressUpdate {
            event: TaskEvent::Removed,
            task_id: task_id.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes_per_sec: 0.0,
            eta_seconds: 0.0,
            percentage: 0.0,
            state: "REMOVED".to_string(),
        });
        
        tracing::debug!("Broadcast task removed: {}", task_id);
    }
    
    /// Wake idle workers to process new tasks
    /// Call this when tasks are added to the queue
    pub fn wake_workers(&self) {
        self.task_notify.notify_waiters();
    }
    
    /// Get task from memory or database (unified lookup)
    /// 
    /// This method provides a unified way to retrieve tasks that works regardless
    /// of whether the task is currently in the TaskManager cache or only in the database.
    /// 
    /// **Lookup Strategy:**
    /// 1. Fast path: Check TaskManager (in-memory cache)
    /// 2. Slow path: Query Database if not in cache
    /// 
    /// This is critical for WebSocket broadcasts where tasks may exist in the DB
    /// but not in the active TaskManager (e.g., completed tasks, tasks from before restart).
    pub async fn get_task_unified(&self, task_id: uuid::Uuid) -> Option<DownloadTask> {
        // Fast path: check memory first
        if let Some(task) = self.task_manager.get_task(task_id) {
            tracing::trace!("Task {} found in TaskManager (cache hit)", task_id);
            return Some(task);
        }
        
        // Slow path: check database
        if let Some(db) = &self.db {
            tracing::trace!("Task {} not in TaskManager, querying database (cache miss)", task_id);
            match db.get_task_by_id(task_id) {
                Ok(task) => {
                    if task.is_some() {
                        tracing::debug!("Task {} found in database", task_id);
                    }
                    task
                }
                Err(e) => {
                    tracing::error!("Failed to query database for task {}: {}", task_id, e);
                    None
                }
            }
        } else {
            tracing::trace!("Task {} not found (no database configured)", task_id);
            None
        }
    }
    
    /// Load pending tasks (QUEUED, PAUSED) from database into TaskManager
    /// Call this on startup to ensure resume/pause operations work correctly
    pub async fn load_pending_tasks(&self) -> usize {
        let Some(db) = &self.db else {
            tracing::debug!("No database configured, skipping pending tasks load");
            return 0;
        };
        
        // Load resumable tasks into memory so workers and pause/resume controls can see them.
        // DOWNLOADING/STARTING are transient states - if server restarts during download,
        // those tasks should be re-queued, not restored with stale state.
        // WAITING is persistent retry state; it must be restored too, otherwise a task that
        // was waiting for retry during restart can be left stuck forever in DB/UI.
        let states = vec![
            "QUEUED".to_string(),
            "WAITING".to_string(),
            "PAUSED".to_string(),
        ];
        
        match db.get_tasks_by_states_async(states).await {
            Ok(tasks) => {
                let count = self.task_manager.restore_tasks(tasks);
                tracing::info!("Loaded {} pending tasks from database into TaskManager", count);
                
                // Wake workers to process any queued tasks
                if count > 0 {
                    self.wake_workers();
                }
                count
            }
            Err(e) => {
                tracing::error!("Failed to load pending tasks from database: {}", e);
                0
            }
        }
    }

    /// Validate if download should be allowed based on library state
    /// Returns Ok(()) if allowed, Err if duplicate/exists
    async fn validate_download_request(&self, tmdb: &Option<TmdbDownloadMetadata>) -> Result<(), anyhow::Error> {
        let Some(meta) = tmdb else {
            return Ok(()); // No TMDB metadata, can't validate against library
        };
        
        let client_guard = self.arr_client.read().await;
        let Some(client) = client_guard.as_ref() else {
            return Ok(()); // FHub integration disabled
        };
        
        let tmdb_id = meta.tmdb_id.unwrap_or(0);
        if tmdb_id == 0 { return Ok(()); }
        
        match meta.media_type.as_deref() {
            Some("movie") => {
                // Check FHub
                match client.get_movie_by_tmdb(tmdb_id).await {
                    Ok(Some(movie)) => {
                        if movie.has_file.unwrap_or(false) {
                            tracing::warn!("Rejecting download: Movie already exists in library (FHub ID: {})", movie.id);
                            return Err(anyhow::anyhow!("Movie already exists in library"));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::warn!("Failed to check FHub for validation: {}", e),
                }
            }
            Some("tv") | Some("episode") => {
                // Check FHub
                if let Some(season) = meta.season {
                    if let Some(episode) = meta.episode {
                        // 1. Resolve Series ID
                        match client.series_exists(tmdb_id).await {
                            Ok(Some(series_id)) => {
                                // 2. Check Episode
                                match client.get_episode_by_details(series_id, season as i32, episode as i32).await {
                                    Ok(Some(ep_info)) => {
                                        if ep_info.has_file {
                                            tracing::warn!("Rejecting download: Episode already exists in library (S{}E{})", season, episode);
                                            return Err(anyhow::anyhow!("Episode already exists in library"));
                                        }
                                    }
                                    Ok(None) => {} // Episode not monitored/found
                                    Err(e) => tracing::warn!("Failed to get episode details for validation: {}", e),
                                }
                            }
                            Ok(None) => {} // Series not in FHub
                            Err(e) => tracing::warn!("Failed to check FHub series for validation: {}", e),
                        }
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }


    
    // =========================================================================
    // Encapsulated Task Operations (DB + TaskManager + Broadcast)
    // =========================================================================
    
    /// Pause all pauseable tasks atomically
    /// Handles: DB update (atomic) → TaskManager update → Broadcast
    pub async fn pause_all_async(&self) -> usize {
        // Need db connection
        let db = match &self.db {
            Some(db) => db,
            None => {
                tracing::error!("No database connection for pause_all");
                return 0;
            }
        };
        
        let pauseable_states = vec![
            "QUEUED".to_string(),
            "DOWNLOADING".to_string(),
            "STARTING".to_string(),
            "WAITING".to_string(),
        ];
        
        // 1. Query pauseable tasks from DB
        let tasks = match db.get_tasks_by_states_async(pauseable_states).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to get pauseable tasks: {}", e);
                return 0;
            }
        };
        
        if tasks.is_empty() {
            return 0;
        }
        
        let task_ids: Vec<uuid::Uuid> = tasks.iter().map(|t| t.id).collect();
        
        // 2. Atomic batch update in DB (single transaction)
        if let Err(e) = db.batch_update_states_async(task_ids.clone(), "PAUSED".to_string()).await {
            tracing::error!("Failed to batch pause tasks: {}", e);
            return 0;
        }
        
        let affected = tasks.len();
        
        // 3. Update TaskManager and broadcast for each task
        for task in tasks {
            // Try to pause in TaskManager (for active downloads)
            if let Some(paused_task) = self.task_manager.pause_task(task.id) {
                self.broadcast_task_update(&paused_task);
            } else {
                // Task was only in DB (queued), add to TaskManager and broadcast
                let mut paused = task.clone();
                paused.state = DownloadState::Paused;
                self.task_manager.add_task(paused.clone());
                self.broadcast_task_update(&paused);
            }
        }
        
        tracing::info!("Paused {} tasks atomically", affected);
        affected
    }
    
    /// Resume all paused tasks atomically
    /// Handles: DB update (atomic) → TaskManager add → Broadcast → Wake workers
    pub async fn resume_all_async(&self) -> usize {
        // Need db connection
        let db = match &self.db {
            Some(db) => db,
            None => {
                tracing::error!("No database connection for resume_all");
                return 0;
            }
        };
        
        let resumable_states = vec!["PAUSED".to_string()];
        
        // 1. Query paused tasks from DB
        let tasks = match db.get_tasks_by_states_async(resumable_states).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("Failed to get paused tasks: {}", e);
                return 0;
            }
        };
        
        if tasks.is_empty() {
            return 0;
        }
        
        let task_ids: Vec<uuid::Uuid> = tasks.iter().map(|t| t.id).collect();
        
        // 2. Atomic batch update in DB (single transaction)
        if let Err(e) = db.batch_update_states_async(task_ids.clone(), "QUEUED".to_string()).await {
            tracing::error!("Failed to batch resume tasks: {}", e);
            return 0;
        }
        
        let affected = tasks.len();
        
        // 3. Add tasks to TaskManager and broadcast
        for task in tasks {
            let mut queued_task = task.clone();
            queued_task.state = DownloadState::Queued;
            
            // Add to TaskManager so workers can pick it up
            self.task_manager.add_task(queued_task.clone());
            
            // Broadcast the state change
            self.broadcast_task_update(&queued_task);
        }
        
        // 4. Wake workers to process new queued tasks
        self.wake_workers();
        
        tracing::info!("Resumed {} tasks atomically", affected);
        affected
    }
    
    /// Spawn a worker
    fn spawn_worker(&self, worker_id: usize) -> JoinHandle<()> {
        let running = self.running.clone();
        let task_manager = self.task_manager.clone();
        let download_engine = self.download_engine.clone();
        let host_registry = self.host_registry.clone();
        let db = self.db.clone();
        let progress_tx = self.progress_tx.clone();
        let event_bus = self.event_bus.clone();
        let config = self.config.clone();
        let task_notify = self.task_notify.clone();
        let arr_client = self.arr_client.clone();
        
        tokio::spawn(async move {
            tracing::debug!("Worker {} started", worker_id);
            
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                // Check if this worker should be active based on current config
                // This allows dynamic max_concurrent changes without restart
                {
                    let current_max = config.read().await.max_concurrent;
                    if worker_id >= current_max {
                        // This worker is over the limit, sleep and check again
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                }
                
                // Get next queued task
                if let Some(task) = task_manager.pop_next_queued() {
                    tracing::info!("Worker {}: Processing task {}", worker_id, task.id);
                    
                    // Acquire locks for current config and engine
                    let (current_engine, current_config) = {
                        let engine_guard = download_engine.read().await;
                        let config_guard = config.read().await;
                        (engine_guard.clone(), config_guard.clone())
                    };

                    // Process task
                    Self::process_task_static(
                        worker_id,
                        task,
                        &task_manager,
                        &current_engine,
                        &host_registry,
                        db.as_ref(),
                        &progress_tx,
                        &event_bus,
                        &current_config,
                        &arr_client,
                    ).await;
                } else {
                    // No tasks - wait for notification or timeout
                    // Use select! to allow cancellation via running flag check
                    tokio::select! {
                        _ = task_notify.notified() => {},
                        _ = tokio::time::sleep(Duration::from_secs(2)) => {},
                    }
                }
            }
            
            tracing::debug!("Worker {} stopped", worker_id);
        })
    }
    
    /// Process a single task (static method for worker)
    async fn process_task_static(
        worker_id: usize,
        mut task: DownloadTask,
        task_manager: &Arc<DownloadTaskManager>,
        download_engine: &Arc<SimpleDownloadEngine>,
        host_registry: &Arc<HostRegistry>,
        db: Option<&Arc<Db>>,
        progress_tx: &broadcast::Sender<ProgressUpdate>,
        event_bus: &Arc<super::events::EventBus>,
        config: &DownloadConfig,
        arr_client: &Arc<tokio::sync::RwLock<Option<Arc<crate::arr::FHubClient>>>>,
    ) {
        // Update state to Starting
        task.state = DownloadState::Starting;
        task.started_at = Some(Utc::now());
        task_manager.update_task(task.clone());
        
        // Publish StateChanged event (QUEUED -> STARTING)
        event_bus.publish(super::events::TaskEvent::StateChanged {
            task: task.clone(),
            old_state: DownloadState::Queued,
            new_state: DownloadState::Starting,
            timestamp: Utc::now(),
        });
        
        if let Some(db) = db {
            let _ = db.save_task(&task);
        }
        
        // Resolve URL using host handler
        let download_url = match host_registry.get_handler(&task.host) {
            Some(handler) => {
                tracing::info!("Worker {}: Resolving URL with {} handler", worker_id, task.host);
                match handler.resolve_download_url(&task.original_url).await {
                    Ok(resolved) => {
                        task.url = resolved.direct_url.clone();
                        task.url_metadata = Some(UrlMetadata {
                            resolved_at: Utc::now(),
                            expires_at: Utc::now() + chrono::Duration::hours(6),
                        });
                        task_manager.update_task(task.clone());
                        resolved.direct_url
                    }
                    Err(e) => {
                        tracing::error!("Worker {}: Failed to resolve URL: {}", worker_id, e);
                        task_manager.mark_failed(task.id, format!("URL resolution failed: {}", e));
                        
                        // Broadcast FAILED state to WebSocket
                        let _ = progress_tx.send(ProgressUpdate {
                            event: TaskEvent::Updated,
                            task_id: task.id.to_string(),
                            downloaded_bytes: 0,
                            total_bytes: task.size,
                            speed_bytes_per_sec: 0.0,
                            eta_seconds: 0.0,
                            percentage: 0.0,
                            state: "FAILED".to_string(),
                        });
                        
                        if let Some(db) = db {
                            if let Some(failed_task) = task_manager.get_task(task.id) {
                                let _ = db.save_task(&failed_task);
                            }
                        }
                        return;
                    }
                }
            }
            None => {
                tracing::warn!("Worker {}: No handler for host '{}', using original URL", worker_id, task.host);
                task.original_url.clone()
            }
        };
        
        // Update state to Downloading
        task.state = DownloadState::Downloading;
        task_manager.update_task(task.clone());
        
        // Download the file
        let destination = PathBuf::from(&task.destination);
        let task_id = task.id;
        let cancel_token = task.cancel_token.clone();
        
        // Clone for the progress callback closure (needs to be 'static)
        let task_manager_clone = Arc::clone(task_manager);
        let progress_tx_clone = progress_tx.clone();
        
        let result = download_engine.download_file(
            &download_url,
            &destination,
            move |progress| {
                // Update ALL progress fields in one atomic operation to prevent flickering
                task_manager_clone.update_task_progress(
                    task_id,
                    progress.downloaded_bytes,
                    progress.total_bytes,
                    progress.speed_bytes_per_sec,
                    progress.eta_seconds,
                    progress.percentage as f32,
                );
                
                // Broadcast progress (WebSocket handler will just read the task, not update again)
                let _ = progress_tx_clone.send(ProgressUpdate {
                    event: TaskEvent::Updated,
                    task_id: task_id.to_string(),
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                    speed_bytes_per_sec: progress.speed_bytes_per_sec,
                    eta_seconds: progress.eta_seconds,
                    percentage: progress.percentage,
                    state: "DOWNLOADING".to_string(),
                });
            },
            &cancel_token,
        ).await;
        
        // Handle result
        match result {
            Ok(()) => {
                tracing::info!("Worker {}: Download completed for {}", worker_id, task_id);
                task_manager.mark_completed(task_id);

                let mut completed_bytes = 0u64;
                let mut total_bytes = 0u64;
                
                // Post-completion: Move file to FHub's series/movie folder
                // This allows FHub's disk scan to detect and import the file
                if let Some(completed_task) = task_manager.get_task(task_id) {
                    completed_bytes = completed_task.downloaded;
                    total_bytes = completed_task.size;

                    let moved_task = Self::move_to_arr_path(
                        &completed_task,
                        arr_client,
                    ).await;
                    
                    // Update task with new destination if file was moved
                    if let Some(updated_task) = moved_task {
                        completed_bytes = updated_task.downloaded;
                        total_bytes = updated_task.size;
                        task_manager.update_task(updated_task.clone());
                        if let Some(db) = db {
                            let _ = db.save_task(&updated_task);
                        }
                    } else if let Some(db) = db {
                        let _ = db.save_task(&completed_task);
                    }
                }
                
                // Broadcast completion with final byte counts so UI does not stall with stale progress
                let _ = progress_tx.send(ProgressUpdate {
                    event: TaskEvent::Updated,
                    task_id: task_id.to_string(),
                    downloaded_bytes: completed_bytes,
                    total_bytes,
                    speed_bytes_per_sec: 0.0,
                    eta_seconds: 0.0,
                    percentage: 100.0,
                    state: "COMPLETED".to_string(),
                });
            }
            Err(e) => {
                let error_string = e.to_string();
                
                // Check if this was a pause action (task state is already Paused)
                // In this case, we should NOT retry or mark as failed
                if let Some(task) = task_manager.get_task(task_id) {
                    if task.state == DownloadState::Paused {
                        tracing::info!("Worker {}: Download paused for {} (by user request)", worker_id, task_id);
                        
                        // Broadcast paused state
                        let _ = progress_tx.send(ProgressUpdate {
                            event: TaskEvent::Updated,
                            task_id: task_id.to_string(),
                            downloaded_bytes: task.downloaded,
                            total_bytes: task.size,
                            speed_bytes_per_sec: 0.0,
                            eta_seconds: 0.0,
                            percentage: task.progress as f64,
                            state: "PAUSED".to_string(),
                        });
                        
                        // Save to database
                        if let Some(db) = db {
                            let _ = db.save_task(&task);
                        }
                        return; // Don't retry or mark as failed
                    }
                }
                
                // Check if user cancelled (not paused)
                if cancel_token.is_cancelled() {
                    // Check one more time if it was paused
                    if let Some(task) = task_manager.get_task(task_id) {
                        if task.state == DownloadState::Paused {
                            return; // Already handled above
                        }
                    }
                    
                    // True cancellation (delete/stop), not pause
                    task_manager.mark_failed(task_id, "Download cancelled".to_string());
                    
                    let _ = progress_tx.send(ProgressUpdate {
                        event: TaskEvent::Updated,
                        task_id: task_id.to_string(),
                        downloaded_bytes: 0,
                        total_bytes: 0,
                        speed_bytes_per_sec: 0.0,
                        eta_seconds: 0.0,
                        percentage: 0.0,
                        state: "FAILED".to_string(),
                    });
                    
                    if let Some(db) = db {
                        if let Some(updated_task) = task_manager.get_task(task_id) {
                            let _ = db.save_task(&updated_task);
                        }
                    }
                    return;
                }
                
                // ── Intelligent error classification ──────────────────────────
                // Use ErrorClassifier to determine the correct recovery strategy
                // instead of blindly retrying everything with the same config.
                use super::error_classifier::{ErrorCategory, ErrorClassifier};
                let category = ErrorClassifier::classify(&e);
                
                tracing::warn!(
                    "Worker {}: Download failed for {} — {:?} — {}",
                    worker_id, task_id, category, error_string
                );
                
                if let Some(mut task) = task_manager.get_task(task_id) {
                    match category {
                        // ── URL expired: refresh the VIP link and try again ──
                        ErrorCategory::UrlRefreshNeeded { max_retries, reason } => {
                            if task.retry_count < max_retries {
                                task.retry_count += 1;
                                task.state = DownloadState::Waiting;
                                // Clear the resolved URL so process_task_static re-resolves
                                // a fresh VIP link on the next attempt via resolve_download_url()
                                task.url = task.original_url.clone();
                                task.url_metadata = None;
                                // Short delay — just enough to avoid hammering the API
                                task.wait_until = Some(Utc::now() + chrono::Duration::seconds(3));
                                task.error_message = Some(format!(
                                    "Link expired ({}/{}): {reason} — getting fresh URL",
                                    task.retry_count, max_retries
                                ));
                                task_manager.update_task(task.clone());
                                tracing::info!(
                                    "Worker {}: URL refresh needed for {} — will re-resolve in 3s (attempt {}/{})",
                                    worker_id, task_id, task.retry_count, max_retries
                                );
                            } else {
                                let msg = format!("Failed after {max_retries} URL refresh attempts: {reason}");
                                tracing::error!("Worker {}: {}", worker_id, msg);
                                Self::fail_task(worker_id, task_id, msg, &task_manager, progress_tx, db).await;
                            }
                        }
                        
                        // ── Retryable: network blip, server busy — retry with delay ──
                        ErrorCategory::Retryable { max_retries, delay_seconds, reason } => {
                            if task.retry_count < max_retries {
                                task.retry_count += 1;
                                task.state = DownloadState::Waiting;
                                // Use classifier's delay, bounded by config max
                                let delay_ms = (delay_seconds * 1000)
                                    .min(config.retry.max_delay_ms as u64);
                                task.wait_until = Some(
                                    Utc::now() + chrono::Duration::milliseconds(delay_ms as i64)
                                );
                                task.error_message = Some(format!(
                                    "Retry {}/{}: {reason}", task.retry_count, max_retries
                                ));
                                task_manager.update_task(task.clone());
                                tracing::warn!(
                                    "Worker {}: Retryable error for {} in {}ms (attempt {}/{}): {}",
                                    worker_id, task_id, delay_ms, task.retry_count, max_retries, reason
                                );
                            } else {
                                let msg = format!("Max retries ({max_retries}) exceeded: {reason}");
                                Self::fail_task(worker_id, task_id, msg, &task_manager, progress_tx, db).await;
                            }
                        }
                        
                        // ── Permanent: no point retrying ──────────────────────
                        ErrorCategory::Permanent { reason } => {
                            let msg = format!("Permanent failure: {reason}");
                            tracing::error!("Worker {}: {} (task {})", worker_id, msg, task_id);
                            Self::fail_task(worker_id, task_id, msg, &task_manager, progress_tx, db).await;
                        }
                        
                        // ── Account issue: needs user action ──────────────────
                        ErrorCategory::AccountIssue { reason, action_required } => {
                            let msg = format!("Account issue: {reason} — {action_required}");
                            tracing::error!("Worker {}: {} (task {})", worker_id, msg, task_id);
                            Self::fail_task(worker_id, task_id, msg, &task_manager, progress_tx, db).await;
                        }
                        
                        // ── System/config issue: retry a few times ────────────
                        ErrorCategory::SystemIssue { max_retries, reason, fix_suggestion } => {
                            if task.retry_count < max_retries {
                                task.retry_count += 1;
                                task.state = DownloadState::Waiting;
                                task.wait_until = Some(Utc::now() + chrono::Duration::seconds(10));
                                task.error_message = Some(format!(
                                    "System issue ({}/{}): {reason} — Tip: {fix_suggestion}",
                                    task.retry_count, max_retries
                                ));
                                task_manager.update_task(task.clone());
                                tracing::warn!(
                                    "Worker {}: System issue for {} (attempt {}/{}): {} — {}",
                                    worker_id, task_id, task.retry_count, max_retries, reason, fix_suggestion
                                );
                            } else {
                                let msg = format!("{reason} — {fix_suggestion}");
                                Self::fail_task(worker_id, task_id, msg, &task_manager, progress_tx, db).await;
                            }
                        }
                    }
                    
                    if let Some(db) = db {
                        if let Some(updated_task) = task_manager.get_task(task_id) {
                            let _ = db.save_task(&updated_task);
                        }
                    }
                }
            }
        }
    }

    
    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay_static(retry_count: u32, config: &DownloadConfig) -> Duration {
        let base_delay = config.retry.base_delay_ms as u64;
        let max_delay = config.retry.max_delay_ms as u64;
        let delay = base_delay * 2u64.pow(retry_count.saturating_sub(1));
        Duration::from_millis(delay.min(max_delay))
    }

    /// Mark a task as failed and broadcast the state over WebSocket.
    /// DRY helper shared by all error category handlers in process_task_static.
    async fn fail_task(
        worker_id: usize,
        task_id: uuid::Uuid,
        message: String,
        task_manager: &Arc<DownloadTaskManager>,
        progress_tx: &broadcast::Sender<ProgressUpdate>,
        db: Option<&Arc<Db>>,
    ) {
        tracing::error!("Worker {}: Permanently failing task {}: {}", worker_id, task_id, message);
        task_manager.mark_failed(task_id, message);

        let _ = progress_tx.send(ProgressUpdate {
            event: TaskEvent::Updated,
            task_id: task_id.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            speed_bytes_per_sec: 0.0,
            eta_seconds: 0.0,
            percentage: 0.0,
            state: "FAILED".to_string(),
        });

        if let Some(db) = db {
            if let Some(failed_task) = task_manager.get_task(task_id) {
                let _ = db.save_task(&failed_task);
            }
        }
    }

    
    /// Restore tasks from database
    async fn restore_from_db(&self) -> anyhow::Result<()> {
        if let Some(db) = &self.db {
            tracing::info!("Restoring tasks from database...");
            
            let tasks = db.get_all_tasks()?;
            let mut restored_count = 0;
            
            for task in tasks {
                match task.state {
                    DownloadState::Downloading | DownloadState::Starting => {
                        // Resume interrupted downloads as QUEUED
                        let mut resumed_task = task.clone();
                        resumed_task.state = DownloadState::Queued;
                        self.task_manager.add_task(resumed_task);
                        restored_count += 1;
                    }
                    DownloadState::Queued | DownloadState::Waiting => {
                        // Re-queue pending downloads
                        self.task_manager.add_task(task);
                        restored_count += 1;
                    }
                    _ => {
                        // Keep other states as-is
                        self.task_manager.add_task(task);
                        restored_count += 1;
                    }
                }
            }
            
            tracing::info!("Restored {} tasks from database", restored_count);
        }
        
        Ok(())
    }

    /// Get current download configuration
    pub async fn get_config(&self) -> DownloadConfig {
        self.config.read().await.clone()
    }
    
    /// Update download configuration
    pub async fn update_config(&self, new_config: DownloadConfig) {
        let old_max_concurrent = {
            let config_guard = self.config.read().await;
            config_guard.max_concurrent
        };
        
        // Update config
        {
            let mut config_guard = self.config.write().await;
            *config_guard = new_config.clone();
        }
        
        // Recreation of engine to pick up new settings
        {
            let new_engine = Arc::new(SimpleDownloadEngine::with_config(new_config.clone()));
            let mut engine_guard = self.download_engine.write().await;
            *engine_guard = new_engine;
        }
        
        // Dynamic worker pool resizing
        let new_max_concurrent = new_config.max_concurrent;
        if new_max_concurrent != old_max_concurrent {
            let mut workers = self.workers.lock().await;
            
            if new_max_concurrent > old_max_concurrent {
                // Spawn additional workers
                let workers_to_add = new_max_concurrent - old_max_concurrent;
                tracing::info!("Spawning {} additional workers ({}→{})", 
                    workers_to_add, old_max_concurrent, new_max_concurrent);
                
                for worker_id in old_max_concurrent..new_max_concurrent {
                    let handle = self.spawn_worker(worker_id);
                    workers.push(handle);
                }
            } else {
                // Workers will automatically sleep when worker_id >= max_concurrent
                // No need to kill them - they'll just idle
                tracing::info!("Reduced max_concurrent ({}→{}). Excess workers will idle.", 
                    old_max_concurrent, new_max_concurrent);
            }
        }
        
        tracing::info!("Updated download configuration: max_concurrent={}, segments={}", 
            new_config.max_concurrent, new_config.segments_per_download);
    }

    
    /// Move completed download to FHub's series/movie folder
    /// Returns updated task if file was moved successfully, None otherwise
    async fn move_to_arr_path(
        task: &DownloadTask,
        arr_client: &Arc<tokio::sync::RwLock<Option<Arc<crate::arr::FHubClient>>>>,
    ) -> Option<DownloadTask> {
        use crate::downloader::MediaType;
        
        // Get arr_client (may not be configured)
        let client = {
            let guard = arr_client.read().await;
            guard.clone()?
        };
        
        let media_type = task.detect_media_type();
        
        // Get the target folder path from FHub
        // Always query by tmdb_id — no dependency on cached arr_series_id
        let (arr_folder, arr_id) = match media_type {
            MediaType::TvSeries | MediaType::TvEpisode => {
                let series_id = match task.arr_series_id {
                    Some(id) => id,
                    None => {
                        let tmdb_id = task.tmdb_id?;
                        tracing::info!("Looking up series by TMDB ID: {}", tmdb_id);
                        match client.series_exists(tmdb_id).await {
                            Ok(Some(id)) => {
                                tracing::info!("Found series in FHub by TMDB {}: ID {}", tmdb_id, id);
                                id as i64
                            }
                            Ok(None) => {
                                tracing::warn!("Series not found in FHub for TMDB {}", tmdb_id);
                                return None;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to look up series by TMDB {}: {}", tmdb_id, e);
                                return None;
                            }
                        }
                    }
                };
                let path = match client.get_series_path(series_id).await {
                    Ok(path) => path,
                    Err(e) => {
                        tracing::warn!("Failed to get series path for ID {}: {}", series_id, e);
                        return None;
                    }
                };
                (path, series_id)
            }
            MediaType::Movie => {
                let movie_id = match task.arr_movie_id {
                    Some(id) => id,
                    None => {
                        let tmdb_id = task.tmdb_id?;
                        tracing::info!("Looking up movie by TMDB ID: {}", tmdb_id);
                        match client.movie_exists(tmdb_id).await {
                            Ok(Some(id)) => {
                                tracing::info!("Found movie in FHub by TMDB {}: ID {}", tmdb_id, id);
                                id as i64
                            }
                            Ok(None) => {
                                tracing::warn!("Movie not found in FHub for TMDB {}", tmdb_id);
                                return None;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to look up movie by TMDB {}: {}", tmdb_id, e);
                                return None;
                            }
                        }
                    }
                };
                let path = match client.get_movie_path(movie_id).await {
                    Ok(path) => path,
                    Err(e) => {
                        tracing::warn!("Failed to get movie path for ID {}: {}", movie_id, e);
                        return None;
                    }
                };
                (path, movie_id)
            }
        };
        
        // Build target path
        let source = std::path::Path::new(&task.destination);
        let filename = source.file_name()?;
        
        let target_dir = match media_type {
            MediaType::TvSeries | MediaType::TvEpisode => {
                // TV: {series_path}/Season XX/
                let season = task.tmdb_season.unwrap_or(1);
                std::path::PathBuf::from(&arr_folder).join(format!("Season {:02}", season))
            }
            MediaType::Movie => {
                // Movie: {movie_path}/
                std::path::PathBuf::from(&arr_folder)
            }
        };
        
        let target_path = target_dir.join(filename);
        
        // Don't move if source and target are the same
        if source == target_path {
            tracing::debug!("File already at target path: {:?}", target_path);
            return None;
        }
        
        // Create target directory
        if let Err(e) = tokio::fs::create_dir_all(&target_dir).await {
            tracing::error!("Failed to create target directory {:?}: {}", target_dir, e);
            return None;
        }
        
        // Move (rename) file to target
        tracing::info!(
            "Moving completed file to arr path: {:?} -> {:?}",
            source, target_path
        );
        
        // Move file and trigger FHub rescan on success
        let move_result = match tokio::fs::rename(&source, &target_path).await {
            Ok(()) => {
                tracing::info!("Successfully moved file to {:?}", target_path);
                let mut updated = task.clone();
                updated.destination = target_path.to_string_lossy().to_string();
                Some(updated)
            }
            Err(e) => {
                // rename fails across filesystems, fall back to copy+delete
                tracing::warn!("rename failed (cross-device?): {}, trying copy+delete", e);
                match tokio::fs::copy(&source, &target_path).await {
                    Ok(_) => {
                        let _ = tokio::fs::remove_file(&source).await;
                        tracing::info!("Successfully copied file to {:?}", target_path);
                        let mut updated = task.clone();
                        updated.destination = target_path.to_string_lossy().to_string();
                        Some(updated)
                    }
                    Err(copy_err) => {
                        tracing::error!("Failed to copy file to {:?}: {}", target_path, copy_err);
                        None
                    }
                }
            }
        };
        
        // Trigger FHub rescan for instant import (fire-and-forget)
        if move_result.is_some() {
            let rescan_result = match media_type {
                MediaType::TvSeries | MediaType::TvEpisode => {
                    client.trigger_series_rescan(arr_id).await
                }
                MediaType::Movie => {
                    client.trigger_movie_refresh(arr_id).await
                }
            };
            if let Err(e) = rescan_result {
                tracing::warn!("FHub rescan trigger failed (non-fatal): {}", e);
            }
        }
        
        move_result
    }

    /// Reload NAS client with new configuration (dynamic update without restart)
    pub async fn reload_arr_client(
        &self,
        tv_config: Option<crate::config::FHubConfig>,
        movie_config: Option<crate::config::FHubConfig>,
    ) {
        let _ = (tv_config, movie_config);
        let new_client = None;
        
        // Create new artifact manager if client exists and db is available
        let new_artifact_manager = if let (Some(ref client), Some(ref database)) = (&new_client, &self.db) {
            Some(Arc::new(crate::arr::FHubArtifactManager::new(
                Arc::clone(client),
                Arc::clone(database),
            )))
        } else {
            None
        };

        {
            let mut arr_guard = self.arr_client.write().await;
            *arr_guard = new_client;
        }
        
        {
            let mut artifact_guard = self.artifact_manager.write().await;
            *artifact_guard = new_artifact_manager;
        }
        
        tracing::info!("Reloaded standalone FHub integration state (external library integration disabled)");
    }

    /// Get access to the arr client for API proxying
    pub async fn get_arr_client(&self) -> Option<Arc<crate::arr::FHubClient>> {
        let guard = self.arr_client.read().await;
        guard.clone()
    }
    

}
