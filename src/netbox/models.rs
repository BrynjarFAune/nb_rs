use crate::{
    fetch::azure::{IntuneDevice, IntuneUser},
    netbox::api::ApiClient,
    LocalCache,
};
use async_trait::async_trait;
use serde::{de::Error, Deserialize, Serialize};

//
// GENERAL
//

#[derive(Debug, Deserialize)]
pub struct ListResponse<T> {
    pub count: u32,
    pub results: Vec<T>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tag {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub color: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Status {
    value: String,
    label: String,
}

impl Status {
    pub fn new(name: String) -> Self {
        Self {
            value: name.to_lowercase(),
            label: name,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Site {
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl Site {
    pub fn new(name: String) -> Self {
        let slug = name.to_lowercase().replace(" ", "-");
        Self {
            id: None,
            name,
            slug,
        }
    }
}

//
// IPAM
//

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetBoxIp4 {
    id: u32,
    address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Prefix {
    id: u32,
    prefix: String,
    status: Status,
    vlan: u32,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vlan {
    vlan_id: u32,
    name: String,
    status: Status,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VlanPrefix {
    vlan: Vlan,
    prefix: Prefix,
}

//
// DCIM
//

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Device {
    pub name: String,
    pub id: Option<u32>,
    pub device_type: DeviceType,
    pub role: DeviceRole,
    pub site: Site,
    pub status: Status,
    pub serial: Option<String>,
    pub platform: Option<Platform>,
    pub primary_ip4: Option<NetBoxIp4>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Platform {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceRole {
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl DeviceRole {
    pub fn new(name: String) -> Self {
        let slug = name.to_lowercase().replace(" ", "-");
        Self {
            name,
            slug,
            id: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceType {
    pub manufacturer: Manufacturer,
    pub id: Option<u32>,
    pub model: String,
    pub slug: String,
}

impl DeviceType {
    pub fn new(manufacturer: Manufacturer, model: String) -> Self {
        let slug = model.to_lowercase().replace(" ", "-");
        Self {
            id: None,
            manufacturer,
            model,
            slug,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manufacturer {
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl Manufacturer {
    pub fn new(name: String) -> Self {
        let slug = name.to_lowercase().replace(" ", "-");
        Self {
            id: None,
            name,
            slug,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceList {
    pub count: i32,
    pub results: Vec<Device>,
}

//
// TENANCY
//

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContactList {
    pub count: Option<String>,
    pub results: Option<Vec<Contact>>,
}

//
// IMPL TRAITS FOR MODELS
//

/*

#[async_trait]
pub trait NetBoxObject: Clone + Send + Sync + 'static {
    // Create in netbox and return object
    async fn create_in_netbox(
        &self,
        api_client: &ApiClient,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    // Create in local cache
    async fn create_in_cache(
        &self,
        cache: &mut LocalCache,
    ) -> Result<(), Box<dyn std::error::Error>>;

    // Get caching key
    fn cache_key(&self) -> String;

    // Create in netbox and cache
    async fn create_and_cache(
        &self,
        api_client: &ApiClient,
        cache: &mut LocalCache,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let created = self.create_in_netbox(api_client).await?;
        created.create_in_cache(cache).await?;
        Ok(created)
    }
}

#[async_trait]
impl NetBoxObject for Device {
    async fn create_in_netbox(
        &self,
        api_client: &ApiClient,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint = "dcim/devices";
        let body = self;
        let res = api_client
            .post::<Device, Device>(endpoint, body)
            .await
            .expect("Failed to create device in NetBox");
        Ok(res)
    }

    async fn create_in_cache(
        &self,
        cache: &mut LocalCache,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = self.cache_key();
        cache.insert(key, self.clone());
        Ok(())
    }

    fn cache_key(&self) -> String {
        self.name.clone()
    }
}

#[async_trait]
impl NetBoxObject for Contact {
    async fn create_in_netbox(
        &self,
        api_client: &ApiClient,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let endpoint = "tenancy/contacts";
        let body = self;
        let res = api_client
            .post::<Contact, Contact>(endpoint, body)
            .await
            .expect("Error creating contact");
        Ok(res)
    }

    async fn create_in_cache(
        &self,
        cache: &mut LocalCache,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = self.cache_key();
        cache.insert(key, self.clone());
        Ok(())
    }

    fn cache_key(&self) -> String {
        //self.display
        String::new()
    }
}
*/
impl From<IntuneUser> for Contact {
    fn from(user: IntuneUser) -> Self {
        Contact {
            id: None,
            name: user.name,
            email: user.mail,
            title: user.title,
        }
    }
}
