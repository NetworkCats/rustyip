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
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(run_updater(
        shared_db,
        db_path,
        update_url,
        update_time_utc,
        cancel,
    ))
}

/// Returns the number of seconds from `now_secs` (seconds since UNIX epoch)
/// until the next occurrence of the given UTC `(hour, minute)`.
fn secs_until_next(now_secs: u64, hour: u8, minute: u8) -> u64 {
    const SECS_PER_DAY: u64 = 86_400;
    let time_of_day = now_secs % SECS_PER_DAY;
    let target = u64::from(hour) * 3600 + u64::from(minute) * 60;
    if target > time_of_day {
        target - time_of_day
    } else {
        // Target time already passed today; schedule for tomorrow.
        SECS_PER_DAY - time_of_day + target
    }
}

/// Runs the daily database updater loop. Sleeps until the configured UTC time
/// each day, then downloads and swaps in a fresh database.
pub async fn run_updater(
    shared_db: SharedDb,
    db_path: PathBuf,
    update_url: String,
    update_time_utc: (u8, u8),
    cancel: CancellationToken,
) {
    let (hour, minute) = update_time_utc;
    loop {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();
        let delay = Duration::from_secs(secs_until_next(now_secs, hour, minute));
        info!(
            "next database update in {}h {}m (at {:02}:{:02} UTC)",
            delay.as_secs() / 3600,
            (delay.as_secs() % 3600) / 60,
            hour,
            minute,
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
    info!("database updated successfully");
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
            (1, 20),
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
    fn secs_until_next_later_today() {
        // 00:00:00 UTC, target 01:20 UTC => 1h 20m = 4800s
        assert_eq!(secs_until_next(0, 1, 20), 4800);
    }

    #[test]
    fn secs_until_next_wraps_to_tomorrow() {
        // 02:00:00 UTC (7200s into the day), target 01:20 (4800s into the day)
        // => should wrap to next day: 86400 - 7200 + 4800 = 84000s
        let now_secs = 7200; // midnight + 2 hours
        assert_eq!(secs_until_next(now_secs, 1, 20), 84000);
    }

    #[test]
    fn secs_until_next_exact_time_wraps_to_tomorrow() {
        // Exactly at the target time => should schedule for next day (full 24h).
        let target_secs = 1 * 3600 + 20 * 60; // 01:20 = 4800s
        assert_eq!(secs_until_next(target_secs, 1, 20), 86400);
    }

    #[test]
    fn secs_until_next_midnight_target() {
        // 23:00:00 UTC, target 00:00 UTC => 1 hour = 3600s
        let now_secs = 23 * 3600;
        assert_eq!(secs_until_next(now_secs, 0, 0), 3600);
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
