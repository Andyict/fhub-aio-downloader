use axum::{
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
    Json,
};
use serde::Serialize;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use figment::providers::Format;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer, Any};
use moka::future::Cache;
use std::time::Duration;

mod api;
mod downloader;
mod hosts;
mod websocket;
mod db;
mod config;
mod utils;
mod arr;
mod constants;
mod error;
mod services;
mod fhub_source;


use std::sync::Arc;

/// Cached FHUB metadata enrichment stored by tmdb_id.
/// Holds everything the source pipeline needs without re-calling metadata APIs.
#[derive(Clone)]
pub struct TmdbEnrichmentCache {
    /// Official display title
    pub official: Option<String>,
    /// Original title in content's native language
    pub original_name: Option<String>,
    /// All aliases merged for similarity scoring
    pub all_aliases: Vec<String>,
    /// Vietnamese titles searched directly as priority source candidates
    pub vn_titles: Vec<String>,
    /// Titles in the content's original language
    pub original_lang_titles: Vec<String>,
    /// US/English alternative titles that differ from official name
    pub us_titles: Vec<String>,
    /// Poster path
    pub poster: Option<String>,
    /// Movie collection parts: title, year, tmdb_id, poster
    pub collections: Vec<(String, String, u64, Option<String>)>,
}

#[derive(Clone)]
pub struct AppState {
    pub host_registry: Arc<hosts::registry::HostRegistry>,
    pub download_orchestrator: Arc<downloader::DownloadOrchestrator>,
    pub download_service: Arc<services::DownloadService>,
    pub tmdb_service: Arc<services::TmdbService>,
    pub folder_cache_service: Arc<services::FolderCacheService>,
    pub tx_broadcast: tokio::sync::broadcast::Sender<downloader::task::DownloadTask>,
    pub config: config::Config,
    pub db: Arc<db::Db>,
    pub search_cache: Cache<String, api::smart_search::SmartSearchResponse>,
    pub tmdb_cache: Cache<String, TmdbEnrichmentCache>,
    /// Shared HTTP client for FHUB source and metadata endpoints.
    pub http_client: Arc<reqwest::Client>,
    /// Cache for source search results to avoid duplicate upstream calls.
    pub fshare_search_cache: Cache<String, Vec<api::search_pipeline::RawFshareResult>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}


