//! Simple Download Engine
//!
//! Uses custom multi-range downloads for Fshare direct URLs when possible,
//! with reqwest single-stream fallback for compatibility.

use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use futures_util::StreamExt;

use super::config::DownloadConfig;
use super::progress::DownloadProgress;

pub struct SimpleDownloadEngine {
    http_client: reqwest::Client,
    config: DownloadConfig,
}

impl SimpleDownloadEngine {
    pub fn with_config(config: DownloadConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .pool_max_idle_per_host(10)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { http_client, config }
    }

    pub async fn download_file<F>(
        &self,
        url: &str,
        destination: &Path,
        progress_callback: F,
        cancel_token: &CancellationToken,
    ) -> anyhow::Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let temp_destination = destination.with_extension(
            format!("{}.fhub", destination.extension().and_then(|e| e.to_str()).unwrap_or(""))
        );

        if self.should_use_multi_range(url).await {
            tracing::info!("Starting multi-range download: {} -> {:?}", url, temp_destination);
            match self.download_with_multi_range(url, &temp_destination, &progress_callback, cancel_token).await {
                Ok(total_downloaded) => {
                    tracing::info!("Multi-range download completed: {} bytes", total_downloaded);
                    if temp_destination.exists() {
                        tokio::fs::rename(&temp_destination, destination).await?;
                        tracing::info!("Renamed {:?} -> {:?}", temp_destination, destination);
                    }
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Multi-range path failed, falling back to single-stream: {}", e);
                }
            }
        }

        tracing::info!("Starting single-stream download: {} -> {:?}", url, temp_destination);
        let total_downloaded = self.download_single_stream(
            url,
            &temp_destination,
            cancel_token,
            progress_callback,
        ).await?;

        tracing::info!("Download completed: {} bytes", total_downloaded);
        if temp_destination.exists() {
            tokio::fs::rename(&temp_destination, destination).await?;
            tracing::info!("Renamed {:?} -> {:?}", temp_destination, destination);
        }

