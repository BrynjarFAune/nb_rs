// Top: Required Imports
use crate::netbox::{
    api::ApiClient,
    models::{
        Contact, Device, DeviceRole, DeviceType, Manufacturer, NetBoxIp4, NetBoxModel, Platform,
        PostDevice, Site, Tag, VirtualMachine,
    },
};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use std::{any::type_name, sync::Arc};
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
    pub virtual_machines: Arc<DashMap<String, VirtualMachine>>,
    pub ipv4: Arc<DashMap<String, NetBoxIp4>>,
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
            virtual_machines: Arc::new(DashMap::new()),
            ipv4: Arc::new(DashMap::new()),
        }
    }

    pub async fn ensure_cached<T: NetBoxModel>(
        &self,
        item: &mut T,
        api: &ApiClient,
        cache: &Arc<DashMap<String, T>>,
    ) -> Result<()>
    where
        T: NetBoxModel + 'static,
    {
        let slug = item.get_slug();
        let key = item.get_cache_key();
        let typename = type_name::<T>();
        println!(
            "🔍 [Cache] {} lookup slug=`{}`, key=`{}`",
            typename, slug, key
        );

        if typename == "netbox_ingester::netbox::models::Platform" {
            println!("Searching cache for Platform using `{}`", key)
        }
        if let Some(cached) = cache.get(&key) {
            if let Some(id) = cached.get_id() {
                println!("✅ [Cache] HIT  `{}` => id={}", key, id.to_string());
                item.set_id(id);
                return Ok(());
            } else {
                println!("⚠️ [Cache] HIT `{}` but no id set", key);
            }
        } else {
            println!("❌ [Cache] MISS `{}`", key);
        }

        // CREATE in NetBox
        let endpoint = format!("{}/", T::get_endpoint());
        let created: T = api.post::<T, T>(&endpoint, item).await?;
        let id = created.get_id().ok_or_else(|| {
            anyhow!(
                "Created item has no ID ({}): {:?}",
                std::any::type_name::<T>(),
                slug
            )
        })?;
        println!("✨ [Cache] CREATED `{}` => id={}", key, id.to_string());

        item.set_id(id.clone());
        cache.insert(key, created);
        Ok(())
    }

    // All the `ensure_*` methods
    pub async fn ensure_tag(&self, tag: &mut Tag, api: &ApiClient) -> Result<()> {
        self.ensure_cached(tag, api, &self.tags).await
    }
    pub async fn ensure_manufacturer(&self, m: &mut Manufacturer, api: &ApiClient) -> Result<()> {
        self.ensure_cached(m, api, &self.manufacturers).await
    }
    pub async fn ensure_device_type(&self, d: &mut DeviceType, api: &ApiClient) -> Result<()> {
        self.ensure_manufacturer(&mut d.manufacturer, api).await?;
        self.ensure_cached(d, api, &self.device_types).await
    }
    pub async fn ensure_platform(&self, p: &mut Platform, api: &ApiClient) -> Result<()> {
        self.ensure_cached(p, api, &self.platforms).await
    }
    pub async fn ensure_role(&self, r: &mut DeviceRole, api: &ApiClient) -> Result<()> {
        self.ensure_cached(r, api, &self.roles).await
    }
    pub async fn ensure_site(&self, s: &mut Site, api: &ApiClient) -> Result<()> {
        self.ensure_cached(s, api, &self.sites).await
    }

    pub async fn ensure_device_components(
        &self,
        device: &mut Device,
        api: &ApiClient,
    ) -> Result<()> {
        let mut tasks: Vec<BoxFuture<'_, Result<()>>> = Vec::new();

        if let Some(ref mut dt) = device.device_type {
            tasks.push(self.ensure_device_type(dt, api).boxed());
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
        if let Some(ref mut tags) = device.tags {
            for tag in tags.iter_mut() {
                tasks.push(self.ensure_tag(tag, api).boxed());
            }
        }

        for result in join_all(tasks).await {
            result?;
        }

        Ok(())
    }

    pub async fn preload(client: Arc<ApiClient>) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Loading cache...");
        let cache = Self::new();
        let mut tasks = Vec::new();

        macro_rules! preload_model {
            ($idx:expr, $label:literal, $type:ty, $endpoint:expr, $map:expr) => {{
                let client = Arc::clone(&client);
                let map = Arc::clone(&$map);
                let ep = $endpoint;
                tasks.push(tokio::spawn(async move {
                    match client.get::<$type>(ep, None).await {
                        Ok(list) => {
                            for item in list {
                                let key = item.get_cache_key();
                                map.insert(key.clone(), item.clone());
                            }
                            println!("✅ Cached {}s from {}", $label, ep);
                        }
                        Err(e) => {
                            // Top‐level error
                            eprintln!("❌ [Cache:{}] error loading `{}`: {}", $label, ep, e);
                            // Full Debug dump (will include serde_json parse error with line/col)
                            eprintln!("❌ Full error debug:\n{:#?}", e);
                            // If you really want to walk the std::error::Error chain:
                            let mut cause = std::error::Error::source(&e);
                            while let Some(inner) = cause {
                                eprintln!("    └─ caused by: {}", inner);
                                cause = std::error::Error::source(inner);
                            }
                        }
                    }
                }));
            }};
        }

        preload_model!(
            0,
            "device-type",
            DeviceType,
            "dcim/device-types",
            cache.device_types
        );
        preload_model!(
            1,
            "manufacturer",
            Manufacturer,
            "dcim/manufacturers",
            cache.manufacturers
        );
        preload_model!(
            2,
            "device-role",
            DeviceRole,
            "dcim/device-roles",
            cache.roles
        );
        preload_model!(3, "site", Site, "dcim/sites", cache.sites);
        preload_model!(4, "tag", Tag, "extras/tags", cache.tags);
        preload_model!(5, "platform", Platform, "dcim/platforms", cache.platforms);
        preload_model!(6, "contact", Contact, "tenancy/contacts", cache.contacts);
        preload_model!(
            7,
            "virtual-machine",
            VirtualMachine,
            "virtualization/virtual-machines",
            cache.virtual_machines
        );
        preload_model!(8, "device", Device, "dcim/devices/", cache.devices);

        let results = join_all(tasks).await;
        for (i, result) in results.into_iter().enumerate() {
            if result.is_err() {
                eprintln!("Error loading preload task {}: {:?}", i, result.err());
            }
        }

        println!("Cache preload complete.");
        Ok(cache)
    }
}
