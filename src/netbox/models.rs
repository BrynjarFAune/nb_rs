use crate::{
    fetch::{
        azure::{IntuneDevice, IntuneUser},
        fortigate::FortiGateDevice,
        nagiosxi::HostStatus,
    },
    netbox::api::{ApiClient, CreateTable},
    utils::sanitize_slug,
    LocalCache,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{de::Error, Deserialize, Serialize};
use std::fmt::Debug;

#[async_trait]
pub trait NetBoxModel: Send + Sync + Clone + Debug + Serialize + for<'de> Deserialize<'de> {
    type Id: ToString + Clone;

    fn get_id(&self) -> Option<Self::Id>;
    fn get_slug(&self) -> String;
    fn get_endpoint() -> &'static str;
    fn set_id(&mut self, id: Self::Id);

    fn get_cache_key(&self) -> String {
        self.get_slug()
    }
}

#[async_trait]
impl<T: NetBoxModel> CreateTable for T {
    async fn create(&self, api: &ApiClient) -> Result<()> {
        let _created: T = api.post(Self::get_endpoint(), self).await?;
        Ok(())
    }
}
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}
impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            slug: sanitize_slug(&name),
            name,
            color: None,
        }
    }
}

#[async_trait]
impl NetBoxModel for Tag {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.slug.clone())
    }

    fn get_endpoint() -> &'static str {
        "extras/tags"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StatusOptions {
    Active,
    Offline,
    Planned,
    Staged,
    Failed,
    Inventory,
    Decommissioning,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Status {
    value: StatusOptions,
}

impl Status {
    pub fn from_value(value: StatusOptions) -> Self {
        Status { value }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Site {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl Site {
    pub fn new(name: String) -> Self {
        let slug = sanitize_slug(&name);
        Self {
            id: None,
            name,
            slug,
        }
    }
}

#[async_trait]
impl NetBoxModel for Site {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        self.slug.clone()
    }

    fn get_endpoint() -> &'static str {
        "dcim/sites"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}

//
// IPAM
//

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetBoxIp4 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub address: String,
}

impl NetBoxIp4 {
    pub fn new(address: String) -> Self {
        Self { id: None, address }
    }
}

#[async_trait]
impl NetBoxModel for NetBoxIp4 {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.address.clone())
    }

    fn get_endpoint() -> &'static str {
        "ipam/ip-addresses"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Prefix {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
    prefix: String,
    status: Status,
    vlan: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vlan {
    vlan_id: u32,
    name: String,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub device_type: Option<DeviceType>,
    pub role: Option<DeviceRole>,
    pub site: Option<Site>,
    pub status: Option<Status>,
    pub serial: Option<String>,
    pub platform: Option<Platform>,
    pub primary_ip4: Option<NetBoxIp4>,
    pub tags: Option<Vec<Tag>>,
}

#[async_trait]
impl NetBoxModel for Device {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        self.name.to_lowercase()
    }

    fn get_cache_key(&self) -> String {
        self.get_slug()
    }

    fn get_endpoint() -> &'static str {
        "dcim/devices"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PostDevice {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub device_type: u32,
    pub role: u32,
    pub site: u32,
    pub status: StatusOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    pub platform: Option<u32>,
    pub tags: Vec<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Platform {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}
impl Platform {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            slug: sanitize_slug(&name),
            name,
        }
    }
}

#[async_trait]
impl NetBoxModel for Platform {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.slug)
    }

    fn get_endpoint() -> &'static str {
        "dcim/platforms"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }

    fn get_cache_key(&self) -> String {
        self.name.to_lowercase().into()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceRole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl DeviceRole {
    pub fn new(name: String) -> Self {
        let slug = sanitize_slug(&name);
        Self {
            name,
            slug,
            id: None,
        }
    }
}

#[async_trait]
impl NetBoxModel for DeviceRole {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        self.slug.clone()
    }

    fn get_endpoint() -> &'static str {
        "dcim/device-roles"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceType {
    pub manufacturer: Manufacturer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub model: String,
    pub slug: String,
}

impl DeviceType {
    pub fn new(manufacturer: Manufacturer, model: String) -> Self {
        let slug = sanitize_slug(&model);
        Self {
            id: None,
            manufacturer,
            model,
            slug,
        }
    }
}

#[async_trait]
impl NetBoxModel for DeviceType {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.slug)
    }

    fn get_endpoint() -> &'static str {
        "dcim/device-types"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }

    fn get_cache_key(&self) -> String {
        format!(
            "{}--{}",
            self.manufacturer.get_slug(),
            sanitize_slug(&self.model)
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manufacturer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
}

impl Manufacturer {
    pub fn new(name: String) -> Self {
        let slug = sanitize_slug(&name);
        Self {
            id: None,
            name,
            slug,
        }
    }
}

#[async_trait]
impl NetBoxModel for Manufacturer {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.slug)
    }

    fn get_endpoint() -> &'static str {
        "dcim/manufacturers"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
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

