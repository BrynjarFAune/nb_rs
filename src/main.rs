mod cache;
mod config;
mod fetch;
mod netbox;
mod parser;
mod utils;

use cache::LocalCache;
use dotenv::dotenv;
use fetch::azure::IntuneUser;
use futures::{future::join_all, TryFutureExt};
use netbox::models::{
    self, Contact, ContactList, Device, DeviceRole, DeviceType, Manufacturer, Site, Status,
};
use std::sync::Arc;
use tokio::{
    self,
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
    let netbox_client = Arc::new(netbox::api::ApiClient::new(&settings.netbox));
    let semaphore = Arc::new(Semaphore::new(settings.netbox.api_limit.clone()));

    let mut handles = Vec::new();

    // Build cache
    let cache_future = cache::LocalCache::preload(netbox_client.clone());

    println!("Preloaded Contacts:");

    // Get data
    let azure_contacts_future = azure_client.fetch_users().map_err(Into::into);
    let azure_devices_future = azure_client.fetch_devices().map_err(Into::into);
    let fortigate_devices_future = fortigate_client.fetch_devices().map_err(Into::into);

    let (local_cache, azure_contacts, azure_devices, fortigate_devices) = tokio::try_join!(
        cache_future,
        azure_contacts_future,
        azure_devices_future,
        fortigate_devices_future
    )?;

    println!("Azure device: {:?}", azure_devices.first());
    println!("FortiGate device: {:?}", fortigate_devices.first());
    println!("Found {} devices via fortigate", fortigate_devices.len());

    // Push data to netbox

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
    join_all(handles).await;

    // End script
    let timer = start_time.elapsed();
    println!("Time elapsed: {:.2?}", timer);

    Ok(())
}