async fn spa_index() -> impl IntoResponse {
    axum::response::Html(tokio::fs::read_to_string("static/index.html").await.unwrap_or_else(|_| "FHUB static index not found".to_string()))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "fhub",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "fhub=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting FHUB v{}", env!("CARGO_PKG_VERSION"));

    // Create FHUB appData directory structure if needed.
    if let Err(e) = config::ensure_appdata_dirs() {
        tracing::warn!("FHUB could not create appData directories: {}. Continuing with compatibility paths.", e);
    }

    // Get paths with compatibility fallback to older locations.
    let config_path = config::get_config_path();
    let db_path = config::get_db_path();
    
    tracing::info!("Loading FHUB config from: {}", config_path.display());
    tracing::info!("Using FHUB database at: {}", db_path.display());

    // Load FHUB config.
    let mut config: config::Config = figment::Figment::new()
        .merge(figment::providers::Serialized::defaults(config::Config::default()))
        .merge(figment::providers::Toml::file(config_path))
        .merge(figment::providers::Env::prefixed("FHUB_"))
        .extract()
        .expect("FHUB failed to load configuration");

    // Compatibility: older FHUB defaults used /downloads, but the Docker image only
    // guarantees /appData/downloads unless an external mount is explicitly supplied.
    // If /downloads is not mounted, silently heal runtime config to the writable appData path.
    if config.downloads.directory == std::path::PathBuf::from("/downloads")
        && !config.downloads.directory.exists()
    {
        let fallback_downloads = config::get_appdata_dir().join("downloads");
        tracing::warn!(
            "Configured download directory /downloads does not exist; using {} instead",
            fallback_downloads.display()
        );
        config.downloads.directory = fallback_downloads;
    }

    // Initialize FHUB database.
    let db = Arc::new(db::Db::new(&db_path).expect("FHUB failed to initialize database"));
    
    // Ensure indexer API key exists.
    if db.get_setting("indexer_api_key").ok().flatten().is_none() {
        let api_key = format!("fhub_{}", uuid::Uuid::new_v4().simple());
        if let Err(e) = db.save_setting("indexer_api_key", &api_key) {
            tracing::warn!("FHUB failed to save generated API key: {}", e);
        } else {
            tracing::info!("Generated new FHUB indexer API key: {}", api_key);
        }
    }

    // Initialize FHUB runtime components.
    let (tx_broadcast, _) = tokio::sync::broadcast::channel(100);
    let shared_http_client = hosts::create_shared_client();
    let host_registry = Arc::new(hosts::create_registry(&config, shared_http_client, Arc::clone(&db)));
    
    // Create activity config from app config.
    let download_config = downloader::config::DownloadConfig {
        max_concurrent: config.downloads.max_concurrent,
        segments_per_download: config.downloads.segments_per_download as usize,
        download_dir: config.downloads.directory.clone(),
        chunk_size: 1024 * 1024,
        retry_attempts: 3,
        retry_backoff_base: 30,
        retry_max_wait: 300,
        retry: downloader::config::RetryConfig::default(),
    };
    let tv_config = None;
    let movie_config = None;

    // Create FHUB activity orchestrator.
    let download_orchestrator = Arc::new(downloader::DownloadOrchestrator::new(
        download_config,
        Arc::clone(&host_registry),
        Some(Arc::clone(&db)),
        tv_config.clone(),
        movie_config.clone(),
    ));
    
    // Start orchestrator workers.
    download_orchestrator.start().await;
    tracing::info!("FHUB activity orchestrator started");
    
    // Load pending activities from database into TaskManager.
    let pending_count = download_orchestrator.load_pending_tasks().await;
    tracing::info!("Loaded {} pending FHUB activities from database", pending_count);

    
    // Initialize caches.
    let search_cache = Cache::builder()
        .max_capacity(100)
        .time_to_live(Duration::from_secs(600))
        .build();

    let tmdb_cache = Cache::builder()
        .max_capacity(500)
        .time_to_live(Duration::from_secs(86400))
        .build();

    // Shared HTTP client for source and metadata endpoints.
    let http_client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(90))
            .user_agent("FHUB/1.0")
            .build()
            .unwrap_or_default()
    );

    // Source response cache.
    let fshare_search_cache = Cache::builder()
        .max_capacity(200)
        .time_to_live(Duration::from_secs(300))
        .build();
    
    // Create DownloadService for business logic abstraction.
    let download_service = Arc::new(services::DownloadService::new(
        Arc::clone(&db),
        Arc::clone(&download_orchestrator),
    ));
    
    // Create metadata service for centralized API access.
    let tmdb_service = Arc::new(services::TmdbService::new_with_default_client());
    
    // Create FolderCacheService for source caching.
    let folder_cache_service = Arc::new(services::FolderCacheService::new(Arc::clone(&db), Arc::clone(&tmdb_service)));
    
    let app_config = config.clone();

    let state = Arc::new(AppState { 
        host_registry,
        download_orchestrator,
        download_service,
        tmdb_service,
        folder_cache_service: Arc::clone(&folder_cache_service),
        tx_broadcast,
        config: app_config,
        db,
        search_cache,
        tmdb_cache,
        http_client,
        fshare_search_cache,
    });

    api::auth::seed_admin_if_needed(&state);

    let admin_api = Router::new()
        .nest("/api/auth", api::auth::admin_router())
        .nest("/api/settings", api::settings::router())
        .layer(middleware::from_fn_with_state(Arc::clone(&state), api::auth::admin_required));

    let protected_api = Router::new()
        .route("/api/ws", get(websocket::handler))
        .nest("/api/auth", api::auth::protected_router())
        .nest("/api/downloads", api::downloads::router())
        .nest("/api/stats", api::stats::router())
        .nest("/api/engine/stats", api::stats::router())
        .nest("/api/arr", api::arr::router())
        .nest("/api/system", api::system::router())
        .nest("/api/search", api::search::router())
        .nest("/api/accounts", api::accounts::router())
        .nest("/sabnzbd", api::sabnzbd::router())
        .nest("/api/indexer", api::indexer::router())
        .nest("/newznab/api", api::indexer::router())
        .nest("/api/media", api::media::router())
        .nest("/api/folder-source", api::folder_source::router())
        .layer(middleware::from_fn_with_state(Arc::clone(&state), api::auth::auth_required));

    // Build router.
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/health/status", get(api::health::health_status))
        .nest("/api/auth", api::auth::public_router())
        .nest("/api/setup", api::setup::router())
        .nest("/api/tmdb", api::tmdb::router())
        .nest("/api/discovery", api::discovery::router())
        .merge(admin_api)
        .merge(protected_api)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .fallback_service(ServeDir::new("static").fallback(get(spa_index)))
        .with_state(state);

    // Spawn folder cache background sync.
    tokio::spawn(async move {
        // Initial sync on startup if cache is stale or empty.
        folder_cache_service.sync_if_stale().await;
        
        // Then sync every 24 hours.
        let mut interval = tokio::time::interval(Duration::from_secs(86400));
        interval.tick().await;
        loop {
            interval.tick().await;
            tracing::info!("[FHUB-SOURCE-CACHE] Starting scheduled daily sync");
            match folder_cache_service.sync_all_sources().await {
                Ok(report) => {
                    tracing::info!(
                        "[FHUB-SOURCE-CACHE] Daily sync complete: {} items from {} sources in {:.1}s",
                        report.total_items, report.total_sources, report.duration_secs
                    );
                }
                Err(e) => {
                    tracing::error!("[FHUB-SOURCE-CACHE] Daily sync failed: {}", e);
                }
            }
        }
    });

    // Run FHUB server.
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("FHUB listening on {}", addr);
    
    // Create socket with SO_REUSEADDR to allow immediate restart.
    use socket2::{Socket, Domain, Type};
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)
        .expect("FHUB failed to create socket");
    socket.set_reuse_address(true)
        .expect("FHUB failed to set SO_REUSEADDR");
    socket.bind(&addr.into())
        .expect("FHUB failed to bind socket");
    socket.listen(1024)
        .expect("FHUB failed to listen on socket");
    
    // Set non-blocking mode before converting to tokio.
    socket.set_nonblocking(true)
        .expect("FHUB failed to set non-blocking mode");
    
    // Convert to tokio listener.
    let listener = tokio::net::TcpListener::from_std(socket.into())
        .expect("FHUB failed to convert to tokio listener");
    
    axum::serve(listener, app).await.unwrap();
}