        Ok(())
    }

    async fn should_use_multi_range(&self, url: &str) -> bool {
        if !url.contains("fshare.vn") {
            return false;
        }
        match Command::new("curl").arg("--version").output().await {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    async fn download_with_multi_range<F>(
        &self,
        url: &str,
        destination: &PathBuf,
        progress_callback: &F,
        cancel_token: &CancellationToken,
    ) -> anyhow::Result<u64>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let initial_bytes = if destination.exists() {
            tokio::fs::metadata(destination).await?.len()
        } else {
            0
        };

        let head = self.http_client.head(url).send().await?;
        if !head.status().is_success() {
            anyhow::bail!("HEAD failed for multi-range path: {}", head.status());
        }
        let total_size = head
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let accept_ranges = head
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        if accept_ranges != "bytes" {
            anyhow::bail!("Server does not advertise byte ranges");
        }

        let dir = destination.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let out = destination.file_name().and_then(|s| s.to_str()).unwrap_or("download.fhub").to_string();
        let parts_dir = dir.join(format!(".{}.parts", out));
        let segments = self.config.segments_per_download.clamp(1, 32);
        let chunk_size = total_size.div_ceil(segments as u64);

        tracing::info!(
            "Custom multi-range tuned for Fshare: segments={}, dir={:?}, out={}",
            segments,
            dir,
            out
        );

        tokio::fs::create_dir_all(&parts_dir).await?;
        let _ = tokio::fs::remove_file(destination).await;

        let mut script = String::from("set -eu\n");
        script.push_str("rm -f \"$DEST\"\n");
        script.push_str("rm -f \"$PARTS_DIR\"/part-* \"$PARTS_DIR\"/part-*.partial 2>/dev/null || true\n");
        script.push_str("PIDS=\"\"\n");
        for i in 0..segments {
            let start = i as u64 * chunk_size;
            if start >= total_size {
                break;
            }
            let end = ((i as u64 + 1) * chunk_size).saturating_sub(1).min(total_size.saturating_sub(1));
            let expected = end.saturating_sub(start).saturating_add(1);
            script.push_str(&format!(
                "(curl -L --fail --silent --show-error --retry 5 --retry-delay 2 --retry-all-errors --connect-timeout 30 --range {start}-{end} -o \"$PARTS_DIR/part-{i:02}.partial\" \"$URL\" && test \"$(stat -c %s \"$PARTS_DIR/part-{i:02}.partial\")\" -eq {expected} && mv \"$PARTS_DIR/part-{i:02}.partial\" \"$PARTS_DIR/part-{i:02}\") &\n"
            ));
            script.push_str("PIDS=\"$PIDS $!\"\n");
        }
        script.push_str("FAIL=0\n");
        script.push_str("for PID in $PIDS; do\n  if ! wait \"$PID\"; then FAIL=1; fi\ndone\n");
        script.push_str("test \"$FAIL\" -eq 0\n");
        script.push_str("");
        for i in 0..segments {
            let start = i as u64 * chunk_size;
            if start >= total_size {
                break;
            }
            let end = ((i as u64 + 1) * chunk_size).saturating_sub(1).min(total_size.saturating_sub(1));
            let expected = end.saturating_sub(start).saturating_add(1);
            script.push_str(&format!(
                "test -f \"$PARTS_DIR/part-{i:02}\" && test \"$(stat -c %s \"$PARTS_DIR/part-{i:02}\")\" -eq {expected}\n"
            ));
        }
        script.push_str("cat");
        for i in 0..segments {
            let start = i as u64 * chunk_size;
            if start >= total_size {
                break;
            }
            script.push_str(&format!(" \"$PARTS_DIR/part-{i:02}\""));
        }
        script.push_str(" > \"$DEST\"\n");
        script.push_str(&format!("test \"$(stat -c %s \"$DEST\")\" -eq {}\n", total_size));

        let mut child = Command::new("sh")
            .arg("-lc")
            .arg(script)
            .env("URL", url)
            .env("DEST", destination)
            .env("PARTS_DIR", &parts_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let start_time = Instant::now();
        loop {
            if cancel_token.is_cancelled() {
                let _ = child.kill().await;
                anyhow::bail!("Download cancelled");
            }

            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    anyhow::bail!("multi-range exited with status {}", status);
                }
                break;
            }

            let mut downloaded = 0u64;
            let mut read_dir = tokio::fs::read_dir(&parts_dir).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                if let Ok(meta) = entry.metadata().await {
                    downloaded = downloaded.saturating_add(meta.len());
                }
            }
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (downloaded.saturating_sub(initial_bytes)) as f64 / elapsed
            } else {
                0.0
            };
            let eta = if speed > 0.0 && total_size > downloaded {
                (total_size - downloaded) as f64 / speed
            } else {
                0.0
            };
            let percentage = if total_size > 0 {
                ((downloaded.min(total_size)) as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };

            progress_callback(DownloadProgress {
                downloaded_bytes: downloaded.min(total_size),
                total_bytes: total_size,
                speed_bytes_per_sec: speed,
                eta_seconds: eta,
                percentage,
                initial_bytes,
            });

            sleep(Duration::from_millis(500)).await;
        }

        let downloaded = tokio::fs::metadata(destination).await?.len();
        let _ = tokio::fs::remove_dir_all(&parts_dir).await;
        progress_callback(DownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total_size.max(downloaded),
            speed_bytes_per_sec: 0.0,
            eta_seconds: 0.0,
            percentage: 100.0,
            initial_bytes,
        });

        Ok(downloaded)
    }

    async fn download_single_stream<F>(
        &self,
        url: &str,
        destination: &PathBuf,
        cancel_token: &CancellationToken,
        progress_callback: F,
    ) -> anyhow::Result<u64>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let initial_bytes = if destination.exists() {
            tokio::fs::metadata(&destination).await?.len()
        } else {
            0
        };

        let mut request = self.http_client.get(url);
        if initial_bytes > 0 {
            request = request.header("Range", format!("bytes={}-", initial_bytes));
            tracing::info!("Resuming download from byte {}", initial_bytes);
        }

        let response = request.send().await?;
        let status = response.status();
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            tracing::info!("File already complete (416)");
            return Ok(initial_bytes);
        }
        if !status.is_success() {
            anyhow::bail!("HTTP error: {}", status);
        }

        let content_length = response.content_length().unwrap_or(0);
        let total_size = if initial_bytes > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
            initial_bytes + content_length
        } else {
            content_length
        };

        let resume_position = if initial_bytes > 0 && status == reqwest::StatusCode::OK {
            tracing::warn!("Server ignored Range header, starting from beginning");
            0
        } else {
            initial_bytes
        };

        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(resume_position == 0)
            .append(resume_position > 0)
            .open(&destination)
            .await?;

        let mut downloaded = resume_position;
        let start_time = Instant::now();
        let mut stream = response.bytes_stream();
        let mut last_progress_update = Instant::now();
        let progress_interval = std::time::Duration::from_millis(250);

        while let Some(chunk_result) = stream.next().await {
            if cancel_token.is_cancelled() {
                anyhow::bail!("Download cancelled");
            }

            let chunk = chunk_result?;
            let chunk_len = chunk.len() as u64;
            file.write_all(&chunk).await?;
            downloaded += chunk_len;

            if last_progress_update.elapsed() >= progress_interval {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (downloaded - resume_position) as f64 / elapsed
                } else {
                    0.0
                };
                let eta = if speed > 0.0 && total_size > downloaded {
                    (total_size - downloaded) as f64 / speed
                } else {
                    0.0
                };
                let percentage = if total_size > 0 {
                    (downloaded as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                };

                progress_callback(DownloadProgress {
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    speed_bytes_per_sec: speed,
                    eta_seconds: eta,
                    percentage,
                    initial_bytes: resume_position,
                });

                last_progress_update = Instant::now();
            }
        }

        file.flush().await?;
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            (downloaded - resume_position) as f64 / elapsed
        } else {
            0.0
        };

        progress_callback(DownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total_size,
            speed_bytes_per_sec: speed,
            eta_seconds: 0.0,
            percentage: 100.0,
            initial_bytes: resume_position,
        });

        Ok(downloaded)
    }
}
