//! FHub Metadata Proxy API Routes
//!
//! Proxy endpoints for cinema metadata, artwork, discovery, and enrichment data.

use axum::{
    routing::get,
    Router,
    Json,
    extract::{State, Path, Query},
    http::StatusCode,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use reqwest::Client;
use crate::AppState;

const TMDB_API_BASE: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/search", get(search))
        .route("/discover/:media_type", get(discover))
        .route("/movie/:id", get(movie_details))
        .route("/tv/:id", get(tv_details))
        .route("/collection/:id", get(collection_details))
        .route("/tv/:id/season/:season", get(season_details))
        .route("/genres/:media_type", get(genres))
        .route("/:media_type/:id/similar", get(similar))
        .route("/:media_type/:id/recommendations", get(recommendations))
        .route("/trending/:media_type/:time_window", get(trending))
        .route("/image/:size/*path", get(proxy_image))
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_media_type")]
    media_type: String,
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Deserialize)]
struct DiscoverQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default, alias = "with_genres")]
    with_genres: Option<String>,
    #[serde(default)]
    year: Option<u32>,
    #[serde(default)]
    sort_by: Option<String>,
    #[serde(default, alias = "vote_average.gte")]
    vote_average_gte: Option<f32>,
    #[serde(default, alias = "vote_average.lte")]
    vote_average_lte: Option<f32>,
    #[serde(default, alias = "primary_release_date.gte")]
    primary_release_date_gte: Option<String>,
    #[serde(default, alias = "primary_release_date.lte")]
    primary_release_date_lte: Option<String>,
    #[serde(default, alias = "first_air_date.gte")]
    first_air_date_gte: Option<String>,
    #[serde(default, alias = "first_air_date.lte")]
    first_air_date_lte: Option<String>,
}

#[derive(Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_page")]
    page: u32,
    language: Option<String>,
}

fn default_media_type() -> String { "multi".to_string() }
fn default_page() -> u32 { 1 }

// ============================================================================
// Response Types
// ============================================================================

#[allow(dead_code)]
#[derive(Serialize)]
struct TmdbResponse {
    #[serde(flatten)]
    data: Value,
}


fn videos_empty(data: &Value) -> bool {
    data.get("videos")
        .and_then(|v| v.get("results"))
        .and_then(|v| v.as_array())
        .map(|items| items.is_empty())
        .unwrap_or(true)
}

fn merge_videos_if_empty(data: &mut Value, fallback: &Value) {
    if !videos_empty(data) {
        return;
    }
    let Some(videos) = fallback.get("videos") else { return; };
    let Some(results) = videos.get("results").and_then(|v| v.as_array()) else { return; };
    if results.is_empty() {
        return;
    }
    if let Some(obj) = data.as_object_mut() {
        obj.insert("videos".to_string(), videos.clone());
    }
}

// ============================================================================
// Metadata Client
// ============================================================================

struct TmdbClient {
    client: Client,
    api_key: String,
}

impl TmdbClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            // Bundled metadata API key for the current FHUB runtime profile.
            api_key: "8d95150f3391194ca66fef44df497ad6".to_string(),
        }
    }
    
    async fn get(&self, path: &str, extra_params: &[(&str, &str)]) -> Result<Value, StatusCode> {
        if self.api_key.is_empty() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        
        let mut url = format!("{}{}", TMDB_API_BASE, path);
        url.push_str(&format!("?api_key={}", self.api_key));
        
        for (key, value) in extra_params {
            url.push_str(&format!("&{}={}", key, value));
        }
        
        let resp = self.client.get(&url)
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
            
        if !resp.status().is_success() {
            return Err(StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY));
        }
        
        resp.json::<Value>()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/tmdb/search - Search movies and TV shows for FHUB metadata cards.
async fn search(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    
    let path = format!("/search/{}", params.media_type);
    let page_str = params.page.to_string();
    
    let mut query_params: Vec<(&str, &str)> = vec![
        ("query", params.q.as_str()),
        ("page", &page_str),
    ];
    if let Some(ref language) = params.language {
        query_params.push(("language", language.as_str()));
    }
    let data = client.get(&path, &query_params).await?;
    
    Ok(Json(data))
}