#[async_trait]
impl NetBoxModel for Contact {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        self.id
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.name)
    }

    fn get_endpoint() -> &'static str {
        "tenancy/contacts"
    }

    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }

    fn get_cache_key(&self) -> String {
        if let Some(ref mail) = self.email {
            mail.to_lowercase().to_string()
        } else {
            self.get_slug()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ContactList {
    pub count: Option<String>,
    pub results: Option<Vec<Contact>>,
}

//
// VIRTUALIZATION
//

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VirtualMachine {
    name: String,
    status: String,
    site: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<DeviceRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    serial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vcpus: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<Tag>>,
}

#[async_trait]
impl NetBoxModel for VirtualMachine {
    type Id = u32;

    fn get_id(&self) -> Option<Self::Id> {
        None // Replace with real ID logic if your struct gets populated with NetBox IDs
    }

    fn get_slug(&self) -> String {
        sanitize_slug(&self.name)
    }

    fn get_endpoint() -> &'static str {
        "virtualization/virtual-machines"
    }

    fn set_id(&mut self, _id: Self::Id) {
        // Handle if needed
    }
}

//
// IMPL TRAITS FOR MODELS
//

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

impl From<HostStatus> for VirtualMachine {
    fn from(value: HostStatus) -> Self {
        VirtualMachine {
            name: value.host_name,
            status: String::from("active"),
            site: 5,
            role: None,
            serial: None,
            platform: None,
            vcpus: None,
            memory: None,
            disk: None,
            tags: None,
        }
    }
}

impl From<IntuneDevice> for Device {
    fn from(value: IntuneDevice) -> Self {
        Device {
            name: value.name,
            id: None,
            device_type: Some(DeviceType::new(
                Manufacturer::new(value.manufacturer.to_string()),
                value.model.to_string(),
            )),
            role: Some(DeviceRole::new("Desktop".to_string())),
            site: Some(Site::new("TOS".to_string())),
            status: Some(Status::from_value(StatusOptions::Active)),
            serial: Some(value.serial),
            platform: Some(Platform::new(format!("{} {}", value.os, value.os_version))),
            primary_ip4: None,
            tags: Some(vec![Tag::new("AAD".to_string())]),
        }
    }
}

impl From<FortiGateDevice> for Device {
    fn from(value: FortiGateDevice) -> Self {
        let status = Some(if value.is_online {
            Status::from_value(StatusOptions::Active)
        } else {
            Status::from_value(StatusOptions::Offline)
        });
        let platform = {
            if let Some(os) = value.os_name {
                Some(Platform::new(os.to_string()))
            } else {
                None
            }
        };
        let primary_ip4 = {
            if let Some(ip) = value.ipv4_address {
                Some(NetBoxIp4::new(ip.to_string()))
            } else {
                None
            }
        };
        let name = {
            if let Some(hostname) = value.hostname.clone() {
                hostname
            } else {
                value.mac.clone()
            }
        };
        let device_type = match (value.device_type, value.hardware_vendor) {
            (Some(ty), Some(vendor)) => Some(DeviceType::new(Manufacturer::new(vendor), ty)),
            _ => None,
        };
        let role = Some(DeviceRole::new("Desktop".to_string()));
        Device {
            name,
            id: None,
            device_type,
            role,
            site: Some(Site::new("TOS".to_string())),
            status,
            serial: None,
            platform,
            primary_ip4,
            tags: Some(vec![Tag::new("FortiGate".to_string())]),
        }
    }
}

// Note: Before converting Device to PostDevice, ensure_tags() should be called
// to sync tags with NetBox and populate their IDs.
impl TryFrom<Device> for PostDevice {
    type Error = anyhow::Error;

    fn try_from(value: Device) -> Result<Self> {
        // collect tag-IDs
        let id_tags: Vec<u32> = value
            .tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| tag.id)
            .collect();

        // pull out each component, returning Err if missing
        let device_type = value
            .device_type
            .as_ref()
            .and_then(|dt| dt.id)
            .ok_or_else(|| anyhow!("Device type must be created first (use ensure_components)"))?;

        let role =
            value.role.as_ref().and_then(|r| r.id).ok_or_else(|| {
                anyhow!("Device role must be created first (use ensure_components)")
            })?;

        let site = value
            .site
            .as_ref()
            .and_then(|s| s.id)
            .ok_or_else(|| anyhow!("Site must be created first (use ensure_components)"))?;

        let status = value
            .status
            .as_ref()
            .map(|s| s.value.clone())
            .ok_or_else(|| anyhow!("Device status is required"))?;

        Ok(PostDevice {
            name: value.name,
            id: value.id,
            device_type,
            role,
            site,
            status,
            serial: value.serial,
            platform: value.platform.as_ref().and_then(|p| p.id),
            tags: id_tags,
        })
    }
}

