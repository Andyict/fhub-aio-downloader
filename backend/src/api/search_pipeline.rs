//! FHub Source Search Pipeline
//!
//! Contains reusable source discovery components extracted from smart search flows.
//! Provides building blocks for FHUB movie and series ingest handlers.

use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use regex::Regex;
use serde_json::Value;
use std::env;
use tracing::{info, warn};

use crate::fhub_source::{FhubIngestPlan, FhubSourceCandidate};

/// Allowed media file extensions (lowercase, without dot)
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v",
    "mpg", "mpeg", "m2ts", "vob", "3gp", "ogv", "divx", "rmvb",
    "rar", "zip", "7z",
];

/// Check if a filename is suitable for the FHUB media/source surface.
pub fn is_media_file(filename: &str) -> bool {
    if let Some(dot_pos) = filename.rfind('.') {
        let ext = &filename[dot_pos + 1..].to_lowercase();
        MEDIA_EXTENSIONS.contains(&ext.as_str())
    } else {
        true
    }
}

/// Raw source search result returned by the upstream source index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFshareResult {
    pub name: String,
    pub url: String,
    pub fcode: String,
    pub size: u64,
    pub score: i32,
}

impl From<RawFshareResult> for FhubSourceCandidate {
    fn from(result: RawFshareResult) -> Self {
        FhubSourceCandidate::new(result.fcode, result.name, result.url).with_size(result.size)
    }
}

fn is_fshare_url(url: &str) -> bool {
    url.contains("fshare.vn/file/") || url.contains("fshare.vn/folder/")
}

fn extract_fshare_code(url: &str) -> String {
    url.split("/file/")
        .nth(1)
        .or_else(|| url.split("/folder/").nth(1))
        .unwrap_or("")
        .split(|c| c == '?' || c == '&' || c == '/' || c == '\\')
        .next()
        .unwrap_or("")
        .to_string()
}

fn decode_html_url(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%3F", "?")
        .replace("%3D", "=")
        .replace("%26", "&")
}

fn extract_fshare_urls_from_html(html: &str, link_re: &Regex, redirect_re: &Regex) -> Vec<String> {
    let mut urls = Vec::new();
    for mat in link_re.find_iter(html) {
        urls.push(decode_html_url(mat.as_str()));
    }
    for caps in redirect_re.captures_iter(html) {
        if let Some(encoded) = caps.get(1) {
            let decoded = decode_html_url(encoded.as_str());
            for mat in link_re.find_iter(&decoded) {
                urls.push(decode_html_url(mat.as_str()));
            }
        }
    }
    urls
}

fn extract_search_result_urls(html: &str, redirect_re: &Regex) -> Vec<String> {
    let href_re = match Regex::new(r#"href=[\"']([^\"']+)[\"']"#) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for caps in href_re.captures_iter(html) {
        let Some(raw) = caps.get(1).map(|m| decode_html_url(m.as_str())) else { continue; };
        let mut url = raw.clone();
        if let Some(rcaps) = redirect_re.captures(&raw) {
            if let Some(encoded) = rcaps.get(1) {
                url = decode_html_url(encoded.as_str());
            }
        }
        if url.starts_with("//") {
            url = format!("https:{}", url);
        }
        if !url.starts_with("http://") && !url.starts_with("https://") { continue; }
        if url.contains("google.") || url.contains("bing.com") || url.contains("duckduckgo.com") { continue; }
        if url.contains("fshare.vn") { continue; }
        if seen.insert(url.clone()) {
            urls.push(url);
        }
        if urls.len() >= 8 { break; }
    }

    urls
}

impl RawFshareResult {
    /// Convert the legacy raw result shape into FHUB's native source candidate boundary.
    pub fn into_fhub_candidate(self) -> FhubSourceCandidate {
        self.into()
    }

    /// Convert a borrowed raw result into FHUB's native source candidate boundary.
    pub fn to_fhub_candidate(&self) -> FhubSourceCandidate {
        self.clone().into()
    }
}

/// Source search pipeline utilities.
pub struct SearchPipeline;

