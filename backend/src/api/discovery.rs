//! Discovery API Routes
//!
//! Smart search and discovery features.

use axum::{

    routing::{get, post},
    Router,
    Json,
    extract::{State, Query},
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use reqwest::Client;
use crate::AppState;
use crate::utils::smart_tokenizer::{smart_parse, MediaType};
use crate::utils::title_matcher::{extract_core_title, get_title_keywords, is_different_franchise_entry};
use std::collections::HashMap;
use futures_util::future::join_all;
use regex::Regex;
use crate::constants::TMDB_API_KEY;
use crate::hosts::{fshare::FshareHandler, create_shared_client, base::HostHandler};
use crate::utils::parser::FilenameParser;
use crate::api::search_pipeline::SearchPipeline;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/smart-search", post(smart_search))
        .route("/popular-today", get(popular_today))
        .route("/available-on-fshare", get(available_on_fshare))
        .route("/resolve-source-link", get(resolve_source_link))
        .route("/trending", get(trending))
        .route("/thuviencine-movies", get(thuviencine_movies))
        .route("/thuviencine-tv", get(thuviencine_tv_series))
}

// ============================================================================
// Request Types
// ============================================================================

#[derive(Deserialize)]
pub struct SmartSearchRequest {
    pub title: String,
    #[serde(default)]
    pub year: Option<String>,
    #[serde(default)]
    pub season: Option<u32>,
    #[serde(default)]
    pub episode: Option<u32>,
    #[serde(default = "default_media_type")]
    pub media_type: String,
    #[serde(default)]
    pub tmdb_id: Option<u32>,
}

#[derive(Deserialize)]
struct PopularQuery {
    #[serde(default = "default_media_type")]
    media_type: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    genre: Option<u32>,
    #[serde(default)]
    keyword: Option<u32>,
    #[serde(default)]
    window: Option<String>,
    #[serde(default = "default_page")]
    page: usize,
}