impl Device {
    pub async fn push_to_netbox(mut self, api: &ApiClient, cache: &LocalCache) -> Result<()> {
        // 1️⃣ Normalize and log the cache key
        let key = self.get_cache_key();
        println!("🔍 [push_to_netbox] lookup key=`{}`", key);

        // 2️⃣ Try cache
        if let Some(cached) = cache.devices.get(&key) {
            if let Some(cached_id) = cached.get_id() {
                println!("✅ [push_to_netbox] cache HIT `{}` → id={}", key, cached_id);
                self.id = Some(cached_id);
            }
        } else {
            println!("❌ [push_to_netbox] cache MISS `{}`", key);
        }

        // 3️⃣ Ensure all related NetBox objects (types, roles, tags, etc.) exist
        //self.ensure_components(api, cache).await?;
        cache
            .ensure_device_components(&mut self, api)
            .await
            .context(format!("While ensuring sub-objects for `{}`", key))?;

        // 4️⃣ Build the payload struct
        let postable: PostDevice = PostDevice::try_from(self.clone())
            .context(format!("Failed to build PostDevice for `{}`", key))?;

        // ─────────────────────────────────────────────
        // 💡 **DEBUG: dump the exact JSON you'll send**
        let json_payload = serde_json::to_string_pretty(&postable)?;
        println!("🗳️  Payload JSON:\n{}", json_payload);
        // ─────────────────────────────────────────────

        // 5️⃣ Decide: PATCH if we already have an id, else POST
        if let Some(id) = self.id {
            let endpoint = format!("dcim/devices/{}/", id);
            let _updated: Device = api
                .patch(&endpoint, &postable)
                .await
                .context(format!("patching device `{}` (id={})", key, id))?;
            println!("🔄 [push_to_netbox] updated `{}` → id={}", key, id);
        } else {
            let created: Device = api
                .post("dcim/devices/", &postable)
                .await
                .context(format!("Creating new device `{}`", key))?;
            let created_id = created
                .get_id()
                .expect("NetBox must return an ID on creation");
            println!("✅ [push_to_netbox] created `{}` → id={}", key, created_id);

            // 6️⃣ Insert into cache under the same normalized key
            cache.devices.insert(key.clone(), created.clone());
            self.id = Some(created_id);
        }

        Ok(())
    }

    pub fn merge_from_intune(&mut self, src: &IntuneDevice) {
        if self.serial.is_none() {
            self.serial = Some(src.serial.clone());
        }
        if self.platform.is_none() {
            self.platform = Some(Platform {
                id: None,
                name: src.os.clone().into(),
                slug: sanitize_slug(&src.os),
            })
        }
        if self.role.is_none() {
            self.role = Some(DeviceRole::new("Desktop".to_string()));
        }

        if self.status.is_none() {
            self.status = Self::status_from_sync(&src.synced);
        }

        if self.device_type.is_none() {
            self.device_type = Some(DeviceType::new(
                Manufacturer::new(src.manufacturer.to_string()),
                src.model.to_string(),
            ));
        }
        self.push_tag(Tag::new("AAD".to_string()));
    }

    pub fn merge_from_fortigate(&mut self, src: &FortiGateDevice) {
        if self.platform.is_none() {
            if let Some(os) = &src.os_name {
                self.platform = Some(Platform {
                    id: None,
                    name: os.to_string(),
                    slug: sanitize_slug(&os.to_string()),
                })
            }
        }

        if src.is_online {
            self.status = Some(Status::from_value(StatusOptions::Active));
        }

        if self.device_type.is_none() {
            let device_type = match (src.device_type.clone(), src.hardware_vendor.clone()) {
                (Some(ty), Some(vendor)) => Some(DeviceType::new(Manufacturer::new(vendor), ty)),
                _ => Some(DeviceType::new(
                    Manufacturer::new("default".into()),
                    "default".into(),
                )),
            };
            if device_type.is_some() {
                self.device_type = device_type;
            } else {
                println!("DeviceType is somehow None");
            }
        }

        self.push_tag(Tag::new("FortiGate".to_string()));
        if let Some(true) = src.dhcp_lease_lease_reserved {
            self.push_tag(Tag::new("Reserved DHCP".to_string()));
        }
    }

    //pub fn merge_from_eset(&mut self, src: Device) {}

    fn push_tag(&mut self, tag: Tag) {
        match &mut self.tags {
            Some(tags) => {
                if !tags.iter().any(|t| t.slug == tag.slug) {
                    tags.push(tag);
                }
            }
            None => {
                self.tags = Some(vec![tag]);
            }
        }
    }

    fn status_from_sync(sync_time: &str) -> Option<Status> {
        let parsed = sync_time.parse::<DateTime<Utc>>().ok()?;
        let days = (Utc::now() - parsed).num_days();

        Some(if days <= 7 {
            Status::from_value(StatusOptions::Active)
        } else {
            Status::from_value(StatusOptions::Offline)
        })
    }
}
