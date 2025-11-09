use std::env;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub mongodb_uri: String,
    pub default_database: Option<String>,
    pub default_collection: Option<String>,
    pub pool_min_size: Option<u32>,
    pub pool_max_size: Option<u32>,
    pub connect_timeout: Option<Duration>,
    pub server_selection_timeout: Option<Duration>,
    pub log_level: Option<String>,
    pub bind_address: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}`")]
    MissingEnv(&'static str),
    #[error("invalid value for `{0}`: {1}")]
    InvalidEnv(&'static str, String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mongodb_uri = get_required("MONGODB_URI")?;

        let default_database = env::var("MONGODB_DEFAULT_DATABASE")
            .ok()
            .filter(|s| !s.is_empty());
        let default_collection = env::var("MONGODB_DEFAULT_COLLECTION")
            .ok()
            .filter(|s| !s.is_empty());

        let pool_min_size = parse_optional_u32("MONGODB_POOL_MIN_SIZE")?;
        let pool_max_size = parse_optional_u32("MONGODB_POOL_MAX_SIZE")?;

        let connect_timeout = parse_optional_duration("MONGODB_CONNECT_TIMEOUT_MS")?;
        let server_selection_timeout =
            parse_optional_duration("MONGODB_SERVER_SELECTION_TIMEOUT_MS")?;

        let log_level = env::var("LOG_LEVEL").ok().filter(|s| !s.is_empty());
        let bind_address =
            env::var("APP_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

        Ok(Self {
            mongodb_uri,
            default_database,
            default_collection,
            pool_min_size,
            pool_max_size,
            connect_timeout,
            server_selection_timeout,
            log_level,
            bind_address,
        })
    }
}

fn get_required(key: &'static str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(ConfigError::MissingEnv(key)),
    }
}

fn parse_optional_u32(key: &'static str) -> Result<Option<u32>, ConfigError> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => value
            .parse::<u32>()
            .map(Some)
            .map_err(|err| ConfigError::InvalidEnv(key, err.to_string())),
        _ => Ok(None),
    }
}

fn parse_optional_duration(key: &'static str) -> Result<Option<Duration>, ConfigError> {
    parse_optional_u64(key).map(|opt| opt.map(Duration::from_millis))
}

fn parse_optional_u64(key: &'static str) -> Result<Option<u64>, ConfigError> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => value
            .parse::<u64>()
            .map(Some)
            .map_err(|err| ConfigError::InvalidEnv(key, err.to_string())),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_env(key: &str, value: &str, f: impl FnOnce()) {
        env::set_var(key, value);
        f();
        env::remove_var(key);
    }

    #[test]
    fn missing_required_variable_fails() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        let old_uri = env::var("MONGODB_URI").ok();
        env::remove_var("MONGODB_URI");
        let result = Config::from_env();
        assert!(matches!(
            result,
            Err(ConfigError::MissingEnv("MONGODB_URI"))
        ));
        if let Some(uri) = old_uri {
            env::set_var("MONGODB_URI", uri);
        }
    }

    #[test]
    fn parses_optional_values() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("MONGODB_POOL_MIN_SIZE");
        env::remove_var("MONGODB_CONNECT_TIMEOUT_MS");
        with_env("MONGODB_POOL_MIN_SIZE", "5", || {
            with_env("MONGODB_CONNECT_TIMEOUT_MS", "1000", || {
                let config = Config::from_env().expect("config");
                assert_eq!(config.pool_min_size, Some(5));
                assert_eq!(config.connect_timeout, Some(Duration::from_millis(1000)));
            });
        });
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn rejects_invalid_u32_values() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("MONGODB_POOL_MIN_SIZE");
        with_env("MONGODB_POOL_MIN_SIZE", "not_a_number", || {
            let result = Config::from_env();
            assert!(matches!(
                result,
                Err(ConfigError::InvalidEnv("MONGODB_POOL_MIN_SIZE", _))
            ));
        });
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn rejects_invalid_duration_values() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("MONGODB_CONNECT_TIMEOUT_MS");
        with_env("MONGODB_CONNECT_TIMEOUT_MS", "invalid", || {
            let result = Config::from_env();
            assert!(matches!(
                result,
                Err(ConfigError::InvalidEnv("MONGODB_CONNECT_TIMEOUT_MS", _))
            ));
        });
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn filters_empty_strings_from_optional_vars() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("MONGODB_DEFAULT_DATABASE");
        env::remove_var("MONGODB_DEFAULT_COLLECTION");
        with_env("MONGODB_DEFAULT_DATABASE", "", || {
            with_env("MONGODB_DEFAULT_COLLECTION", "", || {
                let config = Config::from_env().expect("config");
                assert_eq!(config.default_database, None);
                assert_eq!(config.default_collection, None);
            });
        });
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn uses_default_bind_address_when_not_set() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        let old_bind = env::var("APP_BIND_ADDRESS").ok();
        env::remove_var("APP_BIND_ADDRESS");
        let config = Config::from_env().expect("config");
        assert_eq!(config.bind_address, "127.0.0.1:3000");
        if let Some(addr) = old_bind {
            env::set_var("APP_BIND_ADDRESS", addr);
        }
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn parses_all_optional_pool_settings() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("MONGODB_POOL_MIN_SIZE");
        env::remove_var("MONGODB_POOL_MAX_SIZE");
        env::remove_var("MONGODB_SERVER_SELECTION_TIMEOUT_MS");
        with_env("MONGODB_POOL_MIN_SIZE", "10", || {
            with_env("MONGODB_POOL_MAX_SIZE", "50", || {
                with_env("MONGODB_SERVER_SELECTION_TIMEOUT_MS", "5000", || {
                    let config = Config::from_env().expect("config");
                    assert_eq!(config.pool_min_size, Some(10));
                    assert_eq!(config.pool_max_size, Some(50));
                    assert_eq!(
                        config.server_selection_timeout,
                        Some(Duration::from_millis(5000))
                    );
                });
            });
        });
        env::remove_var("MONGODB_URI");
    }

    #[test]
    fn parses_log_level() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();
        env::set_var("MONGODB_URI", "mongodb://localhost:27017");
        env::remove_var("LOG_LEVEL");
        with_env("LOG_LEVEL", "debug", || {
            let config = Config::from_env().expect("config");
            assert_eq!(config.log_level, Some("debug".to_string()));
        });
        env::remove_var("MONGODB_URI");
    }
}
