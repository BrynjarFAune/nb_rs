use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub netbox: NetBoxConfig,
    pub azure: AzureConfig,
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

pub fn load() -> Result<Settings, ConfigError> {
    let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "./src".into());

    let mut builder =
        Config::builder().add_source(File::with_name(&format!("{}/config.toml", config_dir)));

    if let Ok(url) = env::var("NETBOX_API_URL") {
        builder = builder.set_override("netbox.api_url", url)?;
    }

    if let Ok(key) = env::var("NETBOX_API_KEY") {
        builder = builder.set_override("netbox.api_key", key)?;
    }

    if let Ok(limit) = env::var("NETBOX_API_LIMIT") {
        builder = builder.set_override("netbox.api_limit", limit)?;
    }

    if let Ok(url) = env::var("AZURE_URL") {
        builder = builder.set_override("azure.url", url)?;
    }

    if let Ok(client_id) = env::var("CLIENT_ID") {
        builder = builder.set_override("azure.client_id", client_id)?;
    }

    if let Ok(tenant_id) = env::var("TENANT_ID") {
        builder = builder.set_override("azure.tenant_id", tenant_id)?;
    }

    if let Ok(client_secret) = env::var("CLIENT_SECRET") {
        builder = builder.set_override("azure.client_secret", client_secret)?;
    }

    let config = builder.build()?;

    config.try_deserialize()
}
