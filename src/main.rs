use std::path::PathBuf;

use tracing::error;
use tracing_subscriber::EnvFilter;

use rustyip::{config, db, handlers, routes, updater};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .init();

    let config = config::Config::from_env();
    let db_path = PathBuf::from(&config.db_path);

    if let Err(e) = updater::ensure_db_exists(&db_path, &config.db_update_url).await {
        error!("failed to initialize database: {e}");
        std::process::exit(1);
    }

    let reader = match db::load_db(&db_path) {
        Ok(r) => r,
        Err(e) => {
            error!("failed to load database: {e}");
            std::process::exit(1);
        }
    };

    let shared_db = db::new_shared(reader);

    updater::spawn_updater(
        shared_db.clone(),
        db_path,
        config.db_update_url,
        config.db_update_interval_hours,
    );

    let state = handlers::AppState {
        db: shared_db,
        site_domain: config.site_domain.into(),
        dev_mode: config.dev_mode,
    };

    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(config.listen_addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
}
