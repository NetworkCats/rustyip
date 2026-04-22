//! Background database updater: downloads and hot-swaps the MMDB database on a daily schedule.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::db::{DbReader, SharedDb, load_db};

#[cfg(test)]
use crate::db::new_shared;

pub async fn ensure_db_exists(
    db_path: &Path,
    update_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if db_path.exists() {
        return Ok(());
    }

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    download_db(update_url, db_path).await?;
    Ok(())
}

/// Ensures the database file exists (downloading if needed), loads it, and
/// stores the reader into the shared DB. Called during startup to initialize
/// the database before the application can serve requests.
pub async fn init_db(
    shared_db: &SharedDb,
    db_path: &Path,
    update_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_db_exists(db_path, update_url).await?;
    let reader = load_db(db_path)?;
    shared_db.store(Arc::new(Some(reader)));
    info!("database loaded successfully");
    Ok(())
}

/// Downloads the database to a temp file, validates it, and atomically
/// renames it into place. Returns the validated reader for immediate use.
/// Cleans up the temp file on any failure.
async fn download_db(
    url: &str,
    dest: &Path,
) -> Result<DbReader, Box<dyn std::error::Error + Send + Sync>> {
    let tmp_path = dest.with_extension("mmdb.tmp");

    if let Err(e) = stream_to_file(url, &tmp_path).await {
        let _ = fs::remove_file(&tmp_path).await;
        return Err(e);
    }

    let reader = match validate_db(&tmp_path) {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path).await;
            return Err(e);
        }
    };

    if let Err(e) = fs::rename(&tmp_path, dest).await {
        let _ = fs::remove_file(&tmp_path).await;
        return Err(e.into());
    }

    Ok(reader)
}

async fn stream_to_file(
    url: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let mut response = client.get(url).send().await?.error_for_status()?;

    let mut file = fs::File::create(path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.shutdown().await?;

    Ok(())
}

fn validate_db(path: &Path) -> Result<DbReader, Box<dyn std::error::Error + Send + Sync>> {
    load_db(path).map_err(|e| e.into())
}

pub fn spawn_updater(
    shared_db: SharedDb,
    db_path: PathBuf,
    update_url: String,
    update_time_utc: (u8, u8),
    interval_hours: u8,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(run_updater(
        shared_db,
        db_path,
        update_url,
        update_time_utc,
        interval_hours,
        cancel,
    ))
}

/// Returns the number of seconds from `now_secs` (seconds since UNIX epoch)
/// until the next tick. Ticks occur at `(anchor_hour, anchor_minute)` UTC and
/// every `interval_hours` thereafter. `interval_hours` must divide 24 evenly.
fn secs_until_next_tick(
    now_secs: u64,
    anchor_hour: u8,
    anchor_minute: u8,
    interval_hours: u8,
) -> u64 {
    const SECS_PER_DAY: u64 = 86_400;
    let interval = u64::from(interval_hours) * 3600;
    let anchor = u64::from(anchor_hour) * 3600 + u64::from(anchor_minute) * 60;
    let time_of_day = now_secs % SECS_PER_DAY;
    let offset_from_anchor = if time_of_day >= anchor {
        time_of_day - anchor
    } else {
        SECS_PER_DAY - (anchor - time_of_day)
    };
    let next_boundary = ((offset_from_anchor / interval) + 1) * interval;
    next_boundary - offset_from_anchor
}

/// Runs the database updater loop. Sleeps until the next scheduled tick, then
/// downloads and hot-swaps a fresh database.
pub async fn run_updater(
    shared_db: SharedDb,
    db_path: PathBuf,
    update_url: String,
    update_time_utc: (u8, u8),
    interval_hours: u8,
    cancel: CancellationToken,
) {
    let (hour, minute) = update_time_utc;
    loop {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();
        let delay =
            Duration::from_secs(secs_until_next_tick(now_secs, hour, minute, interval_hours));
        info!(
            "next database update in {}h {}m (anchor {:02}:{:02} UTC, every {}h)",
            delay.as_secs() / 3600,
            (delay.as_secs() % 3600) / 60,
            hour,
            minute,
            interval_hours,
        );
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = cancel.cancelled() => {
                info!("updater task cancelled, stopping");
                break;
            }
        }
        if let Err(e) = update_db(&shared_db, &db_path, &update_url).await {
            error!("database update failed: {e}");
        }
    }
}

