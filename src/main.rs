mod cache;
mod config;
mod consolidate;
mod fetch;
mod netbox;
mod utils;

use cache::LocalCache;
use dashmap::DashMap;
use dotenv::dotenv;
use fetch::nagiosxi;
use futures::{
    //future::join_all,
    stream::{self, StreamExt},
    TryFutureExt,
};
use netbox::models::{Device, NetBoxModel};
use std::sync::Arc;
use tokio::{self, sync::Semaphore, time::Instant};

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
    //let semaphore = Arc::new(Semaphore::new(settings.netbox.api_limit.clone()));

    // Build cache
    let cache_future = cache::LocalCache::preload(netbox_client.clone());

    println!("Preloaded Contacts:");

    // Get data
    //let azure_contacts_future = azure_client.fetch_users().map_err(Into::into);
    let azure_devices_future = azure_client.fetch_devices().map_err(Into::into);
    let fortigate_devices_future = fortigate_client.fetch_devices().map_err(Into::into);
    let nagiosxi_hosts_future = nagiosxi_client.get_hosts().map_err(Into::into);
    let nagiosxi_services_future = nagiosxi_client.get_services().map_err(Into::into);

    let (
        local_cache,
        //azure_contacts,
        azure_devices,
        fortigate_devices,
        nagiosxi_hosts,
        nagiosxi_services,
    ) = tokio::try_join!(
        cache_future,
        //azure_contacts_future,
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
        let d = Device::from(dev.clone());
        let key = d.get_cache_key();
        devices
            .entry(key.clone())
            .and_modify(|existing| existing.merge_from_intune(&dev))
            .or_insert(d);
    }
    println!("post intune consolidation list: {}", devices.len());

    for dev in fortigate_devices {
        let mut d = Device::from(dev.clone());
        let key = d.get_cache_key();
        devices
            .entry(key.clone())
            .and_modify(|existing| existing.merge_from_fortigate(&dev))
            .or_insert(d);
    }
    println!("post fortigate consolidation list: {}", devices.len());

    println!("consolidated device list: {}", devices.len());

    // Push data to netbox
    // let mut handles = Vec::new();

    // for intune_user in azure_contacts {
    //     let mut c: Contact = intune_user.into();
    //     let key = c.get_cache_key();
    //     if !local_cache.contacts.contains_key(&key) {
    //         let netbox_client = Arc::clone(&netbox_client);
    //         let permit = semaphore.clone().acquire_owned().await.unwrap();

    //         let handle = task::spawn(async move {
    //             let result = netbox_client
    //                 .post::<Contact, Contact>("tenancy/contacts", &c)
    //                 .await;
    //             println!("Uploaded contact: {}", &c.name);
    //             drop(permit);
    //             result
    //         });
    //         handles.push(handle);
    //     }
    // }

    // Push devices and submodels
    // let mut device_tasks = Vec::new();
    // for (_, device) in devices {
    //     let device_name = device.name.clone();
    //     if device.device_type.is_none() {
    //         eprintln!(
    //             "⚠ skipping `{}`: no device_type after consolidation",
    //             device_name
    //         );
    //         continue;
    //     }
    //     let api = netbox_client.clone();
    //     let cache = local_cache.clone();
    //     let sem = semaphore.clone();

    //     let task = tokio::spawn(async move {
    //         let permit = sem.acquire_owned().await.unwrap();

    //         if let Err(e) = device.push_to_netbox(&api, &cache).await {
    //             eprintln!("❌ push_to_netbox for `{}` failed:", device_name);
    //             // print the whole error chain
    //             for (i, cause) in e.chain().enumerate() {
    //                 if i == 0 {
    //                     eprintln!("   {}", cause);
    //                 } else {
    //                     eprintln!("   └─ caused by: {}", cause);
    //                 }
    //             }
    //         }

    //         drop(permit);
    //     });
    //     device_tasks.push(task);
    // }

    let concurrency = settings.netbox.api_limit;
    stream::iter(devices.into_iter())
        .map(|(key, mut device)| {
            let api = netbox_client.clone();
            let cache = local_cache.clone();
            async move {
                let res = device.push_to_netbox(&api, &cache).await;
                (key, res)
            }
        })
        .buffer_unordered(concurrency)
        .for_each(|(key, res)| async move {
            if let Err(e) = res {
                eprintln!("❌ push_to_netbox for `{}` failed:", key);
                for (i, cause) in e.chain().enumerate() {
                    if i == 0 {
                        eprintln!("   {}", cause);
                    } else {
                        eprintln!("   └─ caused by: {}", cause);
                    }
                }
            }
        })
        .await;

    //join_all(handles).await;
    //join_all(device_tasks).await;

    let timer = start_time.elapsed();
    println!("Time elapsed: {:.2?}", timer);
    Ok(())
}
