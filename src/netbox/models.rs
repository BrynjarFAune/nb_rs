use crate::{
    fetch::{
        azure::{IntuneDevice, IntuneUser},
        fortigate::FortiGateDevice,
        nagiosxi::HostStatus,
    },
    netbox::api::ApiClient,
    LocalCache,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
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
    pub id: Option<u32>,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
}
impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            slug: name.to_lowercase(),
            name,
            color: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Active,
    Offline,
    Planned,
    Staged,
    Failed,
    Inventory,
    Decommissioning,
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
    pub id: Option<u32>,
    pub address: String,
}

impl NetBoxIp4 {
    pub fn new(address: String) -> Self {
        Self { id: None, address }
    }
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
    pub device_type: Option<DeviceType>,
    pub role: Option<DeviceRole>,
    pub site: Option<Site>,
    pub status: Option<Status>,
    pub serial: Option<String>,
    pub platform: Option<Platform>,
    pub primary_ip4: Option<NetBoxIp4>,
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PostDevice {
    pub name: String,
    pub id: Option<u32>,
    pub device_type: u32,
    pub role: u32,
    pub site: u32,
    pub status: Status,
    pub serial: Option<String>,
    pub platform: Option<u32>,
    pub tags: Vec<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Platform {
    id: Option<u32>,
    name: String,
    slug: String,
}
impl Platform {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            slug: name.to_lowercase(),
            name,
        }
    }
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
            status: Some(Status::Active),
            serial: Some(value.serial),
            platform: Some(Platform::new(format!("{} {}", value.os, value.os_version))),
            primary_ip4: None,
            tags: Some(vec![Tag::new("AAD".to_string())]),
        }
    }
}

impl From<FortiGateDevice> for Device {
    fn from(value: FortiGateDevice) -> Self {
        let status = Some({
            if value.is_online {
                Status::Active
            } else {
                Status::Offline
            }
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
        Device {
            name,
            id: None,
            device_type,
            role: None,
            site: Some(Site::new("TOS".to_string())),
            status,
            serial: None,
            platform,
            primary_ip4,
            tags: Some(vec![Tag::new("FortiGate".to_string())]),
        }
    }
}

impl From<Device> for PostDevice {
    fn from(value: Device) -> Self {
        let id_tags: Vec<u32> = value
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| tag.id.expect("Need a valid tag id"))
            .collect();
        PostDevice {
            name: value.name,
            id: value.id,
            device_type: value
                .device_type
                .as_ref()
                .map(|p| p.id.expect("Need a valid device type id"))
                .expect("Missing device type"),
            role: value
                .role
                .as_ref()
                .map(|p| p.id.expect("Need a valid device role id"))
                .expect("Missing device role"),
            site: value
                .site
                .as_ref()
                .map(|p| p.id.expect("Need a valid site id"))
                .expect("Missing site"),
            status: value.status.expect("Need a valid device status"),
            serial: value.serial,
            // Optional / can be None or Some(id)
            platform: value
                .platform
                .as_ref()
                .map(|p| p.id.expect("Need a valid platform id")),
            tags: id_tags,
        }
    }
}

impl Device {
    pub fn merge_from_intune(&mut self, src: &IntuneDevice) {
        if self.serial.is_none() {
            self.serial = Some(src.serial.clone());
        }
        if self.platform.is_none() {
            self.platform = Some(Platform {
                id: None,
                name: src.os.clone().into(),
                slug: src.os.clone().into(),
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
                    slug: os.to_string(),
                })
            }
        }

        if src.is_online {
            self.status = Some(Status::Active);
        }

        if self.device_type.is_none() {
            let device_type = match (src.device_type.clone(), src.hardware_vendor.clone()) {
                (Some(ty), Some(vendor)) => Some(DeviceType::new(Manufacturer::new(vendor), ty)),
                _ => None,
            };
            if device_type.is_some() {
                self.device_type = device_type;
            }
        }

        self.push_tag(Tag::new("FortiGate".to_string()));
        if let Some(true) = src.dhcp_lease_lease_reserved {
            self.push_tag(Tag::new("Reserved DHCP".to_string()));
        }
    }

    pub fn merge_from_eset(&mut self, src: Device) {}

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
            Status::Active
        } else {
            Status::Offline
        })
    }
}
