use std::env;
use std::net::SocketAddr;

pub struct Config {
    pub listen_addr: SocketAddr,
    pub db_path: String,
    pub db_update_url: String,
    /// Daily UTC time to check for database updates, as (hour, minute).
    pub db_update_time_utc: (u8, u8),
    pub site_domain: String,
    pub ipv4_domain: String,
    pub dev_mode: bool,
}

/// Parses a `"HH:MM"` string into `(hour, minute)`, panicking on invalid input.
fn parse_time_utc(s: &str) -> (u8, u8) {
    let parts: Vec<&str> = s.split(':').collect();
    assert!(
        parts.len() == 2,
        "DB_UPDATE_TIME_UTC must be in HH:MM format"
    );
    let hour: u8 = parts[0]
        .parse()
        .expect("DB_UPDATE_TIME_UTC hour must be a valid integer");
    let minute: u8 = parts[1]
        .parse()
        .expect("DB_UPDATE_TIME_UTC minute must be a valid integer");
    assert!(hour < 24, "DB_UPDATE_TIME_UTC hour must be 0-23");
    assert!(minute < 60, "DB_UPDATE_TIME_UTC minute must be 0-59");
    (hour, minute)
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

        let db_update_time_utc =
            parse_time_utc(&env::var("DB_UPDATE_TIME_UTC").unwrap_or_else(|_| "01:20".to_string()));

        let site_domain = env::var("SITE_DOMAIN").unwrap_or_else(|_| "localhost".to_string());

        let ipv4_domain = env::var("IPV4_DOMAIN").unwrap_or_default();

        let dev_mode = env::var("DEV_MODE")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");

        Self {
            listen_addr,
            db_path,
            db_update_url,
            db_update_time_utc,
            site_domain,
            ipv4_domain,
            dev_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variable tests must run serially because they mutate global state.
    // We recover from poisoned mutexes since panics in should_panic tests are expected.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    const CONFIG_VARS: &[&str] = &[
        "LISTEN_ADDR",
        "DB_PATH",
        "DB_UPDATE_URL",
        "DB_UPDATE_TIME_UTC",
        "SITE_DOMAIN",
        "IPV4_DOMAIN",
        "DEV_MODE",
    ];

    // SAFETY: These env var helpers are only called while holding ENV_LOCK,
    // ensuring no concurrent mutation of the process environment.
    unsafe fn clear_config_vars() {
        for var in CONFIG_VARS {
            unsafe { env::remove_var(var) };
        }
    }

    unsafe fn set_var(key: &str, value: &str) {
        unsafe { env::set_var(key, value) };
    }

    unsafe fn remove_var(key: &str) {
        unsafe { env::remove_var(key) };
    }

    #[test]
    fn defaults_when_no_env_vars() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held, serializing access to env vars.
        unsafe { clear_config_vars() };

        let config = Config::from_env();
        assert_eq!(config.listen_addr.to_string(), "0.0.0.0:3000");
        assert_eq!(config.db_path, "data/Merged-IP.mmdb");
        assert!(config.db_update_url.contains("Merged-IP.mmdb"));
        assert_eq!(config.db_update_time_utc, (1, 20));
        assert_eq!(config.site_domain, "localhost");
        assert!(config.ipv4_domain.is_empty());
        assert!(!config.dev_mode);
    }

    #[test]
    fn custom_listen_addr() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("LISTEN_ADDR", "127.0.0.1:8080");
        }

        let config = Config::from_env();
        assert_eq!(config.listen_addr.to_string(), "127.0.0.1:8080");

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("LISTEN_ADDR") };
    }

    #[test]
    fn custom_db_path() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_PATH", "/tmp/test.mmdb");
        }

        let config = Config::from_env();
        assert_eq!(config.db_path, "/tmp/test.mmdb");

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("DB_PATH") };
    }

    #[test]
    fn custom_update_url() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_UPDATE_URL", "https://example.com/db.mmdb");
        }

        let config = Config::from_env();
        assert_eq!(config.db_update_url, "https://example.com/db.mmdb");

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("DB_UPDATE_URL") };
    }

    #[test]
    fn custom_update_time() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_UPDATE_TIME_UTC", "03:45");
        }

        let config = Config::from_env();
        assert_eq!(config.db_update_time_utc, (3, 45));

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("DB_UPDATE_TIME_UTC") };
    }

    #[test]
    fn custom_site_domain() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("SITE_DOMAIN", "ip.example.com");
        }

        let config = Config::from_env();
        assert_eq!(config.site_domain, "ip.example.com");

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("SITE_DOMAIN") };
    }

    #[test]
    fn custom_ipv4_domain() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("IPV4_DOMAIN", "noipv6.org");
        }

        let config = Config::from_env();
        assert_eq!(config.ipv4_domain, "noipv6.org");

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("IPV4_DOMAIN") };
    }

    #[test]
    fn dev_mode_enabled() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DEV_MODE", "true");
        }

        let config = Config::from_env();
        assert!(config.dev_mode);

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("DEV_MODE") };
    }

    #[test]
    fn dev_mode_case_insensitive() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DEV_MODE", "True");
        }

        let config = Config::from_env();
        assert!(config.dev_mode);

        // SAFETY: ENV_LOCK is held.
        unsafe { remove_var("DEV_MODE") };
    }

    #[test]
    fn dev_mode_disabled_by_default() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe { clear_config_vars() };

        let config = Config::from_env();
        assert!(!config.dev_mode);
    }

    #[test]
    #[should_panic(expected = "LISTEN_ADDR must be a valid socket address")]
    fn invalid_listen_addr_panics() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("LISTEN_ADDR", "not-a-socket-addr");
        }

        let _config = Config::from_env();
    }

    #[test]
    #[should_panic(expected = "DB_UPDATE_TIME_UTC must be in HH:MM format")]
    fn invalid_update_time_panics() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_UPDATE_TIME_UTC", "not-a-time");
        }

        let _config = Config::from_env();
    }

    #[test]
    #[should_panic(expected = "DB_UPDATE_TIME_UTC hour must be 0-23")]
    fn update_time_invalid_hour_panics() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_UPDATE_TIME_UTC", "25:00");
        }

        let _config = Config::from_env();
    }

    #[test]
    #[should_panic(expected = "DB_UPDATE_TIME_UTC minute must be 0-59")]
    fn update_time_invalid_minute_panics() {
        let _guard = lock_env();
        // SAFETY: ENV_LOCK is held.
        unsafe {
            clear_config_vars();
            set_var("DB_UPDATE_TIME_UTC", "01:60");
        }

        let _config = Config::from_env();
    }
}
