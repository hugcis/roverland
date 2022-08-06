use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Database {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Auth {
    pub token: String,
    /// Development mode where no authentication is needed.
    #[serde(default = "default_dev")]
    pub develop: bool,
}

fn default_dev() -> bool {
    false
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct BaseSettings {
    pub url: String,
    pub rust_log: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub database: Database,
    pub auth: Auth,
    pub base: BaseSettings,
}

impl Settings {
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