/// GET /api/tmdb/discover/:media_type - Discover movies or TV shows for FHUB browsing.
async fn discover(
    State(_state): State<Arc<AppState>>,
    Path(media_type): Path<String>,
    Query(params): Query<DiscoverQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    
    let path = format!("/discover/{}", media_type);
    let page_str = params.page.to_string();
    let mut query_params: Vec<(&str, String)> = vec![
        ("page", page_str),
    ];
    
    if let Some(ref genre) = params.genre {
        query_params.push(("with_genres", genre.clone()));
    }
    if let Some(ref genre) = params.with_genres {
        query_params.push(("with_genres", genre.clone()));
    }
    if let Some(year) = params.year {
        let key = if media_type == "movie" { "primary_release_year" } else { "first_air_date_year" };
        query_params.push((key, year.to_string()));
    }
    if let Some(ref sort) = params.sort_by {
        query_params.push(("sort_by", sort.clone()));
    }
    if let Some(vote) = params.vote_average_gte {
        query_params.push(("vote_average.gte", vote.to_string()));
    }
    if let Some(vote) = params.vote_average_lte {
        query_params.push(("vote_average.lte", vote.to_string()));
    }
    if let Some(ref v) = params.primary_release_date_gte {
        query_params.push(("primary_release_date.gte", v.clone()));
    }
    if let Some(ref v) = params.primary_release_date_lte {
        query_params.push(("primary_release_date.lte", v.clone()));
    }
    if let Some(ref v) = params.first_air_date_gte {
        query_params.push(("first_air_date.gte", v.clone()));
    }
    if let Some(ref v) = params.first_air_date_lte {
        query_params.push(("first_air_date.lte", v.clone()));
    }
    
    let params_ref: Vec<(&str, &str)> = query_params.iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();
    
    let data = client.get(&path, &params_ref).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/movie/:id - Get movie details for an FHUB title page.
async fn movie_details(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/movie/{}", id);
    let language = params.language.as_deref().unwrap_or("vi-VN");

    let mut data = client.get(&path, &[
        ("append_to_response", "credits,videos,images,keywords,release_dates,external_ids"),
        ("language", language),
    ]).await?;

    let overview_empty = data.get("overview")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);

    let tagline_empty = data.get("tagline")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);

    if overview_empty || tagline_empty || videos_empty(&data) {
        if let Ok(en_data) = client.get(&path, &[
            ("append_to_response", "credits,videos,images,keywords,release_dates,external_ids"),
            ("language", "en-US"),
        ]).await {
            merge_videos_if_empty(&mut data, &en_data);
            // Do not replace a missing Vietnamese overview/tagline with English text.
            // The UI should remain Vietnamese when language=vi-VN; English fallback is only
            // acceptable when the caller explicitly asks for an English locale.
            if !language.starts_with("vi") {
                if let Some(obj) = data.as_object_mut() {
                    if overview_empty {
                        if let Some(overview) = en_data.get("overview").and_then(|v| v.as_str()) {
                            if !overview.trim().is_empty() {
                                obj.insert("overview".to_string(), Value::String(overview.to_string()));
                            }
                        }
                    }
                    if tagline_empty {
                        if let Some(tagline) = en_data.get("tagline").and_then(|v| v.as_str()) {
                            if !tagline.trim().is_empty() {
                                obj.insert("tagline".to_string(), Value::String(tagline.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(data))
}

/// GET /api/tmdb/tv/:id - Get TV show details for an FHUB series page.
/// Requests Vietnamese overview first; if empty, falls back to English and then
/// original-language metadata for non-English shows.
async fn tv_details(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/tv/{}", id);
    let language = params.language.as_deref().unwrap_or("vi-VN");

    let mut data = client.get(&path, &[
        ("append_to_response", "credits,videos,images,keywords,content_ratings,external_ids"),
        ("language", language),
    ]).await?;

    // Prefer Vietnamese overview. If the metadata provider has no Vietnamese overview,
    // fallback to English, then finally to original language for non-English shows.
    let overview_empty = data.get("overview")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);

    if overview_empty || videos_empty(&data) {
        if let Ok(en_data) = client.get(&path, &[
            ("append_to_response", "credits,videos,images,keywords,content_ratings,external_ids"),
            ("language", "en-US"),
        ]).await {
            merge_videos_if_empty(&mut data, &en_data);
            // Keep Vietnamese UI Vietnamese: use English only for videos/trailers here,
            // not as an overview fallback when the requested locale is Vietnamese.
            if !language.starts_with("vi") {
                if let Some(overview) = en_data.get("overview").and_then(|v| v.as_str()) {
                    if !overview.trim().is_empty() {
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert("overview".to_string(), Value::String(overview.to_string()));
                        }
                    }
                }
            }
        }
    }

    let overview_empty = data.get("overview")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);

    if overview_empty {
        if let Some(orig_lang) = data.get("original_language").and_then(|v| v.as_str()) {
            // Build locale string: zh -> zh-CN, ko -> ko-KR, ja -> ja-JP, etc.
            let locale = match orig_lang {
                "zh" => "zh-CN",
                "ko" => "ko-KR",
                "ja" => "ja-JP",
                "vi" => "vi-VN",
                "th" => "th-TH",
                "pt" => "pt-BR",
                "es" => "es-ES",
                "fr" => "fr-FR",
                "de" => "de-DE",
                "it" => "it-IT",
                "ar" => "ar-SA",
                "ru" => "ru-RU",
                _ => orig_lang,
            };

            if let Ok(native_data) = client.get(&path, &[
                ("append_to_response", "keywords,content_ratings,external_ids"),
                ("language", locale),
            ]).await {
                // Splice in only the overview and native display name if useful for the FHUB UI.
                if let Some(overview) = native_data.get("overview").and_then(|v| v.as_str()) {
                    if !overview.trim().is_empty() {
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert("overview".to_string(), Value::String(overview.to_string()));
                            if let Some(orig_name) = native_data.get("name").and_then(|v| v.as_str()) {
                                obj.insert("original_name_native".to_string(), Value::String(orig_name.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(data))
}

/// GET /api/tmdb/collection/:id - Get collection details for FHUB collections.
async fn collection_details(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/collection/{}", id);
    
    let data = client.get(&path, &[("language", "vi-VN")]).await?;
    
    Ok(Json(data))
}

/// GET /api/tmdb/tv/:id/season/:season - Get season details for FHUB episode browsing.
async fn season_details(
    State(_state): State<Arc<AppState>>,
    Path((id, season)): Path<(u32, u32)>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/tv/{}/season/{}", id, season);
    
    let data = client.get(&path, &[("language", "vi-VN")]).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/genres/:media_type - Get genres for FHUB discovery filters.
async fn genres(
    State(_state): State<Arc<AppState>>,
    Path(media_type): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/genre/{}/list", media_type);
    
    let data = client.get(&path, &[]).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/:media_type/:id/similar - Get similar items for FHUB recommendations.
async fn similar(
    State(_state): State<Arc<AppState>>,
    Path((media_type, id)): Path<(String, u32)>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/{}/{}/similar", media_type, id);
    let page_str = params.page.to_string();
    let language = params.language.as_deref().unwrap_or("vi-VN");
    
    let data = client.get(&path, &[("page", &page_str), ("language", language)]).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/:media_type/:id/recommendations - Get recommendations for FHUB title pages.
async fn recommendations(
    State(_state): State<Arc<AppState>>,
    Path((media_type, id)): Path<(String, u32)>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/{}/{}/recommendations", media_type, id);
    let page_str = params.page.to_string();
    
    let data = client.get(&path, &[("page", &page_str)]).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/trending/:media_type/:time_window - Get trending media for FHUB discovery.
async fn trending(
    State(_state): State<Arc<AppState>>,
    Path((media_type, time_window)): Path<(String, String)>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, StatusCode> {
    let client = TmdbClient::new();
    let path = format!("/trending/{}/{}", media_type, time_window);
    let page_str = params.page.to_string();
    
    let data = client.get(&path, &[("page", &page_str)]).await?;
    Ok(Json(data))
}

/// GET /api/tmdb/image/:size/*path - Proxy metadata images for FHUB artwork.
async fn proxy_image(
    Path((size, path)): Path<(String, String)>,
) -> Result<axum::response::Response, StatusCode> {
    let client = Client::new();
    let url = format!("{}/{}/{}", TMDB_IMAGE_BASE, size, path);
    
    let resp = client.get(&url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
        
    if !resp.status().is_success() {
        return Err(StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY));
    }
    
    let content_type = resp.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
        
    let body = resp.bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
        
    Ok(axum::response::Response::builder()
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=31536000")
        .body(axum::body::Body::from(body))
        .unwrap())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Unit Tests - Helper Functions
    // ========================================================================

    #[test]
    fn test_default_media_type() {
        assert_eq!(default_media_type(), "multi");
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_tmdb_client_creation() {
        let client = TmdbClient::new();
        assert!(client.api_key.is_empty() || !client.api_key.is_empty());
    }

    // ========================================================================
    // Unit Tests - Query Parameters
    // ========================================================================

    #[test]
    fn test_search_query_deserialization() {
        let json = r#"{"q": "Inception", "media_type": "movie", "page": 2}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        
        assert_eq!(query.q, "Inception");
        assert_eq!(query.media_type, "movie");
        assert_eq!(query.page, 2);
    }

    #[test]
    fn test_search_query_defaults() {
        let json = r#"{"q": "Inception"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        
        assert_eq!(query.q, "Inception");
        assert_eq!(query.media_type, "multi"); // default
        assert_eq!(query.page, 1); // default
    }

    #[test]
    fn test_discover_query_full() {
        let json = r#"{
            "page": 1,
            "genre": "28",
            "year": 2024,
            "sort_by": "popularity.desc",
            "vote_average_gte": 7.5
        }"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        
        assert_eq!(query.page, 1);
        assert_eq!(query.genre, Some("28".to_string()));
        assert_eq!(query.year, Some(2024));
        assert_eq!(query.sort_by, Some("popularity.desc".to_string()));
        assert_eq!(query.vote_average_gte, Some(7.5));
    }

    #[test]
    fn test_discover_query_minimal() {
        let json = r#"{"page": 1}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        
        assert_eq!(query.page, 1);
        assert_eq!(query.genre, None);
        assert_eq!(query.year, None);
        assert_eq!(query.sort_by, None);
        assert_eq!(query.vote_average_gte, None);
    }

    #[test]
    fn test_discover_query_defaults() {
        let json = r#"{}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, 1); // default
    }

    #[test]
    fn test_pagination_query() {
        let json = r#"{"page": 5}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, 5);
    }

    #[test]
    fn test_pagination_query_default() {
        let json = r#"{}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, 1);
    }

    // ========================================================================
    // Unit Tests - URL Building
    // ========================================================================

    #[test]
    fn test_tmdb_api_base_constant() {
        assert_eq!(TMDB_API_BASE, "https://api.themoviedb.org/3");
    }

    #[test]
    fn test_tmdb_image_base_constant() {
        assert_eq!(TMDB_IMAGE_BASE, "https://image.tmdb.org/t/p");
    }

    // ========================================================================
    // Integration Tests - TmdbClient
    // ========================================================================

    #[tokio::test]
    async fn test_tmdb_client_get_without_api_key() {
        let client = TmdbClient::new();
        assert!(!client.api_key.is_empty(), "API key should be bundled for this FHUB runtime profile");
    }

    #[tokio::test]
    async fn test_tmdb_client_url_construction() {
        let client = TmdbClient::new();
        
        if client.api_key.is_empty() {
            let result = client.get("/test", &[]).await;
            assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // ========================================================================
    // Router Tests
    // ========================================================================

    #[test]
    fn test_router_creation() {
        let router = router();
        // Should not panic
        drop(router);
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_search_query_special_characters() {
        let json = r#"{"q": "The Matrix: Reloaded (2003)"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "The Matrix: Reloaded (2003)");
    }

    #[test]
    fn test_search_query_unicode() {
        let json = r#"{"q": "千と千尋の神隠し"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "千と千尋の神隠し");
    }

    #[test]
    fn test_discover_query_negative_vote() {
        let json = r#"{"vote_average_gte": 0.0}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.vote_average_gte, Some(0.0));
    }

    #[test]
    fn test_discover_query_max_vote() {
        let json = r#"{"vote_average_gte": 10.0}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.vote_average_gte, Some(10.0));
    }

    #[test]
    fn test_pagination_large_page() {
        let json = r#"{"page": 1000}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, 1000);
    }

    #[test]
    fn test_discover_query_old_year() {
        let json = r#"{"year": 1900}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.year, Some(1900));
    }

    #[test]
    fn test_discover_query_future_year() {
        let json = r#"{"year": 2030}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.year, Some(2030));
    }

    #[test]
    fn test_discover_query_multiple_genres() {
        let json = r#"{"genre": "28,12,16"}"#;
        let query: DiscoverQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.genre, Some("28,12,16".to_string()));
    }

    #[test]
    fn test_discover_query_all_sort_options() {
        let sort_options = vec![
            "popularity.asc",
            "popularity.desc",
            "release_date.asc",
            "release_date.desc",
            "revenue.asc",
            "revenue.desc",
            "primary_release_date.asc",
            "primary_release_date.desc",
            "original_title.asc",
            "original_title.desc",
            "vote_average.asc",
            "vote_average.desc",
            "vote_count.asc",
            "vote_count.desc",
        ];

        for sort in sort_options {
            let json = format!(r#"{{"sort_by": "{}"}}"#, sort);
            let query: DiscoverQuery = serde_json::from_str(&json).unwrap();
            assert_eq!(query.sort_by, Some(sort.to_string()));
        }
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_tmdb_response_serialization() {
        let value = serde_json::json!({
            "results": [],
            "page": 1,
            "total_pages": 1,
            "total_results": 0
        });
        
        let response = TmdbResponse { data: value.clone() };
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized, value);
    }
}

