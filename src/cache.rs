use crate::netbox::{
    api::ApiClient,
    models::{
        Contact, Device, DeviceRole, DeviceType, Manufacturer, NetBoxModel, Platform, Site, Tag,
    },
};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use reqwest::Error as ReqwestError;
use std::sync::Arc;
use std::{collections::HashMap, io};
use tokio::task;

#[derive(Debug, Clone)]
pub struct LocalCache {
    pub devices: Arc<DashMap<String, Device>>,
    pub contacts: Arc<DashMap<String, Contact>>,
    pub manufacturers: Arc<DashMap<String, Manufacturer>>,
    pub device_types: Arc<DashMap<String, DeviceType>>,
    pub roles: Arc<DashMap<String, DeviceRole>>,
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
            device_types: Arc::new(DashMap::new()),
            roles: Arc::new(DashMap::new()),
            sites: Arc::new(DashMap::new()),
            tags: Arc::new(DashMap::new()),
            platforms: Arc::new(DashMap::new()),
        }
    }

    pub async fn ensure_cached<T: NetBoxModel>(
        &self,
        item: &mut T,
        api: &ApiClient,
        cache: &Arc<DashMap<String, T>>,
    ) -> Result<()> {
        // First check cache
        if let Some(cached) = cache.get(&item.get_slug()) {
            if let Some(id) = cached.get_id() {
                println!(
                    "Cache hit for {} with slug: {}",
                    std::any::type_name::<T>(),
                    item.get_slug()
                );
                item.set_id(id);
                return Ok(());
            }
        }

        // If not in cache, try to find in NetBox using a filtered query
        // Append query parameters to filter by slug
        let endpoint = format!("{}/", T::get_endpoint());
        let query = format!("{}?slug={}", endpoint, item.get_slug());
        let existing: Vec<T> = api.get(&query, None).await?;

        if let Some(existing_item) = existing.into_iter().next() {
            if let Some(id) = existing_item.get_id() {
                item.set_id(id.clone());
                cache.insert(item.get_slug(), existing_item);
                return Ok(());
            }
        }

        // If not found, try to create in NetBox
        println!(
            "Creating new {} with slug: {}",
            std::any::type_name::<T>(),
            item.get_slug()
        );
        match api.post::<T, T>(&endpoint, item).await {
            Ok(created) => {
                // Ensure we got an ID back
                let id = created.get_id().ok_or_else(|| {
                    anyhow!(
                        "Created item has no ID ({}): {:?}",
                        std::any::type_name::<T>(),
                        item.get_slug()
                    )
                })?;

                // Update the item and cache
                item.set_id(id.clone());
                cache.insert(item.get_slug(), created);
                Ok(())
            }
            Err(e) => {
                // If creation failed, try one more filtered GET to handle race conditions
                // where another process might have created it
                let existing: Vec<T> = api.get(&query, None).await?;
                if let Some(existing_item) = existing.into_iter().next() {
                    if let Some(id) = existing_item.get_id() {
                        item.set_id(id.clone());
                        cache.insert(item.get_slug(), existing_item);
                        Ok(())
                    } else {
                        Err(e.into()) // Return original error if found item has no ID
                    }
                } else {
                    Err(e.into()) // Return original error if item still not found
                }
            }
        }
    }

    // Helper methods for each type
    pub async fn ensure_tag(&self, tag: &mut Tag, api: &ApiClient) -> Result<()> {
        self.ensure_cached(tag, api, &self.tags).await
    }

    pub async fn ensure_manufacturer(
        &self,
        manufacturer: &mut Manufacturer,
        api: &ApiClient,
    ) -> Result<()> {
        self.ensure_cached(manufacturer, api, &self.manufacturers)
            .await
    }

    pub async fn ensure_device_type(
        &self,
        device_type: &mut DeviceType,
        api: &ApiClient,
    ) -> Result<()> {
        // First ensure manufacturer is cached
        self.ensure_manufacturer(&mut device_type.manufacturer, api)
            .await?;
        // Then cache device type
        self.ensure_cached(device_type, api, &self.device_types)
            .await
    }

    pub async fn ensure_platform(&self, platform: &mut Platform, api: &ApiClient) -> Result<()> {
        self.ensure_cached(platform, api, &self.platforms).await
    }

    pub async fn ensure_role(&self, role: &mut DeviceRole, api: &ApiClient) -> Result<()> {
        self.ensure_cached(role, api, &self.roles).await
    }

    pub async fn ensure_site(&self, site: &mut Site, api: &ApiClient) -> Result<()> {
        self.ensure_cached(site, api, &self.sites).await
    }

    // Method to ensure all device components are cached
    pub async fn ensure_device_components(
        &self,
        device: &mut Device,
        api: &ApiClient,
    ) -> Result<(), ReqwestError> {
        let mut tasks: Vec<BoxFuture<'_, Result<()>>> = Vec::new();

        // Process main components
        if let Some(ref mut device_type) = device.device_type {
            tasks.push(self.ensure_device_type(device_type, api).boxed());
        }
        if let Some(ref mut role) = device.role {
            tasks.push(self.ensure_role(role, api).boxed());
        }
        if let Some(ref mut site) = device.site {
            tasks.push(self.ensure_site(site, api).boxed());
        }
        if let Some(ref mut platform) = device.platform {
            tasks.push(self.ensure_platform(platform, api).boxed());
        }

        // Add tags to parallel processing
        if let Some(ref mut tags) = device.tags {
            for tag in tags.iter_mut() {
                tasks.push(self.ensure_tag(tag, api).boxed());
            }
        }

        // Execute all tasks in parallel
        //        join_all(tasks)
        //            .await
        //            .into_iter()
        //            .collect::<Result<(), _>>()?;
        //
        for result in join_all(tasks).await {
            result;
        }

        Ok(())
    }

    pub async fn preload(client: Arc<ApiClient>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading cache...");
        let cache = Self::new();
        let mut tasks = Vec::new();

        // Device Types
        {
            let netbox_client = Arc::clone(&client);
            let device_types = Arc::clone(&cache.device_types);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<DeviceType>("dcim/device-types", None)
                    .await
                {
                    Ok(device_type_list) => {
                        for device_type in device_type_list {
                            device_types.insert(device_type.slug.clone(), device_type.clone());
                            println!("+ device-type: {}", device_type.model);
                        }
                        println!("Cached device types");
                    }
                    Err(e) => eprintln!("Failed to load device types: {}", e),
                }
            }));
        }

        // Manufacturers
        {
            let netbox_client = Arc::clone(&client);
            let manufacturers = Arc::clone(&cache.manufacturers);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<Manufacturer>("dcim/manufacturers", None)
                    .await
                {
                    Ok(manufacturer_list) => {
                        for manufacturer in manufacturer_list {
                            manufacturers.insert(manufacturer.slug.clone(), manufacturer.clone());
                            println!("+ manufacturer: {}", manufacturer.name);
                        }
                        println!("Cached manufacturers");
                    }
                    Err(e) => eprintln!("Failed to load manufacturers: {}", e),
                }
            }));
        }

        // Device Roles
        {
            let netbox_client = Arc::clone(&client);
            let roles = Arc::clone(&cache.roles);
            tasks.push(task::spawn(async move {
                match netbox_client
                    .get::<DeviceRole>("dcim/device-roles", None)
                    .await
                {
                    Ok(role_list) => {
                        for role in role_list {
                            roles.insert(role.slug.clone(), role.clone());
                            println!("+ device-role: {}", role.name);
                        }
                        println!("Cached device roles");
                    }
                    Err(e) => eprintln!("Failed to load device roles: {}", e),
                }
            }));
        }

        // Sites
        {
            let netbox_client = Arc::clone(&client);
            let sites = Arc::clone(&cache.sites);
            tasks.push(task::spawn(async move {
                match netbox_client.get::<Site>("dcim/sites", None).await {
                    Ok(site_list) => {
                        for site in site_list {
                            sites.insert(site.slug.clone(), site.clone());
                            println!("+ site: {}", site.name);
                        }
                        println!("Cached sites");
                    }
                    Err(e) => eprintln!("Failed to load sites: {}", e),
                }
            }));
        }

        // Tags
        {
            let netbox_client = Arc::clone(&client);
            let tags = Arc::clone(&cache.tags);
            tasks.push(task::spawn(async move {
                match netbox_client.get::<Tag>("extras/tags", None).await {
                    Ok(tag_list) => {
                        for tag in tag_list {
                            tags.insert(tag.slug.clone(), tag.clone());
                            println!("+ tag: {}", tag.name);
                        }
                        println!("Cached tags");
                    }
                    Err(e) => eprintln!("Failed to load tags: {}", e),
                }
            }));
        }

        // Platforms
        {
            let netbox_client = Arc::clone(&client);
            let platforms = Arc::clone(&cache.platforms);
            tasks.push(task::spawn(async move {
                match netbox_client.get::<Platform>("dcim/platforms", None).await {
                    Ok(platform_list) => {
                        for platform in platform_list {
                            platforms.insert(platform.slug.clone(), platform.clone());
                            println!("+ platform: {}", platform.name);
                        }
                        println!("Cached platforms");
                    }
                    Err(e) => eprintln!("Failed to load platforms: {}", e),
                }
            }));
        }

        // Wait for all tasks and collect results
        let results = join_all(tasks).await;
        let mut had_errors = false;

        // Process each task result
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(_) => continue,
                Err(e) => {
                    had_errors = true;
                    let cache_type = match i {
                        0 => "device types",
                        1 => "manufacturers",
                        2 => "device roles",
                        3 => "sites",
                        4 => "tags",
                        5 => "platforms",
                        _ => "unknown cache type",
                    };
                    eprintln!("Error loading {}: {}", cache_type, e);
                }
            }
        }

        if had_errors {
            eprintln!("Cache preload completed with some errors (see above)");
        } else {
            println!("Cache loaded successfully with no errors!");
        }

        Ok(cache)
    }
}
