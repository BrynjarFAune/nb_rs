use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub netbox: NetBoxConfig,
    pub azure: AzureConfig,
    pub fortigate: FortiGateConfig,
    // add eset, azure, foritgate, ...
}

#[derive(Debug, Deserialize)]
pub struct NetBoxConfig {
    pub api_key: String,
    pub api_url: String,
    pub api_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct AzureConfig {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct FortiGateConfig {
    pub url: String,
    pub token: String,
}

pub fn load() -> Result<Settings, ConfigError> {
    let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "./src".into());

    let config = Config::builder()
        .add_source(File::with_name(&format!("{}/config.toml", config_dir)))
        .build()?;

    config.try_deserialize()
}
