mod cache;
mod config;
mod consolidate;
mod fetch;
mod netbox;
mod utils;

use cache::LocalCache;
use dashmap::DashMap;
use dotenv::dotenv;
use fetch::{azure::IntuneUser, nagiosxi};
use futures::{future::join_all, TryFutureExt};
use netbox::models::{
    self, Contact, ContactList, Device, DeviceRole, DeviceType, Manufacturer, PostDevice, Site,
    Status, VirtualMachine,
};
use std::sync::Arc;
use tokio::{
    self,
    runtime::Handle,
    sync::{Mutex, RwLock, Semaphore},
    task,
    time::Instant,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //
    // Prepare environment
    let start_time = Instant::now();
    dotenv().ok();
    let settings = config::load()?;
    let azure_client = Arc::new(fetch::azure::AzureClient::new(&settings.azure).await?);
    let fortigate_client =
        Arc::new(fetch::fortigate::FortiGateClient::new(&settings.fortigate).await?);
    let nagiosxi_client = Arc::new(nagiosxi::NagiosxiClient::new(&settings.nagiosxi));
    let netbox_client = Arc::new(netbox::api::ApiClient::new(&settings.netbox));
    let semaphore = Arc::new(Semaphore::new(settings.netbox.api_limit.clone()));

    // Build cache
    let cache_future = cache::LocalCache::preload(netbox_client.clone());

    println!("Preloaded Contacts:");

    // Get data
    let azure_contacts_future = azure_client.fetch_users().map_err(Into::into);
    let azure_devices_future = azure_client.fetch_devices().map_err(Into::into);
    let fortigate_devices_future = fortigate_client.fetch_devices().map_err(Into::into);
    let nagiosxi_hosts_future = nagiosxi_client.get_hosts().map_err(Into::into);
    let nagiosxi_services_future = nagiosxi_client.get_services().map_err(Into::into);

    let (
        local_cache,
        azure_contacts,
        azure_devices,
        fortigate_devices,
        nagiosxi_hosts,
        nagiosxi_services,
    ) = tokio::try_join!(
        cache_future,
        azure_contacts_future,
        azure_devices_future,
        fortigate_devices_future,
        nagiosxi_hosts_future,
        nagiosxi_services_future
    )?;

    println!("Found {} devices from fortigate", &fortigate_devices.len());
    println!("Found {} devices via azure", &azure_devices.len());
    println!("Found {} NagiosXI hosts", &nagiosxi_hosts.recordcount);
    println!("Found {} NagiosXI services", &nagiosxi_services.recordcount);
    println!(
        "First NagiosXI service: {:?}",
        &nagiosxi_services.servicestatus.first()
    );

    // consolidate data

    let devices = DashMap::<String, Device>::new();

    for dev in azure_devices {
        let key = dev.name.to_lowercase();
        devices
            .entry(key.clone())
            .and_modify(|existing| existing.merge_from_intune(&dev))
            .or_insert_with(|| Device::from(dev));
    }
    println!("post intune consolidation list: {}", devices.len());

    for dev in fortigate_devices {
        let key = dev
            .hostname
            .as_ref()
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| dev.mac.clone());
        devices
            .entry(key.clone())
            .and_modify(|existing| existing.merge_from_fortigate(&dev))
            .or_insert_with(|| Device::from(dev));
    }
    println!("post fortigate consolidation list: {}", devices.len());

    println!("consolidated device list: {}", devices.len());

    let postable_devices = Arc::new(
        devices
            .into_iter()
            .map(|(_, device)| PostDevice::from(device))
            .collect::<Vec<_>>(),
    );

    // Push data to netbox
    let mut handles = Vec::new();

    for contact in azure_contacts {
        if !local_cache.contacts.contains_key(&contact.name) {
            let netbox_client = Arc::clone(&netbox_client);
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let handle = task::spawn(async move {
                let tmp_contact: Contact = contact.into();
                let result = netbox_client
                    .post::<Contact, Contact>("tenancy/contacts", &tmp_contact)
                    .await;
                println!("++ Contact: {}", &tmp_contact.name);
                drop(permit);
                result
            });
            handles.push(handle);
        }
    }

    for device in postable_devices.iter() {
        let dev = device.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = netbox_client.clone();

        //let handle = task::spawn(async move {
        // let result = client.post("dcim/devices", &dev).await;
        // drop(permit);
        // result
        //});
        //handles.push(handle);

        println!("~ {} | skipping for now", &dev.name);
    }

    join_all(handles).await;

    // End script
    let timer = start_time.elapsed();
    println!("Time elapsed: {:.2?}", timer);

    Ok(())
}
