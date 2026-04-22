use std::path::PathBuf;

use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use rustyip::{config, db, handlers, routes, updater};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
        )
        .init();

    let config = config::Config::from_env();
    let db_path = PathBuf::from(&config.db_path);

    // Create shared DB in an empty (not-yet-loaded) state so the server can
    // start immediately and respond to health checks while the database is
    // still being downloaded on first deploy.
    let shared_db = if db_path.exists() {
        let reader = match db::load_db(&db_path) {
            Ok(r) => r,
            Err(e) => {
                error!("failed to load database: {e}");
                std::process::exit(1);
            }
        };
        db::new_shared(reader)
    } else {
        info!("database file not found, server will start while download proceeds");
        db::new_empty()
    };

    let cancel = CancellationToken::new();

    let site_domain: std::sync::Arc<str> = config.site_domain.into();
    let ipv4_domain: std::sync::Arc<str> = config.ipv4_domain.into();
    let openapi_json = handlers::build_openapi_json(&site_domain);

    let state = handlers::AppState {
        db: shared_db.clone(),
        site_domain,
        ipv4_domain,
        dev_mode: config.dev_mode,
        openapi_json,
    };

    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(config.listen_addr)
        .await
        .expect("failed to bind listener");

    // Spawn background database initialization and periodic updater.
    let init_db = shared_db.clone();
    let init_path = db_path.clone();
    let init_url = config.db_update_url.clone();
    let init_cancel = cancel.clone();
    let updater_handle = tokio::spawn(async move {
        // Only download if the DB was not loaded synchronously above.
        if !db::is_ready(&init_db)
            && let Err(e) = updater::init_db(&init_db, &init_path, &init_url).await
        {
            error!("failed to initialize database: {e}");
            std::process::exit(1);
        }

        // Start periodic updater after the initial load succeeds.
        updater::run_updater(
            init_db,
            init_path,
            init_url,
            config.db_update_time_utc,
            config.db_update_interval_hours,
            init_cancel,
        )
        .await;
    });

    info!("listening on {}", config.listen_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    info!("server stopped, cleaning up background tasks");
    cancel.cancel();
    let _ = updater_handle.await;
    info!("shutdown complete");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => info!("received SIGINT, shutting down"),
        () = terminate => info!("received SIGTERM, shutting down"),
    }
}
