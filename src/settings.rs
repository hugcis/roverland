/// This module is used to parse and read from configuration files for the
/// server.
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// This configuration object contains the database config.
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Database {
    /// Database url.
    pub url: String,
    /// Maximum number of connections to the database.
    pub max_connections: u32,
}

/// This configuration object contains the authentication config.
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Auth {
    /// Development mode where no authentication is needed.
    #[serde(default = "default_dev")]
    pub develop: bool,
}

fn default_dev() -> bool {
    false
}

/// The app wide settings
#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct BaseSettings {
    /// The URL the application is being served from.
    pub url: String,
    /// The rust log parameter. Describes how much logging is wanted.
    pub rust_log: Option<String>,
}

/// This structure contains all the config parameters of the app.
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    /// Settings related to the database.
    pub database: Database,
    /// Authentication config.
    pub auth: Auth,
    /// The app-wide config.
    pub base: BaseSettings,
}

impl Settings {
    /// Creates a new configuration form config files and environment variables.
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(".env").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(Environment::with_prefix("rov"))
            .build()?;
        s.try_deserialize()
    }
}
