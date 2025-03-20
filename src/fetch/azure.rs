use crate::config::AzureConfig;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, StatusCode,
};
use serde::Deserialize;
use serde_json::Value;
use std::fmt::Debug;

#[derive(Debug)]
pub struct AzureClient {
    client: Client,
    token: Option<String>,
    url: String,
}

#[derive(Debug, Deserialize)]
pub struct IntuneDevice {
    #[serde(rename = "deviceName")]
    name: String,
    #[serde(rename = "enrolledDateTime")]
    enrolled: String,
    #[serde(rename = "lastSyncDateTime")]
    synced: String,
    #[serde(rename = "operatingSystem")]
    os: String,
    #[serde(rename = "osVersion")]
    os_version: String,
    #[serde(rename = "managementAgent")]
    management_agend: String,
    #[serde(rename = "emailAddress")]
    user: String,
    model: String,
    manufacturer: String,
    #[serde(rename = "serialNumber")]
    serial: String,
    #[serde(rename = "wiFiMacAddress")]
    wifi_mac: String,
    #[serde(rename = "totalStorageSpaceInBytes")]
    total_storage: usize,
    #[serde(rename = "freeStorageSpaceInBytes")]
    free_storage: usize,
}

#[derive(Debug, Deserialize)]
pub struct IntuneUser {
    #[serde(rename = "displayName")]
    pub name: String,
    pub mail: Option<String>,
    #[serde(rename = "jobTitle")]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsersResponse {
    #[serde(rename = "@odata.nextLink")]
    next: Option<String>,
    value: Vec<IntuneUser>,
}

#[derive(Debug, Deserialize)]
struct DevicesResponse {
    #[serde(rename = "@odata.nextLink")]
    next: Option<String>,
    value: Vec<IntuneDevice>,
}

impl AzureClient {
    pub async fn new(config: &AzureConfig) -> anyhow::Result<Self> {
        let client = Client::new();
        let token = Self::fetch_token(&client, &config).await?;
        let url = &config.url;

        Ok(AzureClient {
            client,
            url: url.to_string(),
            token: Some(token),
        })
    }

    pub async fn fetch_token(client: &Client, config: &AzureConfig) -> anyhow::Result<String> {
        let params = [
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("scope", &"https://graph.microsoft.com/.default".to_string()),
            ("grant_type", &"client_credentials".to_string()),
        ];

        let res = client
            .post(format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                config.tenant_id
            ))
            .form(&params)
            .send()
            .await?;

        let res_json: Value = res.json().await?;
        let token = res_json["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve token"))?
            .to_string();

        Ok(token)
    }

    pub async fn fetch_users(&self) -> Result<Vec<IntuneUser>, reqwest::Error> {
        let mut all_users = Vec::new();
        let mut next_link: Option<String> = Some(format!("{}/users", self.url));

        let mut headers = HeaderMap::new();
        if let Some(token) = &self.token {
            if let Ok(value) = HeaderValue::from_str(token) {
                headers.insert("Authorization", value);
            }
        }

        while let Some(url) = next_link {
            let res = self
                .client
                .get(&url)
                .headers(headers.clone())
                .send()
                .await?;

            match res.status() {
                StatusCode::OK => {
                    let json = res.json::<UsersResponse>().await?;
                    all_users.extend(json.value);
                    next_link = json.next;
                }
                _ => {
                    let status_code = res.status();
                    let response = res.text().await;
                    eprintln!("Error fetching users: {:?} - {:?}", status_code, response);
                    break;
                }
            }
        }

        Ok(all_users)
    }

    pub async fn fetch_devices(&self) -> Result<Vec<IntuneDevice>, reqwest::Error> {
        let mut all_devices = Vec::new();
        let mut next_link: Option<String> =
            Some(format!("{}/deviceManagement/managedDevices", self.url));

        let mut headers = HeaderMap::new();
        if let Some(token) = &self.token {
            if let Ok(value) = HeaderValue::from_str(token) {
                headers.insert("Authorization", value);
            }
        }

        while let Some(url) = next_link {
            let res = self
                .client
                .get(&url)
                .headers(headers.clone())
                .send()
                .await?;

            match res.status() {
                StatusCode::OK => {
                    let json = res.json::<DevicesResponse>().await?;
                    all_devices.extend(json.value);
                    next_link = json.next;
                }
                _ => {
                    let status_code = res.status();
                    let response = res.text().await;
                    eprintln!("Error fetching devices: {:?} - {:?}", status_code, response);
                    break;
                }
            }
        }

        Ok(all_devices)
    }
}
