//! Path Builder Module
//!
//! Handles destination path construction for downloads based on TMDB metadata.
//! Extracted from orchestrator.rs for better modularity.

use std::path::Path;

/// TMDB metadata for organizing downloads into folder structures
#[derive(Debug, Clone)]
pub struct TmdbDownloadMetadata {
    /// TMDB ID
    #[allow(dead_code)]
    pub tmdb_id: Option<i64>,
    /// Media type: "movie" or "tv"
    #[allow(dead_code)]
    pub media_type: Option<String>,
    /// Movie/Show title
    pub title: Option<String>,
    /// Release year
    pub year: Option<i32>,
    /// Collection name (for movies in a collection)
    pub collection_name: Option<String>,
    /// Season number (for TV)
    #[allow(dead_code)]
    pub season: Option<i32>,
    /// Episode number (for TV)
    #[allow(dead_code)]
    pub episode: Option<i32>,
}

/// Path builder for download destinations
pub struct PathBuilder;

impl PathBuilder {
    /// Build organized destination path based on TMDB metadata
    /// Movies: collection_name/movie_name (year)/file OR movie_name (year)/file
    /// TV: series_name/season_xx/file
    pub fn build_destination_path(
        filename: &str, 
        category: &str, 
        tmdb: &Option<TmdbDownloadMetadata>, 
        root_dir: &Path
    ) -> String {
        let movies_root = std::env::var("FHUB_MOVIES_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root_dir.join("Movies"));
        let shows_root = std::env::var("FHUB_SHOWS_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root_dir.join("Shows"));
        
        if let Some(meta) = tmdb {
            let media_type = meta.media_type.as_deref().unwrap_or(category);
            
            match media_type {
                "movie" => {
                    let base_dir = movies_root.as_path();
                    // Build: [Collection]/MovieName (Year)/filename
                    let movie_folder = if let Some(ref title) = meta.title {
                        if let Some(ref year) = meta.year {
                            format!("{} ({})", Self::sanitize_filename(title), year)
                        } else {
                            Self::sanitize_filename(title)
                        }
                    } else {
                        "Unknown Movie".to_string()
                    };
                    
                    if let Some(ref collection) = meta.collection_name {
                        base_dir.join(Self::sanitize_filename(collection))
                            .join(&movie_folder)
                            .join(filename)
                            .to_string_lossy()
                            .to_string()
                    } else {
                        base_dir.join(&movie_folder)
                            .join(filename)
                            .to_string_lossy()
                            .to_string()
                    }
                }
                "tv" => {
                    let base_dir = shows_root.as_path();
                    // Build: SeriesName/Season XX/filename
                    let series_folder = if let Some(ref title) = meta.title {
                        Self::sanitize_filename(title)
                    } else {
                        "Unknown Series".to_string()
                    };
                    
                    let season_folder = if let Some(season) = meta.season {
                        format!("Season {:02}", season)
                    } else {
                        "Season 01".to_string()
                    };
                    
                    base_dir.join(&series_folder)
                        .join(&season_folder)
                        .join(filename)
                        .to_string_lossy()
                        .to_string()
                }
                _ => {
                    // Default: just use root dir
                    root_dir.join(filename).to_string_lossy().to_string()
                }
            }
        } else {
            // No TMDB metadata: route by explicit category when available.
            let base_dir = match category {
                "movie" | "movies" => movies_root.as_path(),
                "tv" | "show" | "shows" | "series" => shows_root.as_path(),
                _ => root_dir,
            };
            base_dir.join(filename).to_string_lossy().to_string()
        }
    }
    
    /// Sanitize a string for use as a folder/file name
    pub fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }
}
