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
    let cancel = CancellationToken::new();

    let updater_handle = updater::spawn_updater(
        shared_db.clone(),
        db_path,
        config.db_update_url,
        config.db_update_interval_hours,
        cancel.clone(),
    );

    let site_domain: std::sync::Arc<str> = config.site_domain.into();
    let openapi_json = handlers::build_openapi_json(&site_domain);

    let state = handlers::AppState {
        db: shared_db,
        site_domain,
        dev_mode: config.dev_mode,
        openapi_json,
    };

    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(config.listen_addr)
        .await
        .expect("failed to bind listener");

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