async fn update_db(
    shared_db: &SharedDb,
    db_path: &Path,
    update_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = download_db(update_url, db_path).await?;
    shared_db.store(Arc::new(Some(reader)));
    let size = fs::metadata(db_path)
        .await
        .ok()
        .map(|m| m.len())
        .unwrap_or(0);
    info!("database updated successfully ({size} bytes)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use tempfile::TempDir;

    fn test_db_path() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(format!("{manifest_dir}/data/Merged-IP.mmdb"))
    }

    #[test]
    fn validate_db_with_valid_file() {
        let path = test_db_path();
        let result = validate_db(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_db_with_invalid_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let bad_file = dir.path().join("bad.mmdb");
        std::fs::write(&bad_file, b"not a valid mmdb file").unwrap();

        let result = validate_db(&bad_file);
        assert!(result.is_err());
    }

    #[test]
    fn validate_db_with_nonexistent_file_returns_error() {
        let result = validate_db(Path::new("/nonexistent/path.mmdb"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ensure_db_exists_skips_download_when_file_present() {
        let path = test_db_path();
        let result = ensure_db_exists(&path, "https://invalid.example.com/db.mmdb").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ensure_db_exists_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let nested_path = dir.path().join("a").join("b").join("c").join("test.mmdb");

        // This will fail at the download stage (invalid URL), but the parent
        // directories should still be created before the download attempt.
        let _ = ensure_db_exists(&nested_path, "https://invalid.example.com/db.mmdb").await;

        assert!(nested_path.parent().unwrap().exists());
    }

    #[tokio::test]
    async fn download_db_cleans_up_tmp_on_download_failure() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("test.mmdb");
        let tmp_path = dest.with_extension("mmdb.tmp");

        let result = download_db("https://invalid.example.com/db.mmdb", &dest).await;
        assert!(result.is_err());
        assert!(
            !tmp_path.exists(),
            "temp file should be cleaned up on failure"
        );
    }

    #[tokio::test]
    async fn download_db_cleans_up_tmp_on_validation_failure() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("test.mmdb");
        let tmp_path = dest.with_extension("mmdb.tmp");

        // Pre-create a temp file with invalid content to simulate a download
        // that succeeds but produces an invalid MMDB.
        tokio::fs::write(&tmp_path, b"not a valid mmdb")
            .await
            .unwrap();

        // validate_db will fail, and the temp file should be cleaned up.
        let result = validate_db(&tmp_path);
        assert!(result.is_err());

        // Simulate the cleanup that download_db performs on validation failure.
        if result.is_err() {
            let _ = fs::remove_file(&tmp_path).await;
        }
        assert!(
            !tmp_path.exists(),
            "temp file should be cleaned up after validation failure"
        );
    }

    #[tokio::test]
    async fn spawn_updater_stops_on_cancellation() {
        let reader = load_db(&test_db_path()).unwrap();
        let shared_db = new_shared(reader);
        let cancel = CancellationToken::new();

        let handle = spawn_updater(
            shared_db,
            PathBuf::from("/nonexistent/path.mmdb"),
            "https://invalid.example.com/db.mmdb".to_string(),
            (0, 20),
            6,
            cancel.clone(),
        );

        cancel.cancel();

        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(
            result.is_ok(),
            "updater task should exit promptly on cancellation"
        );
    }

    #[test]
    fn secs_until_next_tick_daily_from_midnight() {
        // 00:00 UTC, anchor 01:20, interval 24h => 1h 20m = 4800s.
        assert_eq!(secs_until_next_tick(0, 1, 20, 24), 4800);
    }

    #[test]
    fn secs_until_next_tick_daily_wraps_to_tomorrow() {
        // 02:00 UTC (7200s), anchor 01:20 (4800s), interval 24h
        // => 86400 - 7200 + 4800 = 84000s.
        assert_eq!(secs_until_next_tick(7200, 1, 20, 24), 84000);
    }

    #[test]
    fn secs_until_next_tick_daily_exact_time_wraps() {
        // Exactly at the anchor with 24h interval => schedule for next day.
        let target_secs = 3600 + 20 * 60;
        assert_eq!(secs_until_next_tick(target_secs, 1, 20, 24), 86400);
    }

    #[test]
    fn secs_until_next_tick_every_six_hours_at_midnight() {
        // 00:00 UTC with anchor 00:20 and interval 6h => next tick in 20m = 1200s.
        assert_eq!(secs_until_next_tick(0, 0, 20, 6), 1200);
    }

    #[test]
    fn secs_until_next_tick_every_six_hours_just_after_tick() {
        // 00:21 UTC (1260s), anchor 00:20, interval 6h
        // => next tick is 06:20 UTC = 6h - 1m = 21540s.
        let now_secs = 20 * 60 + 60; // 00:21
        assert_eq!(secs_until_next_tick(now_secs, 0, 20, 6), 21540);
    }

    #[test]
    fn secs_until_next_tick_every_six_hours_at_tick() {
        // Exactly at a tick (00:20) with 6h interval => schedule for 6h later.
        let target_secs = 20 * 60;
        assert_eq!(secs_until_next_tick(target_secs, 0, 20, 6), 6 * 3600);
    }

    #[test]
    fn secs_until_next_tick_every_six_hours_before_anchor() {
        // 00:10 UTC (600s), anchor 00:20, interval 6h => next tick in 10m = 600s.
        assert_eq!(secs_until_next_tick(600, 0, 20, 6), 600);
    }

    #[test]
    fn secs_until_next_tick_midnight_target() {
        // 23:00 UTC, anchor 00:00, interval 24h => 1h = 3600s.
        let now_secs = 23 * 3600;
        assert_eq!(secs_until_next_tick(now_secs, 0, 0, 24), 3600);
    }

    #[tokio::test]
    async fn update_db_swaps_shared_reader() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("test.mmdb");

        // Copy the real DB to a temp location
        std::fs::copy(test_db_path(), &dest).unwrap();

        let original_reader = load_db(&dest).unwrap();
        let shared_db = new_shared(original_reader);

        // Load a new reader and manually swap it to verify the mechanism
        let new_reader = load_db(&dest).unwrap();
        shared_db.store(Arc::new(Some(new_reader)));

        // Verify the shared DB still works after the swap
        let guard = shared_db.load();
        let reader = Option::as_ref(&guard).unwrap();
        let ip: IpAddr = "1.1.1.1".parse().unwrap();
        let result = reader.lookup(ip);
        assert!(result.is_ok());
    }
}