impl SearchPipeline {
    /// Execute source search with a moka cache layer.
    /// Cache key is the normalized query string. Hits skip the network entirely.
    pub async fn execute_fshare_search_cached(
        client: &Client,
        query: &str,
        limit: usize,
        cache: &Cache<String, Vec<RawFshareResult>>,
    ) -> Vec<RawFshareResult> {
        let cache_key = query.trim().to_lowercase();

        if let Some(cached) = cache.get(&cache_key).await {
            info!("FHUB source cache hit: '{}' ({} results)", query, cached.len());
            return cached.into_iter().take(limit).collect();
        }

        let results = Self::execute_fshare_search(client, query, limit).await;

        if !results.is_empty() {
            cache.insert(cache_key, results.clone()).await;
        }

        results
    }

    /// Execute source search without cache for targeted discovery queries.
    pub async fn execute_fshare_search(client: &Client, query: &str, limit: usize) -> Vec<RawFshareResult> {
        let mut results = Self::search_timfshare(client, query, limit).await;

        // VietMediaF uses fshare.vip as its primary FShare index, with TimFShare as fallback.
        // Keep it ahead of generic web scraping because it returns structured FShare metadata.
        if results.len() < limit {
            let remaining = limit.saturating_sub(results.len());
            results.extend(Self::search_fshare_vip(client, query, remaining).await);
        }

        // TimFShare/fshare.vip are unofficial indexes and may block or change routes.
        // Supplement with public web search for indexed fshare.vn file/folder URLs,
        // then let FShare metadata enrichment resolve filenames/sizes where possible.
        if results.len() < limit {
            let remaining = limit.saturating_sub(results.len());
            results.extend(Self::search_public_fshare_links(client, query, remaining).await);
        }

        Self::deduplicate_by_fcode(results)
            .into_iter()
            .filter(|r| is_media_file(&r.name) || r.url.contains("/folder/"))
            .take(limit)
            .collect()
    }

