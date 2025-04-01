use std::{
    any::{Any, TypeId},
    boxed,
    collections::HashMap,
    sync::Arc,
};

use crate::netbox::models::{self, Device};
use dashmap::{mapref::entry, DashMap};
use tokio::{sync::Semaphore, task::JoinSet};

/*
*
*   CACHE WILL:
*       FETCH DATA FROM NETBOX
*       STORE NETBOX DATA IN A LOCAL CACHE
*       FUNCTIONS:
*           LOOKUPS
*           APPENDING
*       FEATURES:
*           THREAD SAFE
*
*/

pub struct Cache {
    /*
    pub device_cache: DashMap<String, models::Device>,
    pub contact_cache: DashMap<String, models::Contact>,
    pub site_cache: DashMap<String, models::Site>,
    pub device_type_cache: DashMap<String, models::DeviceType>,
    pub device_role_cache: DashMap<String, models::DeviceRole>,
    pub manufacturer_cache: DashMap<String, models::Manufacturer>,
    pub platform_cache: DashMap<String, models::Platform>,
    pub ip_cache: DashMap<String, models::NetBoxIp4>,
    pub vlan_cache: DashMap<String, models::VlanPrefix>,
    */
    pub caches: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Cache {
    pub fn new() -> Self {
        /*
                Self {
                    device_cache: DashMap::new(),
                    contact_cache: DashMap::new(),
                    site_cache: DashMap::new(),
                    device_type_cache: DashMap::new(),
                    device_role_cache: DashMap::new(),
                    manufacturer_cache: DashMap::new(),
                    platform_cache: DashMap::new(),
                    ip_cache: DashMap::new(),
                    vlan_cache: DashMap::new(),
                }
        */
        Self {
            caches: HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, key: String, value: T) {
        let type_id = TypeId::of::<T>();

        let map = self
            .caches
            .entry(type_id)
            .or_insert_with(|| Box::new(DashMap::<String, T>::new()))
            .downcast_mut::<DashMap<String, T>>()
            .expect("Failed to downcast dashmap");

        map.insert(key, value);
    }

    pub fn get<T: 'static + Send + Sync + Clone>(&self, key: String) -> Option<T> {
        let type_id = TypeId::of::<T>();

        self.caches
            .get(&type_id)
            .and_then(|boxed| boxed.downcast_ref::<DashMap<String, T>>())
            .and_then(|map| map.get(&key).map(|entry| entry.value().clone()))
    }

    pub fn update<T, F>(&mut self, key: &str, mutator: F)
    where
        T: 'static + Send + Sync,
        F: FnOnce(&mut T),
    {
        let type_id = TypeId::of::<T>();

        if let Some(boxed) = self.caches.get(&type_id) {
            if let Some(map) = boxed.downcast_ref::<DashMap<String, T>>() {
                if let Some(mut entry) = map.get_mut(key) {
                    mutator(&mut *entry)
                }
            }
        }
    }
}
