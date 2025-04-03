use crate::config::FortiGateConfig;
use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Debug, fs, str::FromStr};

#[derive(Debug)]
pub struct FortiGateClient {
    client: Client,
    token: String,
    url: String,
}

#[derive(Debug, Deserialize)]
pub struct FortiGateDevice {
    mac: String,
}

#[derive(Debug, Deserialize)]
pub struct Vlan {
    name: String,
    prefix: String,
}

#[derive(Debug, Deserialize)]
pub struct Ip {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct FortiGateResponse {
    results: Vec<FortiGateDevice>,
}

/*
* Fortigate res:
*   {
*       some_bs: "",
*       results: [
*           device: {
*       }]
*   }
*
*   just directly map the results to a vec
* */

impl FortiGateClient {
    pub async fn new(config: &FortiGateConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Load root certificate
        let cert_bytes = fs::read("certs/FortiGate.crt")?;
        let cert = Certificate::from_pem(&cert_bytes)?;

        let client = Client::builder()
            .add_root_certificate(cert)
            .danger_accept_invalid_certs(true) // ⚠️ unsafe in prod
            .build()?;

        Ok(FortiGateClient {
            client,
            url: config.url.clone(),
            token: config.token.clone(),
        })
    }

    pub async fn fetch_devices(&self) -> anyhow::Result<Vec<FortiGateDevice>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.token))
                .map_err(|e| anyhow!("Invalid header value: {}", e))?,
        );
        headers.insert("Content-Type", HeaderValue::from_str("application/json")?);

        let url = format!("{}/monitor/user/device/query", &self.url);
        println!("Attempting fetch from FortiGate...");

        let res = self.client.get(url).headers(headers).send().await?;

        match res.status() {
            StatusCode::OK => {
                let json = res.json::<FortiGateResponse>().await?;
                return Ok(json.results);
            }
            _ => {
                eprintln!(
                    "Error fetching devices from FortiGate: {:?} - {:?}",
                    res.status(),
                    res.text().await,
                );
                return Ok(Vec::new());
            }
        }
    }
}