#[derive(Deserialize)]
struct AvailabilityQuery {
    title: String,
    #[serde(default)]
    year: Option<String>,
    #[serde(default)]
    original_title: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
struct SourceLinkQuery {
    url: String,
}

fn default_media_type() -> String { "movie".to_string() }
fn default_limit() -> usize { 20 }
fn default_page() -> usize { 1 }

// ============================================================================
// Response Types
// ============================================================================

#[allow(dead_code)]
#[derive(Serialize)]
pub struct SmartSearchResponse {
    pub queries_used: Vec<String>,
    pub results: Vec<SearchResult>,
    pub groups: Option<Vec<QualityGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<Vec<SeasonGroup>>,
    pub media_type: String,
    pub version: String,
}

#[derive(Serialize, Clone)]
pub struct SearchResult {
    pub name: String,
    pub original_name: String,
    pub url: String,
    pub size: u64,
    pub score: i32,
    pub fcode: String,
    pub quality: Option<String>,
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub viet_sub: bool,
    pub viet_dub: bool,
}

#[derive(Serialize)]
pub struct QualityGroup {
    pub quality: String,
    pub score: i32,
    pub count: usize,
    pub files: Vec<SearchResult>,
}

#[derive(Serialize)]
pub struct SeasonGroup {
    pub season: u32,
    pub episodes_grouped: Vec<EpisodeGroup>,
}

#[derive(Serialize)]
pub struct EpisodeGroup {
    pub episode_number: u32,
    pub name: String,
    pub files: Vec<SearchResult>,
}

#[derive(Serialize)]
struct PopularItem {
    id: u32,
    title: String,
    media_type: String,
    year: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    poster_url: Option<String>,
    backdrop_path: Option<String>,
    overview: Option<String>,
    vote_average: f32,
    score: f32,
    fshare_available: bool,
    fshare_count: usize,
}

#[derive(Serialize)]
struct PopularResponse {
    results: Vec<PopularItem>,
}

#[derive(Serialize)]
struct AvailabilityResponse {
    available: bool,
    count: usize,
    results: Vec<SearchResult>,
}


fn decode_html_entities(input: &str) -> String {
    input
        .replace("&#8211;", "–")
        .replace("&#8212;", "—")
        .replace("&#8217;", "'")
        .replace("&#8216;", "'")
        .replace("&#8220;", "\"")
        .replace("&#8221;", "\"")
        .replace("&#038;", "&")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&nbsp;", " ")
}

fn parse_fshare_result(name: String, url: String, size: u64, score: i32) -> SearchResult {
    let name = decode_html_entities(&name);
    let parsed = smart_parse(&name);
    let fcode = url
        .split("/file/")
        .nth(1)
        .or_else(|| url.split("/folder/").nth(1))
        .unwrap_or("")
        .split(|c| c == '?' || c == '&' || c == '/' || c == '\\')
        .next()
        .unwrap_or("")
        .to_string();

    SearchResult {
        name: parsed.title,
        original_name: name,
        url,
        size,
        score,
        fcode,
        quality: format!("{} {}",
            parsed.resolution.as_deref().unwrap_or(""),
            parsed.source.as_deref().unwrap_or("")
        ).trim().to_string().into(),
        resolution: parsed.resolution.clone(),
        source: parsed.source.clone(),
        viet_sub: parsed.viet_sub,
        viet_dub: parsed.viet_dub,
    }
}

async fn search_timfshare(client: &Client, query: &str) -> Vec<Value> {
    SearchPipeline::execute_fshare_search(client, query, 50)
        .await
        .into_iter()
        .map(|item| json!({
            "name": item.name,
            "url": item.url,
            "size": item.size,
        }))
        .collect()
}

async fn enrich_fshare_result(state: Arc<AppState>, mut result: SearchResult) -> SearchResult {
    result.name = decode_html_entities(&result.name);
    result.original_name = decode_html_entities(&result.original_name);

    if result.url.contains("/file/") && (result.size == 0 || result.quality.as_deref().unwrap_or("").is_empty()) {
        let handler = FshareHandler::new(state.config.fshare.clone(), create_shared_client());
        if let Ok(info) = tokio::time::timeout(std::time::Duration::from_secs(8), handler.get_file_info(&result.url)).await.unwrap_or_else(|_| Err(anyhow::anyhow!("timeout"))) {
            result.size = info.size;
            result.original_name = decode_html_entities(&info.filename);
            let parsed = smart_parse(&result.original_name);
            result.name = parsed.title;
            let quality = format!("{} {}", parsed.resolution.as_deref().unwrap_or(""), parsed.source.as_deref().unwrap_or("")).trim().to_string();
            result.quality = if quality.is_empty() { None } else { Some(quality) };
            result.resolution = parsed.resolution;
            result.source = parsed.source;
            result.viet_sub = parsed.viet_sub;
            result.viet_dub = parsed.viet_dub;
        }
    } else if result.url.contains("/folder/") && result.size == 0 {
        if let Some(folder_code) = result.url.split("/folder/").nth(1).and_then(|s| s.split(|c| c == '?' || c == '&' || c == '/').next()) {
            let client = Client::builder().timeout(std::time::Duration::from_secs(8)).build().unwrap_or_default();
            if let Ok(resp) = client
                .get("https://www.fshare.vn/api/v3/files/folder")
                .query(&[("linkcode", folder_code), ("page", "1"), ("per-page", "100"), ("sort", "type")])
                .header("User-Agent", "Mozilla/5.0 (FHub)")
                .send()
                .await
            {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(items) = data["items"].as_array() {
                        let mut total_size = 0u64;
                        let mut first_file_name: Option<String> = None;
                        for item in items {
                            let size = item["size"].as_u64().or_else(|| item["size"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0);
                            total_size = total_size.saturating_add(size);
                            if first_file_name.is_none() {
                                if let Some(name) = item["name"].as_str() {
                                    first_file_name = Some(decode_html_entities(name));
                                }
                            }
                        }
                        if total_size > 0 { result.size = total_size; }
                        if result.quality.as_deref().unwrap_or("").is_empty() {
                            if let Some(name) = first_file_name {
                                let parsed = FilenameParser::parse(&name);
                                let quality = parsed.quality_attrs.quality_name();
                                if !quality.is_empty() && quality != "Unknown" {
                                    result.quality = Some(quality.clone());
                                    result.resolution = Some(quality);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

async fn enrich_fshare_results(state: Arc<AppState>, results: Vec<SearchResult>) -> Vec<SearchResult> {
    join_all(results.into_iter().map(|result| enrich_fshare_result(state.clone(), result))).await
}

async fn search_thuviencine(client: &Client, query: &str) -> Vec<SearchResult> {
    let search_url = format!(
        "https://thuviencine.live/wp-json/wp/v2/search?search={}&subtype=post&per_page=5",
        urlencoding::encode(query)
    );
    let mut results = Vec::new();
    let mut seen = HashMap::new();

    let posts: Vec<Value> = match client
        .get(&search_url)
        .header("User-Agent", "Mozilla/5.0 (FHub)")
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
        _ => Vec::new(),
    };

    let re = match Regex::new(r#"https?://(?:www\.)?fshare\.vn/(?:file|folder)/[A-Za-z0-9]+"#) {
        Ok(re) => re,
        Err(_) => return results,
    };

    for post in posts {
        let post_id = post["id"].as_u64().unwrap_or(0);
        if post_id == 0 { continue; }
        let title = post["title"].as_str().unwrap_or("Thư Viện Cine").to_string();
        let download_url = format!("https://thuviencine.live/download?id={}", post_id);

        let html = match client
            .get(&download_url)
            .header("User-Agent", "Mozilla/5.0 (FHub)")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            _ => String::new(),
        };

        for mat in re.find_iter(&html) {
            let url = mat.as_str().to_string();
            if seen.contains_key(&url) { continue; }
            seen.insert(url.clone(), true);
            let label = format!("{} - {}", title, if url.contains("/folder/") { "Folder FShare" } else { "File FShare" });
            results.push(parse_fshare_result(label, url, 0, 80));
        }
    }

    results
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/discovery/smart-search - Perform smart search with v2 tactics
async fn smart_search(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<SmartSearchRequest>,
) -> Json<Value> {
    let client = Client::builder().cookie_store(true).build().unwrap_or_default();
    let mut queries = vec![payload.title.clone()];
    
    // 1. Resolve Aliases from TMDB
    let mut aliases = Vec::new();
    if let Some(tmdb_id) = payload.tmdb_id {
        let url = format!(
            "https://api.themoviedb.org/3/{}/{}/alternative_titles?api_key={}",
            if payload.media_type == "tv" { "tv" } else { "movie" },
            tmdb_id,
            TMDB_API_KEY
        );
        
        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(data) = resp.json::<Value>().await {
                if let Some(titles) = data["titles"].as_array().or_else(|| data["results"].as_array()) {
                    for t in titles {
                        if let Some(title) = t["title"].as_str().or_else(|| t["name"].as_str()) {
                            aliases.push(title.to_string());
                            if aliases.len() < 3 {
                                queries.push(title.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Add Year to queries
    if let Some(ref year) = payload.year {
        let base_queries = queries.clone();
        for q in base_queries {
            queries.push(format!("{} {}", q, year));
        }
    }

    // Add S/E markers
    if let (Some(s), Some(e)) = (payload.season, payload.episode) {
        queries.push(format!("{} S{:02}E{:02}", payload.title, s, e));
        queries.push(format!("{} S{:02} E{:02}", payload.title, s, e));
        queries.push(format!("{} {}x{:02}", payload.title, s, e));
    } else if let Some(s) = payload.season {
        queries.push(format!("{} Season {}", payload.title, s));
        queries.push(format!("{} S{:02}", payload.title, s));
    }

    // 2. Prefer Thư Viện Cine for movie searches because TimFshare can be slow/unresponsive.
    //    Keep TimFshare as fallback, and keep TV searches on TimFshare first for season/episode matching.
    let prefer_thuviencine = payload.media_type != "tv";
    let mut thuviencine_results = if prefer_thuviencine {
        search_thuviencine(&client, &payload.title).await
    } else {
        Vec::new()
    };

    let mut all_raw_results = Vec::new();
    if !prefer_thuviencine || thuviencine_results.is_empty() {
        for query in &queries {
            all_raw_results.extend(search_timfshare(&client, query).await);
        }
    }

    // 3. Parse and Filter
    let mut filtered_results = Vec::new();
    let mut seen_urls = HashMap::new();
    let search_keywords = get_title_keywords(&payload.title);

    for item in all_raw_results {
        let name = item["name"].as_str().unwrap_or("").to_string();
        let url = item["url"].as_str().unwrap_or("").to_string();
        
        if seen_urls.contains_key(&url) { continue; }
        seen_urls.insert(url.clone(), true);

        // Franchise Conflict Check
        if is_different_franchise_entry(&payload.title, &name) { continue; }

        // Unified Similarity Check from v2
        let sim_res = crate::utils::title_matcher::calculate_unified_similarity(&payload.title, &name, &[]);
        if !sim_res.is_valid && search_keywords.len() > 1 {
            // Allow if it's a very high similarity match of an alias
            let mut alias_match = false;
            for alias in &aliases {
                let alias_sim = crate::utils::title_matcher::calculate_unified_similarity(alias, &name, &[]);
                if alias_sim.is_valid {
                    alias_match = true;
                    break;
                }
            }
            if !alias_match { continue; }
        }

        let parsed = smart_parse(&name);
        
        // Season validation for series (Strict Mode)
        if let Some(req_s) = payload.season {
            if let Some(file_s) = parsed.season {
                if file_s != req_s { continue; }
            }
        }
        // Episode validation REMOVED to match V2 permissive behavior (V2 regex fails on 'Chapter', allowing all files)

        let fcode = url.split("/file/").last().unwrap_or("").to_string();
        
        // Calculate Score (V2 Parity)
        let matched_count = (search_keywords.len() as f32 * sim_res.score).round() as i32;
        let mut score = matched_count * 10;
        score += (sim_res.score * 50.0) as i32;
        
        if parsed.year.is_some() { score += 20; }
        if parsed.resolution.is_some() { score += 10; }
        if parsed.viet_dub || parsed.viet_sub { score += 15; }
        
        let size_gb = item["size"].as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
        score += (size_gb.min(10.0) * 5.0) as i32;

        filtered_results.push(SearchResult {
            name: parsed.title,
            original_name: name,
            url,
            size: item["size"].as_u64().unwrap_or(0),
            score,
            fcode,
            quality: format!("{} {}", 
                parsed.resolution.as_deref().unwrap_or(""),
                parsed.source.as_deref().unwrap_or("")
            ).trim().to_string().into(),
            resolution: parsed.resolution.clone(),
            source: parsed.source.clone(),
            viet_sub: parsed.viet_sub,
            viet_dub: parsed.viet_dub,
        });
    }

    if filtered_results.is_empty() && !thuviencine_results.is_empty() {
        filtered_results.extend(thuviencine_results);
    }

    // Sort by score desc
    filtered_results.sort_by(|a, b| b.score.cmp(&a.score));

    // 4. Grouping
    if payload.media_type == "tv" {
        // Group by season/episode
        let mut seasons_map: HashMap<u32, HashMap<u32, Vec<SearchResult>>> = HashMap::new();
        
        for res in filtered_results {
            let s = payload.season.unwrap_or(1); // Default to search season or 1
            let e = smart_parse(&res.original_name).episode.unwrap_or(0);
            
            seasons_map.entry(s).or_default()
                .entry(e).or_default()
                .push(res);
        }

        let mut seasons = Vec::new();
        for (s_num, eps_map) in seasons_map {
            let mut episodes_grouped = Vec::new();
            for (e_num, files) in eps_map {
                episodes_grouped.push(EpisodeGroup {
                    episode_number: e_num,
                    name: format!("Episode {}", e_num),
                    files,
                });
            }
            episodes_grouped.sort_by(|a, b| a.episode_number.cmp(&b.episode_number));
            seasons.push(SeasonGroup {
                season: s_num,
                episodes_grouped,
            });
        }

        Json(json!({
            "queries_used": queries,
            "seasons": seasons,
            "media_type": "tv",
            "version": "v3.2-reflection-fix"
        }))
    } else {
        // Group by Quality for Movies
        let mut quality_map: HashMap<String, Vec<SearchResult>> = HashMap::new();
        for res in filtered_results {
            let q = res.resolution.clone().unwrap_or("SD".to_string());
            quality_map.entry(q).or_default().push(res);
        }

        let mut groups = Vec::new();
        for (q_name, files) in quality_map {
            let avg_score = files.iter().map(|f| f.score).sum::<i32>() / files.len() as i32;
            groups.push(QualityGroup {
                quality: q_name,
                score: avg_score,
                count: files.len(),
                files,
            });
        }
        groups.sort_by(|a, b| b.score.cmp(&a.score));

        Json(json!({
            "queries_used": queries,
            "groups": groups,
            "media_type": "movie",
            "version": "v3.2-reflection-fix"
        }))
    }
}

/// GET /api/discovery/popular-today - Get popular items with Fshare availability
async fn popular_today(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<PopularQuery>,
) -> Json<PopularResponse> {
    let client = Client::new();
    let window = params.window.as_deref().unwrap_or("day");
    let url = if params.genre.is_some() || params.keyword.is_some() {
        let mut url = format!(
            "https://api.themoviedb.org/3/discover/{}?api_key={}&sort_by=popularity.desc&page={}",
            params.media_type,
            TMDB_API_KEY,
            params.page
        );
        if let Some(genre_id) = params.genre {
            url.push_str(&format!("&with_genres={}", genre_id));
        }
        if let Some(keyword_id) = params.keyword {
            url.push_str(&format!("&with_keywords={}", keyword_id));
        }
        url
    } else if window == "all" || params.page > 1 {
        // TMDB trending has no real pagination; use popular pages for "Xem thêm"
        // to avoid repeating the same 20 trending items.
        format!(
            "https://api.themoviedb.org/3/{}//popular?api_key={}&page={}",
            params.media_type,
            TMDB_API_KEY,
            params.page
        ).replace("//popular", "/popular")
    } else {
        format!(
            "https://api.themoviedb.org/3/trending/{}/{}?api_key={}",
            params.media_type,
            if window == "week" { "week" } else { "day" },
            TMDB_API_KEY
        )
    };
    
    let mut results: Vec<PopularItem> = Vec::new();
    
    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(data) = resp.json::<Value>().await {
            if let Some(items) = data["results"].as_array() {
                for item in items.iter().take(params.limit) {
                    let id = item["id"].as_u64().unwrap_or(0) as u32;
                    let title = item["title"].as_str()
                        .or_else(|| item["name"].as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let poster_path = item["poster_path"].as_str();
                    let backdrop_path = item["backdrop_path"].as_str().map(|s| s.to_string());
                    let overview = item["overview"].as_str().map(|s| s.to_string());
                    let release_date = item["release_date"].as_str().map(|s| s.to_string());
                    let first_air_date = item["first_air_date"].as_str().map(|s| s.to_string());
                    let year = release_date
                        .as_deref()
                        .or(first_air_date.as_deref())
                        .and_then(|date| date.get(0..4))
                        .filter(|value| value.chars().all(|ch| ch.is_ascii_digit()))
                        .map(|value| value.to_string());
                    let vote_average = item["vote_average"].as_f64().unwrap_or(0.0) as f32;
                    
                    results.push(PopularItem {
                        id,
                        title,
                        media_type: params.media_type.clone(),
                        year,
                        release_date,
                        first_air_date,
                        poster_url: poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                        backdrop_path,
                        overview,
                        vote_average,
                        score: vote_average,
                        fshare_available: true, // Mocked for UI
                        fshare_count: 5,
                    });
                }
            }
        }
    }
    
    Json(PopularResponse { results })
}

/// GET /api/discovery/resolve-source-link - Resolve the exact source page URL (e.g. Thuviencine download page) to FShare links.
async fn resolve_source_link(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SourceLinkQuery>,
) -> Json<AvailabilityResponse> {
    let source_url = params.url.trim();
    let mut results: Vec<SearchResult> = Vec::new();

    if source_url.contains("fshare.vn/file/") || source_url.contains("fshare.vn/folder/") {
        results.push(parse_fshare_result("Nguồn FShare".to_string(), source_url.to_string(), 0, 100));
    } else if source_url.starts_with("https://thuviencine.live/") || source_url.starts_with("http://thuviencine.live/") || source_url.starts_with("https://thuviencine.xyz/") || source_url.starts_with("http://thuviencine.xyz/") {
        let client = Client::builder().cookie_store(true).build().unwrap_or_default();
        let html = match client
            .get(source_url)
            .header("User-Agent", "Mozilla/5.0 (FHub)")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            _ => String::new(),
        };
        if let Ok(re) = Regex::new(r#"https?://(?:www\.)?fshare\.vn/(?:file|folder)/[A-Za-z0-9]+(?:\?[^\"'<>\s]*)?"#) {
            let mut seen = std::collections::HashSet::new();
            for mat in re.find_iter(&html) {
                let url = mat.as_str().trim_end_matches(['&', '?', '.', ',']).to_string();
                if seen.insert(url.clone()) {
                    results.push(parse_fshare_result("Nguồn Thuviencine".to_string(), url, 0, 100));
                }
            }
        }
    }

    results = enrich_fshare_results(state, results).await;
    let count = results.len();
    Json(AvailabilityResponse { available: count > 0, count, results })
}

/// GET /api/discovery/available-on-fshare - Check Fshare availability
async fn available_on_fshare(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AvailabilityQuery>,
) -> Json<AvailabilityResponse> {
    let query = if let Some(ref year) = params.year {
        format!("{} {}", params.title, year)
    } else {
        params.title.clone()
    };
    let original_query = params.original_title.as_ref().and_then(|original| {
        if original.trim().is_empty() || original.eq_ignore_ascii_case(&params.title) {
            None
        } else if let Some(ref year) = params.year {
            Some(format!("{} {}", original, year))
        } else {
            Some(original.clone())
        }
    });
    
    let client = Client::builder().cookie_store(true).build().unwrap_or_default();
    let mut results: Vec<SearchResult> = Vec::new();
    let limit = params.limit.clamp(1, 200);

    let mut seen_urls = std::collections::HashSet::new();
    for search_query in std::iter::once(&query).chain(original_query.iter()) {
        for item in search_timfshare(&client, search_query).await.into_iter().take(limit) {
            let name = item["name"].as_str().unwrap_or("").to_string();
            let url = item["url"].as_str().unwrap_or("").to_string();
            if !url.is_empty() && seen_urls.insert(url.clone()) {
                results.push(parse_fshare_result(name, url, item["size"].as_u64().unwrap_or(0), 0));
            }
        }
    }

    if results.is_empty() {
        for search_query in std::iter::once(&query).chain(original_query.iter()) {
            for item in search_thuviencine(&client, search_query).await.into_iter().take(limit) {
                if !item.url.is_empty() && seen_urls.insert(item.url.clone()) {
                    results.push(item);
                }
            }
        }
    }
    
    results = enrich_fshare_results(state, results).await;
    let count = results.len();
    Json(AvailabilityResponse {
        available: count > 0,
        count,
        results,
    })
}


#[derive(Deserialize)]
struct ThuviencineQuery {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn extract_year_from_title(title: &str) -> Option<String> {
    Regex::new(r"\((19|20)\d{2}\)").ok()
        .and_then(|re| re.find(title).map(|m| m.as_str().trim_matches(['(', ')']).to_string()))
}

fn clean_thuviencine_title(title: &str) -> String {
    let decoded = decode_html_entities(title);
    let no_year = Regex::new(r"\s*\((19|20)\d{2}(?:\s*-\s*(?:19|20)\d{2})?\)\s*$")
        .ok()
        .map(|re| re.replace(&decoded, "").to_string())
        .unwrap_or(decoded);
    let no_prefix = no_year.strip_prefix("Phim ").unwrap_or(&no_year).to_string();
    let preferred = no_prefix
        .split('–')
        .last()
        .unwrap_or(&no_prefix)
        .trim();
    Regex::new(r"(?i)\s*:\s*Season\s+\d+(?:\s*-\s*\d+)?\s*$")
        .ok()
        .map(|re| re.replace(preferred, "").to_string())
        .unwrap_or_else(|| preferred.to_string())
        .trim()
        .to_string()
}

fn clean_tv_title(title: &str) -> String {
    clean_thuviencine_title(title)
}

async fn enrich_thuviencine_items_with_tmdb(results: &mut Vec<TrendingItem>, media_type: &str, client: &Client) {
    let mut tasks = Vec::new();
    for item in results.iter() {
        let title = extract_core_title(&item.name);
        let year = item.year.clone();
        let client = client.clone();
        let media_type = media_type.to_string();
        tasks.push(tokio::spawn(async move {
            let date_param = if media_type == "tv" { "first_air_date_year" } else { "year" };
            let mut url = format!("https://api.themoviedb.org/3/search/{}?api_key={}&query={}", media_type, TMDB_API_KEY, urlencoding::encode(&title));
            if let Some(y) = &year { url.push_str(&format!("&{}={}", date_param, y)); }
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(data) = resp.json::<Value>().await {
                    return data["results"].as_array().and_then(|r| r.first()).cloned();
                }
            }
            None
        }));
    }
    let tmdb_results = join_all(tasks).await;
    for (i, join_res) in tmdb_results.into_iter().enumerate() {
        if let Ok(Some(data)) = join_res {
            let item = &mut results[i];
            item.tmdb_id = data["id"].as_u64().map(|id| id as u32);
            item.tmdb_title = data["title"].as_str().or_else(|| data["name"].as_str()).map(|s| s.to_string());
            if let Some(path) = data["poster_path"].as_str() { item.poster_url = Some(format!("https://image.tmdb.org/t/p/w500{}", path)); }
            item.vote_average = data["vote_average"].as_f64().map(|v| v as f32);
            if item.year.is_none() {
                let date_key = if media_type == "tv" { "first_air_date" } else { "release_date" };
                item.year = data[date_key].as_str().and_then(|d| d.split('-').next()).map(|s| s.to_string());
            }
        }
    }
}

/// GET /api/discovery/thuviencine-movies - latest movies from Thuviencine category
async fn thuviencine_movies(Query(query): Query<ThuviencineQuery>) -> Json<TrendingResponse> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .cookie_store(true)
        .build()
        .unwrap_or_default();
    let limit = query.limit.clamp(1, 30);
    let page = query.page.max(1);
    let posts_url = format!(
        "https://thuviencine.xyz/wp-json/wp/v2/posts?categories=2&per_page={}&page={}&_fields=id,title,link,date",
        limit, page
    );
    let posts: Vec<Value> = match client.get(&posts_url).header("User-Agent", "Mozilla/5.0 (FHub)").send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut results = Vec::new();
    for post in posts {
        let post_id = post["id"].as_u64().unwrap_or(0);
        if post_id == 0 { continue; }
        let raw_title = post["title"]["rendered"].as_str().or_else(|| post["title"].as_str()).unwrap_or("Phim lẻ");
        let title = decode_html_entities(raw_title).trim().to_string();
        let parsed_title = clean_thuviencine_title(&title);
        let year = extract_year_from_title(&title);
        let download_url = format!("https://thuviencine.xyz/download/?id={}", post_id);
        results.push(TrendingItem {
            fcode: post_id.to_string(),
            original_filename: title.clone(),
            name: parsed_title,
            url: download_url,
            size: 0,
            quality: Some("Thư Viện Cine".to_string()),
            has_vietsub: true,
            has_vietdub: false,
            tmdb_id: None,
            tmdb_title: None,
            poster_url: None,
            vote_average: None,
            year,
            media_type: Some("movie".to_string()),
        });
    }
    enrich_thuviencine_items_with_tmdb(&mut results, "movie", &client).await;
    Json(TrendingResponse { results })
}

/// GET /api/discovery/thuviencine-tv - latest TV Series from Thuviencine category
async fn thuviencine_tv_series(Query(query): Query<ThuviencineQuery>) -> Json<TrendingResponse> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .cookie_store(true)
        .build()
        .unwrap_or_default();
    let limit = query.limit.clamp(1, 30);
    let page = query.page.max(1);
    let posts_url = format!(
        "https://thuviencine.xyz/wp-json/wp/v2/posts?categories=1&per_page={}&page={}&_fields=id,title,link,date",
        limit, page
    );
    let posts: Vec<Value> = match client.get(&posts_url).header("User-Agent", "Mozilla/5.0 (FHub)").send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut results = Vec::new();
    for post in posts {
        let post_id = post["id"].as_u64().unwrap_or(0);
        if post_id == 0 { continue; }
        let raw_title = post["title"]["rendered"].as_str().or_else(|| post["title"].as_str()).unwrap_or("Phim bộ");
        let title = decode_html_entities(raw_title).trim().to_string();
        let parsed_title = clean_tv_title(&title);
        let year = extract_year_from_title(&title);
        let download_url = format!("https://thuviencine.xyz/download/?id={}", post_id);
        // Keep list loading fast: do not open every post's download page here.
        // The UI only needs poster/metadata for the grid; resolving the final FShare URL
        // can happen later when the user opens/chooses a title.
        results.push(TrendingItem {
            fcode: post_id.to_string(),
            original_filename: title.clone(),
            name: parsed_title,
            url: download_url,
            size: 0,
            quality: Some("FShare Folder".to_string()),
            has_vietsub: true,
            has_vietdub: false,
            tmdb_id: None,
            tmdb_title: None,
            poster_url: None,
            vote_average: None,
            year,
            media_type: Some("tv".to_string()),
        });
    }
    enrich_thuviencine_items_with_tmdb(&mut results, "tv", &client).await;
    Json(TrendingResponse { results })
}

/// GET /api/discovery/trending
async fn trending() -> Json<TrendingResponse> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .cookie_store(true)
        .build()
        .unwrap_or_default();
    let url = "https://timfshare.com/api/key/data-top";
    
    let mut results = Vec::new();
    
    if let Ok(resp) = client.get(url).send().await {
        if let Ok(data) = resp.json::<Value>().await {
            if let Some(items) = data["dataFile"].as_array() {
                // Filter video files
                let video_exts = [".mp4", ".mkv", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v"];
                
                for item in items.iter().take(50) {
                    let name = item["name"].as_str().unwrap_or("").to_string();
                    let has_video_ext = video_exts.iter().any(|ext| name.to_lowercase().ends_with(ext));
                    
                    if !has_video_ext { continue; }
                    
                    let parsed = smart_parse(&name);
                    let url = format!("https://www.fshare.vn/file/{}", item["linkcode"].as_str().unwrap_or(""));
                    let fcode = item["linkcode"].as_str().unwrap_or("").to_string();
                    let size = item["size"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                    
                    // Quality string
                    let qual_str = format!("{} {}", 
                        parsed.resolution.as_deref().unwrap_or(""), 
                        parsed.source.as_deref().unwrap_or("")
                    ).trim().to_string();
                    let quality = if qual_str.is_empty() { None } else { Some(qual_str) };

                    results.push(TrendingItem {
                        fcode,
                        original_filename: name.clone(),
                        name: parsed.title,
                        url,
                        size,
                        quality,
                        has_vietsub: parsed.viet_sub,
                        has_vietdub: parsed.viet_dub,
                        tmdb_id: None,
                        tmdb_title: None,
                        poster_url: None,
                        vote_average: None,
                        year: parsed.year.map(|y| y.to_string()),
                        media_type: if parsed.media_type == MediaType::TvShow { Some("tv".to_string()) } else { Some("movie".to_string()) },
                    });
                    
                    if results.len() >= 20 { break; }
                }
            }
        }
    }
    
    // Parallel Enrichment
    let mut tasks = Vec::new();
    for item in results.iter() {
        let clean_title = extract_core_title(&item.name); // Using parsed title
        let year = item.year.clone();
        let is_series = item.media_type.as_deref() == Some("tv");
        let client = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let media_type = if is_series { "tv" } else { "movie" };
            let mut url = format!(
                "https://api.themoviedb.org/3/search/{}?api_key={}&query={}",
                media_type, TMDB_API_KEY, urlencoding::encode(&clean_title)
            );
             if let Some(y) = &year {
                if media_type == "movie" {
                    url.push_str(&format!("&primary_release_year={}", y));
                } else {
                    url.push_str(&format!("&first_air_date_year={}", y));
                }
            }
            
            if let Ok(resp) = client.get(&url).send().await {
                 if let Ok(data) = resp.json::<Value>().await {
                    if let Some(results) = data["results"].as_array() {
                        if let Some(first) = results.first() {
                            return Some(first.clone());
                        }
                    }
                }
            }
            // Retry without year
            if year.is_some() {
                 let url = format!(
                    "https://api.themoviedb.org/3/search/{}?api_key={}&query={}",
                    media_type, TMDB_API_KEY, urlencoding::encode(&clean_title)
                );
                 if let Ok(resp) = client.get(&url).send().await {
                     if let Ok(data) = resp.json::<Value>().await {
                        if let Some(results) = data["results"].as_array() {
                            if let Some(first) = results.first() {
                                return Some(first.clone());
                            }
                        }
                    }
                }
            }
            None
        }));
    }
    
    let tmdb_results = join_all(tasks).await;
    
    for (i, join_res) in tmdb_results.into_iter().enumerate() {
        if let Ok(Some(data)) = join_res {
            let item = &mut results[i];
            item.tmdb_id = data["id"].as_u64().map(|id| id as u32);
            item.tmdb_title = data["title"].as_str().or_else(|| data["name"].as_str()).map(|s| s.to_string());
            if let Some(path) = data["poster_path"].as_str() {
                item.poster_url = Some(format!("https://image.tmdb.org/t/p/w500{}", path));
            }
            item.vote_average = data["vote_average"].as_f64().map(|v| v as f32);
            
             // Fix media type if unknown
            if item.media_type.is_none() {
                 if let Some(mt) = data["media_type"].as_str() {
                     item.media_type = Some(mt.to_string());
                 }
            }
             // Fix year if unknown
            if item.year.is_none() {
                if let Some(date) = data.get("release_date").and_then(|v| v.as_str()) {
                    item.year = date.split('-').next().map(|s| s.to_string());
                } else if let Some(date) = data.get("first_air_date").and_then(|v| v.as_str()) {
                    item.year = date.split('-').next().map(|s| s.to_string());
                }
            }
        }
    }

    Json(TrendingResponse { results })
}

#[derive(Serialize)]
pub struct TrendingResponse {
    pub results: Vec<TrendingItem>,
}

#[derive(Serialize)]
pub struct TrendingItem {
    #[serde(rename = "id")]
    pub fcode: String,
    
    #[serde(rename = "name")]
    pub original_filename: String,
    
    #[serde(rename = "parsed_name")]
    pub name: String,
    
    pub url: String,
    pub size: u64,
    pub quality: Option<String>,
    
    #[serde(rename = "vietsub")]
    pub has_vietsub: bool,
    
    #[serde(rename = "vietdub")]
    pub has_vietdub: bool,
    
    pub tmdb_id: Option<u32>,
    pub tmdb_title: Option<String>,
    pub poster_url: Option<String>,
    pub vote_average: Option<f32>,
    pub year: Option<String>,
    pub media_type: Option<String>,
}