    async fn search_timfshare(client: &Client, query: &str, limit: usize) -> Vec<RawFshareResult> {
        let endpoints = [
            "https://api.timfshare.com/v1/string-query-search",
            "https://timfshare.com/api/v1/string-query-search",
            "https://api.timfshare.com/api/v1/string-query-search",
        ];
        let mut results = Vec::new();

        info!("Executing FHUB TimFShare source search: '{}'", query);
        for endpoint in endpoints {
            let url = format!("{}?query={}", endpoint, urlencoding::encode(query));
            let mut req = client.post(&url)
                .header("Referer", format!("https://timfshare.com/search?key={}", urlencoding::encode(query)))
                .header("Origin", "https://timfshare.com")
                .header("Content-Length", "0");
            if endpoint.contains("/v1/") {
                if let Ok(token) = env::var("FHUB_TIMFSHARE_TOKEN") {
                    if !token.trim().is_empty() {
                        req = req.header("Authorization", format!("Bearer {}", token.trim()));
                    }
                }
            }
            let resp = req.send().await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(data) = r.json::<Value>().await {
                        if let Some(arr) = data["data"].as_array() {
                            info!("FHUB TimFShare '{}' returned {} results", query, arr.len());
                            for item in arr.iter().take(limit) {
                                let name = item["name"].as_str().unwrap_or("Unknown").to_string();
                                let f_url = item["url"].as_str().unwrap_or("").to_string();
                                if !is_fshare_url(&f_url) { continue; }
                                results.push(RawFshareResult {
                                    name,
                                    url: f_url.clone(),
                                    fcode: extract_fshare_code(&f_url),
                                    size: item["size"].as_u64().unwrap_or(0),
                                    score: 0,
                                });
                            }
                            if !results.is_empty() { break; }
                        }
                    }
                }
                Ok(r) => warn!("FHUB TimFShare '{}' returned HTTP {} at {}", query, r.status(), endpoint),
                Err(e) => warn!("FHUB TimFShare '{}' request failed at {}: {}", query, endpoint, e),
            }
        }

        results
    }

    async fn search_fshare_vip(client: &Client, query: &str, limit: usize) -> Vec<RawFshareResult> {
        if limit == 0 { return Vec::new(); }

        let url = format!("https://fshare.vip/s.php?keyword={}", urlencoding::encode(query));
        info!("Executing FHUB VietMediaF/fshare.vip source search: '{}'", query);

        let resp = client.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36 VietMedia/1.0")
            .header("Accept", "application/json,text/plain,*/*")
            .header("Accept-Language", "vi-VN,vi;q=0.9,en;q=0.8")
            .header("Referer", "https://fshare.vip/")
            .send()
            .await;

        let data = match resp {
            Ok(r) if r.status().is_success() => match r.json::<Value>().await {
                Ok(data) => data,
                Err(e) => {
                    warn!("FHUB fshare.vip '{}' returned invalid JSON: {}", query, e);
                    return Vec::new();
                }
            },
            Ok(r) => {
                warn!("FHUB fshare.vip '{}' returned HTTP {}", query, r.status());
                return Vec::new();
            }
            Err(e) => {
                warn!("FHUB fshare.vip '{}' request failed: {}", query, e);
                return Vec::new();
            }
        };

        let items = match data.get("data").and_then(|v| v.as_array()) {
            Some(items) => items,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for item in items.iter().take(limit) {
            let name = item.get("Name").and_then(|v| v.as_str())
                .or_else(|| item.get("name").and_then(|v| v.as_str()))
                .unwrap_or("Unknown")
                .replace('@', "");
            let f_url = item.get("Link").and_then(|v| v.as_str())
                .or_else(|| item.get("url").and_then(|v| v.as_str()))
                .unwrap_or("")
                .split('?')
                .next()
                .unwrap_or("")
                .to_string();
            if !is_fshare_url(&f_url) { continue; }
            results.push(RawFshareResult {
                name,
                url: f_url.clone(),
                fcode: extract_fshare_code(&f_url),
                size: item.get("Size").and_then(|v| v.as_u64())
                    .or_else(|| item.get("size").and_then(|v| v.as_u64()))
                    .unwrap_or(0),
                score: 5,
            });
        }

        info!("FHUB fshare.vip '{}' returned {} usable results", query, results.len());
        results
    }

    async fn search_public_fshare_links(client: &Client, query: &str, limit: usize) -> Vec<RawFshareResult> {
        if limit == 0 { return Vec::new(); }

        // Use broad queries, not only `site:fshare.vn`: many useful FShare links are
        // embedded in forum/blog result pages (HDVietnam, ThuvienHD, etc.), and search
        // engines often do not expose the raw fshare.vn URL as the result URL.
        let search_queries = [
            format!("\"{}\" \"fshare.vn/file\"", query),
            format!("\"{}\" \"fshare.vn/folder\"", query),
            format!("\"{}\" fshare", query),
            format!("{} fshare hdvietnam", query),
            format!("{} fshare thuvienhd", query),
            format!("site:fshare.vn (file OR folder) \"{}\"", query),
        ];
        let urls: Vec<String> = search_queries.iter().flat_map(|search_query| [
            format!("https://www.google.com/search?q={}&num=20", urlencoding::encode(search_query)),
            format!("https://www.bing.com/search?q={}&count=20", urlencoding::encode(search_query)),
            format!("https://duckduckgo.com/html/?q={}", urlencoding::encode(search_query)),
        ]).collect();
        let link_re = match Regex::new(r#"https?://(?:www\.)?fshare\.vn/(?:file|folder)/[A-Za-z0-9]+(?:\?[^\"'<>\s&]*)?"#) {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };
        let redirect_re = match Regex::new(r#"[?&](?:q|u|url)=([^&\"'<>]+)"#) {
            Ok(re) => re,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for url in urls {
            let html = match client.get(&url)
                .header("User-Agent", "Mozilla/5.0 (compatible; FHub/1.0)")
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
                Ok(resp) => {
                    warn!("FHUB public FShare search '{}' returned HTTP {}", query, resp.status());
                    String::new()
                }
                Err(e) => {
                    warn!("FHUB public FShare search '{}' failed: {}", query, e);
                    String::new()
                }
            };

            for candidate in extract_fshare_urls_from_html(&html, &link_re, &redirect_re) {
                let fcode = extract_fshare_code(&candidate);
                if fcode.is_empty() || !seen.insert(fcode.clone()) { continue; }
                let kind = if candidate.contains("/folder/") { "Folder" } else { "File" };
                results.push(RawFshareResult {
                    name: format!("{} - {} FShare", query, kind),
                    url: candidate,
                    fcode,
                    size: 0,
                    score: -10,
                });
                if results.len() >= limit { return results; }
            }

            // Search engines often list intermediary pages rather than raw FShare URLs.
            // Visit a small top-result window and extract embedded FShare links from those pages.
            for page_url in extract_search_result_urls(&html, &redirect_re) {
                let page_html = match client.get(&page_url)
                    .header("User-Agent", "Mozilla/5.0 (compatible; FHub/1.0)")
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
                    _ => String::new(),
                };
                for candidate in extract_fshare_urls_from_html(&page_html, &link_re, &redirect_re) {
                    let fcode = extract_fshare_code(&candidate);
                    if fcode.is_empty() || !seen.insert(fcode.clone()) { continue; }
                    let kind = if candidate.contains("/folder/") { "Folder" } else { "File" };
                    results.push(RawFshareResult {
                        name: format!("{} - {} FShare", query, kind),
                        url: candidate,
                        fcode,
                        size: 0,
                        score: -20,
                    });
                    if results.len() >= limit { return results; }
                }
            }
        }

        results
    }

    /// Convert raw source results into FHUB-native source candidates.
    pub fn into_fhub_candidates(results: Vec<RawFshareResult>) -> Vec<FhubSourceCandidate> {
        results.into_iter().map(Into::into).collect()
    }

    /// Build a FHUB-native ingest plan from raw source search results.
    pub fn build_ingest_plan(
        title: impl Into<String>,
        media_type: impl Into<String>,
        results: Vec<RawFshareResult>,
    ) -> FhubIngestPlan {
        let mut plan = FhubIngestPlan::new(title, media_type);
        for candidate in Self::into_fhub_candidates(results) {
            plan.push_candidate(candidate);
        }
        plan
    }

    /// Deduplicate results by source code.
    #[allow(dead_code)]
    pub fn deduplicate_by_fcode(results: Vec<RawFshareResult>) -> Vec<RawFshareResult> {
        let mut seen = std::collections::HashSet::new();
        results.into_iter()
            .filter(|r| {
                let pure_fcode = r.fcode.split('?').next().unwrap_or(&r.fcode);
                seen.insert(pure_fcode.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_result_converts_to_fhub_candidate() {
        let raw = RawFshareResult {
            name: "Movie.2024.1080p.mkv".to_string(),
            url: "https://example.test/file/abc".to_string(),
            fcode: "abc".to_string(),
            size: 1024,
            score: 99,
        };

        let candidate = raw.into_fhub_candidate();
        assert_eq!(candidate.code, "abc");
        assert_eq!(candidate.title, "Movie.2024.1080p.mkv");
        assert_eq!(candidate.url, "https://example.test/file/abc");
        assert_eq!(candidate.size, 1024);
    }

    #[test]
    fn raw_result_supports_into_conversion() {
        let raw = RawFshareResult {
            name: "Native.2024.2160p.mkv".to_string(),
            url: "https://example.test/file/native".to_string(),
            fcode: "native".to_string(),
            size: 4096,
            score: 10,
        };

        let candidate: FhubSourceCandidate = raw.into();
        assert_eq!(candidate.code, "native");
        assert_eq!(candidate.size, 4096);
    }

    #[test]
    fn pipeline_converts_multiple_candidates() {
        let results = vec![
            RawFshareResult {
                name: "One.mkv".to_string(),
                url: "https://example.test/file/one".to_string(),
                fcode: "one".to_string(),
                size: 10,
                score: 0,
            },
            RawFshareResult {
                name: "Two.mp4".to_string(),
                url: "https://example.test/file/two".to_string(),
                fcode: "two".to_string(),
                size: 20,
                score: 0,
            },
        ];

        let candidates = SearchPipeline::into_fhub_candidates(results);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].code, "one");
        assert_eq!(candidates[1].size, 20);
    }

    #[test]
    fn pipeline_builds_ingest_plan() {
        let results = vec![
            RawFshareResult {
                name: "Film.1080p.mkv".to_string(),
                url: "https://example.test/file/film".to_string(),
                fcode: "film".to_string(),
                size: 100,
                score: 0,
            },
            RawFshareResult {
                name: "Film.2160p.mkv".to_string(),
                url: "https://example.test/file/film4k".to_string(),
                fcode: "film4k".to_string(),
                size: 300,
                score: 0,
            },
        ];

        let plan = SearchPipeline::build_ingest_plan("Film", "movie", results);
        assert_eq!(plan.title, "Film");
        assert_eq!(plan.media_type, "movie");
        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(plan.total_size(), 400);
    }
}
