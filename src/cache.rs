use dashmap::DashMap;
use futures::future::join_all;
use serde::Deserialize;
use std::sync::Arc;
use tokio::task;

use crate::netbox::{
    api::ApiClient,
    models::{
        Contact, ContactList, Device, DeviceRole, DeviceType, Manufacturer, Platform, Site, Tag,
    },
};

#[derive(Clone)]
pub struct LocalCache {
    pub devices: Arc<DashMap<String, Device>>,
    pub contacts: Arc<DashMap<String, Contact>>,
    pub manufacturers: Arc<DashMap<String, Manufacturer>>,
    pub device_type: Arc<DashMap<String, DeviceType>>,
    pub device_role: Arc<DashMap<String, DeviceRole>>,
    pub sites: Arc<DashMap<String, Site>>,
    pub tags: Arc<DashMap<String, Tag>>,
    pub platforms: Arc<DashMap<String, Platform>>,
}

impl LocalCache {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(DashMap::new()),
            contacts: Arc::new(DashMap::new()),
            manufacturers: Arc::new(DashMap::new()),
            device_role: Arc::new(DashMap::new()),
            device_type: Arc::new(DashMap::new()),
            sites: Arc::new(DashMap::new()),
            tags: Arc::new(DashMap::new()),
            platforms: Arc::new(DashMap::new()),
        }
    }

    pub async fn preload(
        client: Arc<crate::netbox::api::ApiClient>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading cache...");
        let cache = Self::new();
        let mut tasks = Vec::new();

        // Contact cache
        {
            let netbox_client = Arc::clone(&client);
            let contacts = Arc::clone(&cache.contacts);
            tasks.push(task::spawn(async move {
                println!("loading contacts");

                match netbox_client.get::<Contact>("tenancy/contacts", None).await {
                    Ok(contact_list) => {
                        for contact in contact_list {
                            contacts.insert(contact.name.clone(), contact.clone());
                            println!("+ contact: {}", contact.name);
                        }
                        println!("cached contacts");
                    }
                    Err(e) => eprint!("failed to load contacts: {}", e),
                }
            }));
        }
        // Device cache
        {
            let netbox_client = Arc::clone(&client);
            let devices = Arc::clone(&cache.devices);
            tasks.push(task::spawn(async move {
                println!("loading contacts");

                match netbox_client.get::<Device>("dcim/devices", None).await {
                    Ok(device_list) => {
                        for device in device_list {
                            devices.insert(device.name.clone(), device.clone());
                            println!("+ device: {}", device.name);
                        }
                        println!("cached devices");
                    }
                    Err(e) => eprint!("failed to load devices: {}", e),
                }
            }));
        }
        // Device Type cache
        {
            println!("Caching device types...");
            let netbox_client = Arc::clone(&client);
            let device_types = Arc::clone(&cache.device_type);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<DeviceType>("dcim/device-types", None)
                    .await
                {
                    Ok(device_type_list) => {
                        for device_type in device_type_list {
                            device_types.insert(device_type.model.clone(), device_type.clone());
                            println!("+ device-type: {}", device_type.model);
                        }
                        println!("cached device-types");
                    }
                    Err(e) => eprint!("failed to load device-types: {}", e),
                }
            }));
        }
        // Device role cache
        {
            println!("Caching device roles...");
            let netbox_client = Arc::clone(&client);
            let device_roles = Arc::clone(&cache.device_role);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<DeviceRole>("dcim/device-roles", None)
                    .await
                {
                    Ok(device_role_list) => {
                        for device_role in device_role_list {
                            device_roles.insert(device_role.name.clone(), device_role.clone());
                            println!("+ device-role: {}", device_role.name);
                        }
                        println!("cached device-roles");
                    }
                    Err(e) => eprint!("failed to load device-roles: {}", e),
                }
            }));
        }
        // Manufacturer cache
        {
            println!("Caching manufacturers...");
            let netbox_client = Arc::clone(&client);
            let manufacturers = Arc::clone(&cache.manufacturers);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<Manufacturer>("dcim/manufacturers", None)
                    .await
                {
                    Ok(manufacturer_list) => {
                        for manufacturer in manufacturer_list {
                            manufacturers.insert(manufacturer.name.clone(), manufacturer.clone());
                            println!("+ manufacturer: {}", manufacturer.name);
                        }
                        println!("cached manufacturers");
                    }
                    Err(e) => eprint!("failed to load manufacturers: {}", e),
                }
            }));
        }
        // Site cache
        {
            println!("Caching sites...");
            let netbox_client = Arc::clone(&client);
            let sites = Arc::clone(&cache.sites);
            tasks.push(task::spawn(async move {
                match netbox_client.get::<Site>("dcim/sites", None).await {
                    Ok(site_list) => {
                        for site in site_list {
                            sites.insert(site.name.clone(), site.clone());
                            println!("+ site: {}", site.name);
                        }
                        println!("cached sites");
                    }
                    Err(e) => eprint!("failed to load sites: {}", e),
                }
            }));
        }
        // Tag cache
        {
            println!("Caching tags...");
            let netbox_client = Arc::clone(&client);
            let tags = Arc::clone(&cache.tags);
            tasks.push(task::spawn(async move {
                match netbox_client.get::<Tag>("extras/tags", None).await {
                    Ok(tag_list) => {
                        for tag in tag_list {
                            tags.insert(tag.name.clone(), tag.clone());
                            println!("+ tag: {}", tag.name);
                        }
                        println!("cached tags");
                    }
                    Err(e) => eprint!("failed to load tags: {}", e),
                }
            }));
        }

        join_all(tasks).await;

        println!("Finished loading cache!");
        Ok(cache)
    }
}
