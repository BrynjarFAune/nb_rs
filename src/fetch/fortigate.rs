use crate::config::FortiGateConfig;
use anyhow::anyhow;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Debug, fs, str::FromStr, usize};

#[derive(Debug)]
pub struct FortiGateClient {
    client: Client,
    token: String,
    url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FortiGateDevice {
    pub mac: String,
    pub is_online: bool,
    pub fortiswitch_id: Option<String>,
    pub fortiswitch_port_id: Option<usize>,
    pub ipv4_address: Option<String>,
    pub hardware_vendor: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub hostname: Option<String>,
    pub last_seen: usize,
    pub dhcp_lease_status: Option<String>,
    pub dhcp_lease_expire: Option<usize>,
    pub dhcp_lease_lease_reserved: Option<bool>,
    pub device_type: Option<String>,
    pub online_interfaces: Option<Vec<String>>,
    pub other_macs: Option<Vec<MacAddress>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MacAddress {
    pub ipv4_address: Option<String>,
    pub mac: String,
    pub last_seen: usize,
    pub is_online: bool,
    pub fortiswitch_id: String,
    pub fortiswitch_port_id: usize,
    pub dhcp_lease_status: Option<String>,
    pub dhcp_lease_expire: Option<usize>,
    pub dhcp_lease_lease_reserved: Option<bool>,
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
