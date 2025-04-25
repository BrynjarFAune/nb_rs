use anyhow::Ok;
use reqwest::Client;
use serde::Deserialize;

use crate::config::NagiosxiConfig;

#[derive(Debug)]
pub struct NagiosxiClient {
    pub client: Client,
    pub api_key: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HostStatus {
    pub host_object_id: String,
    pub host_name: String,
    pub display_name: String,
    pub address: String,
    pub status_update_time: String,
    pub output: String,
    pub current_state: String,
    pub last_check: String,
    pub next_check: String,
    pub last_time_up: String,
    pub last_time_down: String,
    pub last_time_unreachable: String,
    pub state_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceStatus {
    host_name: String,
    service_description: String,
    display_name: String,
    host_address: String,
    host_alias: String,
    output: String,
    current_state: String,
    last_check: String,
    next_check: String,
    last_time_ok: String,
    last_time_warning: String,
    last_time_unknown: String,
    last_time_critical: String,
}

#[derive(Debug, Deserialize)]
pub struct HostsList {
    pub recordcount: usize,
    pub hoststatus: Vec<HostStatus>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceList {
    pub recordcount: usize,
    pub servicestatus: Vec<ServiceStatus>,
}

impl NagiosxiClient {
    pub fn new(config: &NagiosxiConfig) -> Self {
        let api_key = config.api_key.clone();
        let url = config.url.clone();
        Self {
            client: Client::new(),
            api_key,
            url,
        }
    }

    pub async fn get_hosts(&self) -> anyhow::Result<HostsList> {
        let url = format!("{}/objects/hoststatus?apikey={}", self.url, self.api_key);
        let res = self.client.get(url).send().await?;

        Ok(res.json::<HostsList>().await?)
    }

    pub async fn get_services(&self) -> anyhow::Result<ServiceList> {
        let url = format!("{}/objects/servicestatus?apikey={}", self.url, self.api_key);
        let res = self.client.get(url).send().await?;

        Ok(res.json::<ServiceList>().await?)
    }
}
