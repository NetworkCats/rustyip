use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::error;

use crate::db::{DbReader, SharedDb, load_db};

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

    download_db(update_url, db_path).await
}

async fn download_db(
    url: &str,
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tmp_path = dest.with_extension("mmdb.tmp");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let response = client.get(url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;

    let mut file = fs::File::create(&tmp_path).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;
    drop(file);

    validate_db(&tmp_path)?;
    fs::rename(&tmp_path, dest).await?;

    Ok(())
}

fn validate_db(path: &Path) -> Result<DbReader, Box<dyn std::error::Error + Send + Sync>> {
    load_db(path).map_err(|e| e.into())
}

pub fn spawn_updater(
    shared_db: SharedDb,
    db_path: PathBuf,
    update_url: String,
    interval_hours: u64,
) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(interval_hours * 3600);
        loop {
            tokio::time::sleep(interval).await;
            if let Err(e) = update_db(&shared_db, &db_path, &update_url).await {
                error!("database update failed: {e}");
            }
        }
    });
}

async fn update_db(
    shared_db: &SharedDb,
    db_path: &Path,
    update_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    download_db(update_url, db_path).await?;
    let reader = load_db(db_path)?;
    shared_db.store(Arc::new(reader));
    Ok(())
}
