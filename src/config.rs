use std::env;
use std::net::SocketAddr;

pub struct Config {
    pub listen_addr: SocketAddr,
    pub db_path: String,
    pub db_update_url: String,
    pub db_update_interval_hours: u64,
    pub site_domain: String,
}

impl Config {
    pub fn from_env() -> Self {
        let listen_addr = env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()
            .expect("LISTEN_ADDR must be a valid socket address");

        let db_path = env::var("DB_PATH").unwrap_or_else(|_| "data/Merged-IP.mmdb".to_string());

        let db_update_url = env::var("DB_UPDATE_URL").unwrap_or_else(|_| {
            "https://github.com/NetworkCats/Merged-IP-Data/releases/latest/download/Merged-IP.mmdb"
                .to_string()
        });

        let db_update_interval_hours = env::var("DB_UPDATE_INTERVAL_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .expect("DB_UPDATE_INTERVAL_HOURS must be a valid integer");

        let site_domain = env::var("SITE_DOMAIN").unwrap_or_else(|_| "localhost".to_string());

        Self {
            listen_addr,
            db_path,
            db_update_url,
            db_update_interval_hours,
            site_domain,
        }
    }
}
