use std::sync::Arc;

use anyhow::Result;

use crate::config::FHubConfig;
use crate::db::Db;
use crate::downloader::task::DownloadTask;

#[derive(Clone)]
pub struct FHubClient;

#[derive(Debug, Clone)]
pub struct FHubMovie {
    pub id: i32,
    pub has_file: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct FHubEpisode {
    pub has_file: bool,
}

impl FHubClient {
    pub fn new(_tv: Option<FHubConfig>, _movie: Option<FHubConfig>) -> Self {
        Self
    }

    pub async fn get_movie_by_tmdb(&self, _tmdb_id: i64) -> Result<Option<FHubMovie>> { Ok(None) }
    pub async fn series_exists(&self, _tmdb_id: i64) -> Result<Option<i32>> { Ok(None) }
    pub async fn get_episode_by_details(&self, _series_id: i32, _season_number: i32, _episode_number: i32) -> Result<Option<FHubEpisode>> { Ok(None) }
    pub async fn get_series_path(&self, _series_id: i64) -> Result<String> { Ok("/downloads/tv".to_string()) }
    pub async fn movie_exists(&self, _tmdb_id: i64) -> Result<Option<i32>> { Ok(None) }
    pub async fn get_movie_path(&self, _movie_id: i64) -> Result<String> { Ok("/downloads/movies".to_string()) }
    pub async fn trigger_series_rescan(&self, _series_id: i64) -> Result<()> { Ok(()) }
    pub async fn trigger_movie_refresh(&self, _movie_id: i64) -> Result<()> { Ok(()) }
}

#[derive(Clone)]
pub struct FHubArtifactManager;

impl FHubArtifactManager {
    pub fn new(_client: Arc<FHubClient>, _db: Arc<Db>) -> Self {
        Self
    }

    pub async fn manage_artifact(&self, _task: &DownloadTask) -> Result<ArtifactStatus> {
        Ok(ArtifactStatus::Skipped {
            reason: "FHub standalone mode: external library integration disabled".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ArtifactStatus {
    Created { arr_id: i32 },
    AlreadyMonitored { arr_id: i32 },
    Skipped { reason: String },
    Failed { error: String },
}
