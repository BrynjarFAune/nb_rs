use crate::netbox::api::{ApiClient, CreateTable};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Device {
    name: String,
    id: u32,
    device_type: DeviceType,
    role: DeviceRole,
    site: Site,
    status: Status,
    serial: String,
    platform: Option<Platform>,
    primary_ip4: Option<NetBoxIp4>,
    tags: Option<Vec<Tag>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tag {
    id: u32,
    name: String,
    slug: String,
    color: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetBoxIp4 {
    id: u32,
    address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Platform {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    value: String,
    label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Site {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceRole {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceType {
    manufacturer: Manufacturer,
    id: u32,
    model: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manufacturer {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceList {
    pub count: i32,
    pub results: Vec<Device>,
}

pub struct DcimApi {
    client: ApiClient,
}

impl DcimApi {
    pub fn new(client: ApiClient) -> Self {
        DcimApi { client }
    }

    pub async fn get_devices(&self) -> Result<DeviceList, reqwest::Error> {
        self.client.get("dcim/devices/").await
    }

    pub async fn create_device(&self, body: &Device) -> Result<Device, reqwest::Error> {
        self.client
            .post::<Device, Device>("tenancy/contacts/", body)
            .await
    }
}

#[async_trait]
impl CreateTable for Device {
    async fn create(&self, api: &ApiClient) -> Result<(), reqwest::Error> {
        //api.create_device(self).await
        Ok(())
    }
}
